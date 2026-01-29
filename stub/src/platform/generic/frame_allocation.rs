//! Implementation of the frame allocation API provided to the remainder of `revm-stub`.

use core::{mem, ptr::NonNull};

use sync::Spinlock;

use crate::platform::{
    allocate, deallocate,
    generic::{AllocationPolicy, OutOfMemory, platform},
};

/// The head of the linked list of [`AllocationRecord`]s.
static HEAD: Spinlock<Option<SendPtr<AllocationRecord>>> = Spinlock::new(None);

/// Returns the size, in bytes, of a frame.
pub fn frame_size() -> u64 {
    platform().frame_size()
}

/// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`].
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
/// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
pub fn allocate_frames(
    count: u64,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, OutOfMemory> {
    let frame_region_start = platform().allocate_frames(count, policy)?;
    if insert_region(frame_region_start, count).is_err() {
        // SAFETY:
        //
        // The frame region has not been exposed outside of this module and has not been stored.
        unsafe { platform().deallocate_frames(frame_region_start, count) }

        crate::trace!("allocate_frames({count}, {policy:?}) -> OutOfMemory");
        return Err(OutOfMemory);
    };

    crate::trace!("allocate_frames({count}, {policy:?}) -> {frame_region_start:#x}");
    Ok(FrameAllocation {
        physical_address: frame_region_start,
        count,
    })
}

/// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`]. The
/// starting physical address of the region will be a multiple of `alignment`.
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
/// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
pub fn allocate_frames_aligned(
    count: u64,
    alignment: u64,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, OutOfMemory> {
    let frame_region_start = platform().allocate_frames_aligned(count, alignment, policy)?;
    if insert_region(frame_region_start, count).is_err() {
        // SAFETY:
        //
        // The frame region has not been exposed outside of this module and has not been stored.
        unsafe { platform().deallocate_frames(frame_region_start, count) }

        crate::trace!("allocate_frames_aligned({count}, {policy:?}) -> OutOfMemory");
        return Err(OutOfMemory);
    };

    crate::trace!("allocate_frames_aligned({count}, {policy:?}) -> {frame_region_start:#x}");
    Ok(FrameAllocation {
        physical_address: frame_region_start,
        count,
    })
}

/// Deallocates a region of `count` frames with the starting `physical_address`.
///
/// # Safety
///
/// These frames must have been allocated by a call to [`allocate_frames()`] or
/// [`allocate_frames_aligned()`] and must not be referenced after the start of this function.
#[expect(
    clippy::missing_panics_doc,
    reason = "panic only if safety invariants are breached"
)]
pub unsafe fn deallocate_frames(physical_address: u64, count: u64) {
    crate::trace!("deallocate_frames({physical_address:#x}, {count})");
    if count == 0 {
        // Zero-sized deallocations require no work.
        return;
    }

    // SAFETY:
    //
    // The frame region will not be accessed after this.
    unsafe { remove_region(physical_address, count) }

    // SAFETY:
    //
    // The frame region will not be accessed after this.
    unsafe { platform().deallocate_frames(physical_address, count) }
}

/// Deallocates all outstanding frame allocations.
///
/// # Safety
///
/// All frame regions allocated via this module must not be accessed after the start of this
/// function.
pub unsafe fn deallocate_all_frames() {
    let frame_size = platform().frame_size();
    let mut head = HEAD.lock();

    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    while let Some(node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and the list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_ref() };

        let size = record.end - record.start;
        let count = size / frame_size;

        // SAFETY:
        //
        // This region was allocated by this module and will not be accessed
        // again after this call.
        unsafe {
            platform().deallocate_frames(record.start, count);
        }

        let next = record.next;

        // SAFETY:
        //
        // This allocation record is no longer referenced.
        unsafe {
            deallocate(
                node.cast::<u8>(),
                mem::size_of::<AllocationRecord>(),
                mem::align_of::<AllocationRecord>(),
            );
        }

        current = next;
    }

    // Clear list and release lock
    *head = None;
}

/// Removes a region of physical memory from the linked list.
pub unsafe fn remove_region(start: u64, count: u64) {
    if count == 0 {
        // Zero-sized deallocations require no work.
        return;
    }

    let frame_size = platform().frame_size();

    let free_start = start;
    let Some(free_end) = count
        .checked_mul(platform().frame_size())
        .and_then(|size| free_start.checked_add(size))
    else {
        crate::warn!("deallocate_frames() failed");
        return;
    };

    let mut head = HEAD.lock();
    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    let mut previous: Option<NonNull<AllocationRecord>> = None;
    while let Some(mut node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_mut() };

        // Skip record if its end is less than the target region's start.
        if record.end < free_start {
            previous = current;
            current = record.next;
            continue;
        }

        // Exit the loop if the current record's start is greater than or equal to the target
        // region's end.
        if record.start >= free_end {
            break;
        }

        // We have that:
        //
        // record.start < free_end
        // record.end >= free_start
        //
        // This implies that the two regions intersect, so we now need to check that the
        // overlapping region is precisely the target deallocation region, since the algorithm
        // guarantees that all mergable regions have been merged.
        let overlap_start = record.start.max(free_start);
        let overlap_end = record.end.min(free_end);
        let overlap_size = overlap_end - overlap_start;
        let overlap_count = overlap_size / frame_size;

        if overlap_start != free_start || overlap_count != count {
            // Drop `head` to enable deallocation in panic handler.
            drop(head);
            panic!("free region not valid");
        }

        match (overlap_start > record.start, overlap_end > record.end) {
            (true, true) => {
                // Allocated regions remain at the start and end.
                record.end = overlap_start;

                let Some(allocation_record_allocation) = allocate(
                    mem::size_of::<AllocationRecord>(),
                    mem::align_of::<AllocationRecord>(),
                ) else {
                    // If the allocation fails, we leak the top region of frames to prevent
                    // unsafety.
                    return;
                };
                let allocation_record_ptr = allocation_record_allocation
                    .ptr_nonnull()
                    .cast::<AllocationRecord>();

                record.next = Some(allocation_record_ptr);
                // SAFETY:
                //
                // This memory region was just allocated and thus this function has exclusive
                // access to it.
                unsafe {
                    allocation_record_ptr.write(AllocationRecord {
                        start: overlap_end,
                        end: record.end,
                        next: record.next,
                    })
                }

                // Forget this [`Allocation`] to prevent early free.
                mem::forget(allocation_record_allocation);
                return;
            }
            (true, false) => {
                // Allocated region remains at the start.
                record.end = overlap_start;
                return;
            }
            (false, true) => {
                // Allocated region remains at the end.
                record.start = overlap_end;
                return;
            }
            (false, false) => {
                // No allocated regions remain.
                let next = record.next;
                match previous {
                    // SAFETY:
                    //
                    // All `Some` values point to a valid [`AllocationRecord`] and this linked list
                    // is protected by the [`HEAD`] lock.
                    Some(mut previous) => unsafe { previous.as_mut().next = next },
                    None => *head = next.map(SendPtr),
                }

                // SAFETY:
                //
                // This node is not in use any longer.
                unsafe {
                    deallocate(
                        node.cast::<u8>(),
                        mem::size_of::<AllocationRecord>(),
                        mem::align_of::<AllocationRecord>(),
                    )
                }
                return;
            }
        }
    }
}

/// Inserts a region of physical memory into the linked list.
fn insert_region(start: u64, count: u64) -> Result<(), OutOfMemory> {
    let mut head = HEAD.lock();

    let mut current_start = start;
    let mut current_end = count
        .checked_mul(platform().frame_size())
        .and_then(|size| start.checked_add(size))
        .ok_or(OutOfMemory)?;

    // Ordering prevents having to handle deallocating `node` if `current_end` calculation fails.
    let allocation_record_allocation = allocate(
        mem::size_of::<AllocationRecord>(),
        mem::align_of::<AllocationRecord>(),
    )
    .ok_or(OutOfMemory)?;
    let allocation_record_ptr = allocation_record_allocation
        .ptr_nonnull()
        .cast::<AllocationRecord>();

    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    let mut previous: Option<NonNull<AllocationRecord>> = None;
    while let Some(node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_ref() };

        if record.end < current_start {
            previous = current;
            current = record.next;
            continue;
        }

        if record.start > current_end {
            break;
        }

        current_start = current_start.min(record.start);
        current_end = current_end.max(record.end);

        let next = record.next;
        match previous {
            // SAFETY:
            //
            // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
            // protected by the [`HEAD`] lock.
            Some(mut previous) => unsafe { previous.as_mut().next = next },
            None => *head = next.map(SendPtr),
        }

        // SAFETY:
        //
        // We have removed all references to this memory block and it will not be used after this.
        unsafe {
            deallocate(
                node.cast::<u8>(),
                mem::size_of::<AllocationRecord>(),
                mem::align_of::<AllocationRecord>(),
            )
        }

        current = next;
    }

    // SAFETY:
    //
    // The region of memory pointed to by `allocation_record_ptr` was just allocated and thus is
    // under the exclusive control of this module.
    unsafe {
        allocation_record_ptr.write(AllocationRecord {
            start: current_start,
            end: current_end,
            next: current,
        })
    }

    match previous {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
        // protected by the [`HEAD`] lock.
        Some(mut previous) => unsafe { previous.as_mut().next = Some(allocation_record_ptr) },
        None => *head = Some(SendPtr(allocation_record_ptr)),
    }

    // Forget this [`Allocation`] to prevent early free.
    mem::forget(allocation_record_allocation);
    Ok(())
}

/// Wrapper around a region of frames allocated with [`allocate_frames()`] or
/// [`allocate_frames_aligned()`].
///
/// This structure automatically frees the region of frames when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameAllocation {
    /// The physical address of the start of the allocated frame region.
    physical_address: u64,
    /// The number of frames in the allocated frame region.
    count: u64,
}

impl FrameAllocation {
    /// Returns the physical address of the start of the allocated frame region.
    pub const fn physical_address(&self) -> u64 {
        self.physical_address
    }

    /// Returns the number of frames in the allocated frame region.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Returns the total number of bytes controlled by the allocated frame region.
    pub fn total_size(&self) -> u64 {
        self.count().strict_mul(frame_size())
    }
}

impl Drop for FrameAllocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of frames indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.physical_address, self.count) }
    }
}

/// The record of an allocation.
struct AllocationRecord {
    /// The physical address at the start of the allocation.
    start: u64,
    /// The physical address at the end of the allocation.
    end: u64,
    /// The next [`AllocationRecord`] in the linked list.
    next: Option<NonNull<AllocationRecord>>,
}

// SAFETY:
//
// [`AllocationRecord`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl Send for AllocationRecord {}

// SAFETY:
//
// [`AllocationRecord`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl Sync for AllocationRecord {}
/// Sync wrapper.
#[repr(transparent)]
struct SendPtr<T>(NonNull<T>);

// SAFETY:
//
// [`SendPtr<T>`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl<T> Send for SendPtr<T> {}
// SAFETY:
//
// [`SendPtr<T>`]s are safe to read from other threads, since they don't expose anything that
// requires a single thread.
unsafe impl<T> Sync for SendPtr<T> {}

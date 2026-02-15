//! Implementation of the frame allocation API provided to the remainder of `revm-stub`.

use core::{mem, ptr::NonNull};

use sync::Spinlock;

use crate::platform::{
    allocate, deallocate,
    generic::{AllocationPolicy, OutOfMemory, platform},
    memory_structs::FrameRange,
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
    let range = platform().allocate_frames(count, policy)?;
    if insert_range(range).is_err() {
        // SAFETY:
        //
        // The frame range has not been exposed outside of this module and has not been stored.
        unsafe { platform().deallocate_frames(range) }

        crate::trace!("allocate_frames({count}, {policy:?}) -> OutOfMemory");
        return Err(OutOfMemory);
    };

    crate::trace!("allocate_frames({count}, {policy:?}) -> {range:x?}");
    Ok(FrameAllocation(range))
}

/// Allocates a range of `count` frames in accordance with the provided [`AllocationPolicy`]. The
/// starting physical address of the range will be a multiple of `alignment`.
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
    let range = platform().allocate_frames_aligned(count, alignment, policy)?;
    if insert_range(range).is_err() {
        // SAFETY:
        //
        // The frame range has not been exposed outside of this module and has not been stored.
        unsafe { platform().deallocate_frames(range) }

        crate::trace!("allocate_frames_aligned({count}, {alignment}, {policy:?}) -> OutOfMemory");
        return Err(OutOfMemory);
    };

    crate::trace!("allocate_frames_aligned({count}, {alignment}, {policy:?}) -> {range:x?}");
    Ok(FrameAllocation(range))
}

/// Deallocates the provided [`FrameRange`].
///
/// # Safety
///
/// The [`Frame`][f]s contained within the provided [`FrameRange`] must have been allocated by a
/// call to [`allocate_frames()`] or [`allocate_frames_aligned()`] and must not be utilized after
/// this function.
///
/// [f]: crate::platform::memory_structs::Frame
pub unsafe fn deallocate_frames(range: FrameRange) {
    crate::trace!("deallocate_frames({range:x?})");
    // SAFETY:
    //
    // The [`FrameRange`] will not be accessed after this.
    unsafe { remove_range(range) }

    // SAFETY:
    //
    // The frame region will not be accessed after this.
    unsafe { platform().deallocate_frames(range) }
}

/// Deallocates all outstanding frame allocations.
///
/// # Safety
///
/// All [`FrameRange`]s allocated via this module must not be accessed after the start of this
/// function.
pub unsafe fn deallocate_all_frames() {
    let mut head = HEAD.lock();

    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    while let Some(node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and the list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_ref() };

        // SAFETY:
        //
        // This region was allocated by this module and will not be accessed
        // again after this call.
        unsafe {
            platform().deallocate_frames(record.range);
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
#[expect(clippy::missing_safety_doc)]
#[expect(clippy::missing_panics_doc)]
pub unsafe fn remove_range(range: FrameRange) {
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
        if record.range.end() < range.start() {
            previous = current;
            current = record.next;
            continue;
        }

        // Exit the loop if the current record's start is greater than or equal to the target
        // region's end.
        if record.range.start() >= range.end() {
            break;
        }

        // We have that:
        //
        // record.start < free_end
        // record.end >= free_start
        //
        // This implies that `range` is fully subsumed inside of `record.range`. If that is not the
        // case, then the requested [`FrameRange`] is not valid for deallocation.
        let (lower, overlaps, upper) = record.range.partition_with(range);
        if overlaps != range {
            // Drop `head` to enable deallocation in panic handler.
            drop(head);

            panic!("invalid frame range deallocated");
        }

        match (!lower.is_empty(), !upper.is_empty()) {
            (true, true) => {
                // Allocated regions remain at the start and end.
                record.range = lower;

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
                        range: upper,
                        next: record.next,
                    })
                }

                // Forget this [`FrameAllocation`] to prevent early free.
                mem::forget(allocation_record_allocation);
                return;
            }
            (true, false) => {
                // Allocated region remains at the start.
                record.range = lower;
                return;
            }
            (false, true) => {
                // Allocated region remains at the end.
                record.range = upper;
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
fn insert_range(mut range: FrameRange) -> Result<(), OutOfMemory> {
    let mut head = HEAD.lock();

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

        if record.range.end() < range.start() {
            previous = current;
            current = record.next;
            continue;
        }

        if record.range.start() > range.end() {
            break;
        }

        range = range
            .merge(record.range)
            .expect("frame range checks failed");

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
            range,
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

    // Forget this [`FrameAllocation`] to prevent early free.
    mem::forget(allocation_record_allocation);
    Ok(())
}

/// Wrapper around a region of frames allocated with [`allocate_frames()`] or
/// [`allocate_frames_aligned()`].
///
/// This structure automatically frees the region of frames when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameAllocation(FrameRange);

impl FrameAllocation {
    /// Returns the [`FrameRange`] that this [`FrameAllocation`] owns.
    pub const fn range(&self) -> FrameRange {
        self.0
    }
}

impl Drop for FrameAllocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of frames indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.0) }
    }
}

/// A record of an allocated [`FrameRange`].
///
/// This may have been merged with adjacent [`FrameRange`]s.
struct AllocationRecord {
    /// The
    range: FrameRange,
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

//! Implementation of generic parts of cross address space switching.

use core::{
    mem::{self, MaybeUninit},
    ptr::NonNull,
    slice,
};

use stub_api::{AllocationFlags, MapFlags, MemoryDescriptor, MemoryType, Status, TakeoverFlags};
use sync::Spinlock;

use crate::{
    arch::{
        AddressSpaceImpl,
        generic::address_space::{AddressSpace, MapError, ProtectionFlags},
    },
    platform::{
        AllocationPolicy, BufferTooSmall, allocate, allocate_frames_aligned, deallocate,
        deallocate_frames, frame_size, memory_map, read_bytes_at, read_u64_at, remove_region,
        takeover, write_u64_at,
    },
    util::{u64_to_usize, usize_to_u64},
    warn,
};

/// The `func_id` of the `write` function.
pub const WRITE_ID: u16 = 0;
/// The `func_id` of the `allocate_frames` function.
pub const ALLOCATE_FRAMES_ID: u16 = 1;
/// The `func_id` of the `deallocate_frames` function.
pub const DEALLOCATE_FRAMES_ID: u16 = 2;
/// The `func_id` of the `get_memory_map` function.
pub const GET_MEMORY_MAP_ID: u16 = 3;
/// The `func_id` of the `map` function.
pub const MAP_ID: u16 = 4;
/// The `func_id` of the `unmap` function.
pub const UNMAP_ID: u16 = 5;
/// The `func_id` of the `takeover` function.
pub const TAKEOVER_ID: u16 = 6;
/// The maximum generic architecture independent function ID.
pub const MAX_GENERIC_ID: u16 = 6;

/// The saved memory map.
static MEMORY_MAP: Spinlock<MemoryMapWrapper> = Spinlock::new(MemoryMapWrapper::new());
/// The frames allocated by the application.
static FRAME_ALLOCATIONS: Spinlock<FrameVec> = Spinlock::new(FrameVec::new());

/// Handles generic cross address space calls.
#[expect(clippy::too_many_arguments)]
pub fn handle_call(
    address_space: &mut AddressSpaceImpl,
    func_id: u16,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
    arg_5: u64,
    _arg_6: u64,
) -> Status {
    match func_id {
        WRITE_ID => {
            let mut string_ptr = arg_1;
            let mut string_len = arg_2;
            if string_ptr == 0 {
                return Status::INVALID_USAGE;
            }

            let mut buffer = [0; 4096];
            while string_len != 0 {
                let Ok(physical_address) = address_space.translate_virt(string_ptr) else {
                    return Status::INVALID_USAGE;
                };
                let offset = string_ptr % address_space.page_size();
                let max_buffer_transfer_size =
                    u64_to_usize(string_len.min(usize_to_u64(buffer.len())));
                let max_page_transfer_size = u64_to_usize(address_space.page_size() - offset);
                let transfer_size = max_buffer_transfer_size.min(max_page_transfer_size);

                read_bytes_at(physical_address, &mut buffer[..transfer_size]);
                let Ok(str) = str::from_utf8(&buffer[..transfer_size]) else {
                    return Status::INVALID_USAGE;
                };

                crate::print!("{str}");
                string_ptr = string_ptr.wrapping_add(usize_to_u64(transfer_size));
                string_len = string_len.wrapping_sub(usize_to_u64(transfer_size));
            }

            Status::SUCCESS
        }
        ALLOCATE_FRAMES_ID => {
            let count = arg_1;
            let alignment = arg_2;
            let flags = arg_3;
            let physical_address_ptr = arg_4;
            if count == 0
                || physical_address_ptr == 0
                || !physical_address_ptr.is_multiple_of(4)
                || flags & AllocationFlags::VALID.0 != flags
            {
                return Status::INVALID_USAGE;
            }

            let Ok(physical_address_ptr_address) =
                address_space.translate_virt(physical_address_ptr)
            else {
                return Status::INVALID_USAGE;
            };
            let policy = match AllocationFlags(flags & 0b11) {
                AllocationFlags::ANY => AllocationPolicy::Any,
                AllocationFlags::BELOW => {
                    AllocationPolicy::Below(read_u64_at(physical_address_ptr_address))
                }
                _ => return Status::INVALID_USAGE,
            };
            let Ok(frame_allocation) = allocate_frames_aligned(count, alignment, policy) else {
                return Status::OUT_OF_MEMORY;
            };

            write_u64_at(
                physical_address_ptr_address,
                frame_allocation.physical_address(),
            );

            FRAME_ALLOCATIONS.lock().add_region(
                frame_allocation.physical_address(),
                frame_allocation.count(),
            );

            // Forget the frame allocation to prevent it from being freed early.
            mem::forget(frame_allocation);
            Status::SUCCESS
        }
        DEALLOCATE_FRAMES_ID => {
            let physical_address = arg_1;
            let count = arg_2;

            let mut frame_allocation = FRAME_ALLOCATIONS.lock();
            if !frame_allocation.contains_region(physical_address, count) {
                warn!("tried to deallocate unallocated frame region");
                return Status::INVALID_USAGE;
            }

            frame_allocation.remove_region(physical_address, count);

            // SAFETY:
            //
            // Application must have previously allocated said frames.
            unsafe { deallocate_frames(physical_address, count) }

            // Ensure that the [`FRAME_ALLOCATIONS`] lock is held until the frames are successfully
            // deallocated.
            drop(frame_allocation);
            Status::SUCCESS
        }
        GET_MEMORY_MAP_ID => {
            let size_ptr = arg_1;
            let map_buffer_ptr = arg_2;
            let key_ptr = arg_3;
            let descriptor_size_ptr = arg_4;
            let descriptor_version_ptr = arg_5;
            crate::trace!(
                "allocate_frames(\
                {size_ptr}, \
                {map_buffer_ptr:#x}, \
                {key_ptr:#x}, \
                {descriptor_size_ptr:#x}, \
                {descriptor_version_ptr:#x}\
            )"
            );

            // Validate that `size_ptr` is not zero and is properly aligned.
            if size_ptr == 0 || !size_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>())) {
                return Status::INVALID_USAGE;
            }

            let mut map = MEMORY_MAP.lock();
            map.update();

            let Err(BufferTooSmall { required_count }) = memory_map(&mut []) else {
                // If the memory map is empty, then the memory map is clearly not implemented
                // correctly.
                warn!("empty platform memory map is unusual");
                return Status::INVALID_USAGE;
            };

            let Ok(size_physical_address) = address_space.translate_virt(size_ptr) else {
                return Status::INVALID_USAGE;
            };

            // Validate that the pointers are properly aligned and are not NULL.
            if map_buffer_ptr == 0
                || !map_buffer_ptr.is_multiple_of(usize_to_u64(mem::align_of::<MemoryDescriptor>()))
                || key_ptr == 0
                || !key_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
                || descriptor_size_ptr == 0
                || !descriptor_size_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
                || descriptor_version_ptr == 0
                || !descriptor_version_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
            {
                write_u64_at(size_physical_address, usize_to_u64(required_count));
                return Status::INVALID_USAGE;
            }

            let Ok(key_physical_address) = address_space.translate_virt(key_ptr) else {
                write_u64_at(size_physical_address, usize_to_u64(required_count));
                return Status::INVALID_USAGE;
            };

            let Ok(descriptor_size_physical_address) =
                address_space.translate_virt(descriptor_size_ptr)
            else {
                write_u64_at(size_physical_address, usize_to_u64(required_count));
                return Status::INVALID_USAGE;
            };

            let Ok(descriptor_version_physical_address) =
                address_space.translate_virt(descriptor_version_ptr)
            else {
                write_u64_at(size_physical_address, usize_to_u64(required_count));
                return Status::INVALID_USAGE;
            };

            for (index, descriptor) in map.descriptors().iter().enumerate() {
                let descriptor_ptr = map_buffer_ptr.strict_add(usize_to_u64(
                    index.strict_mul(mem::size_of::<MemoryDescriptor>()),
                ));
                let start_ptr = descriptor_ptr
                    .strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, start)));
                let count_ptr = descriptor_ptr
                    .strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, count)));
                let region_type_ptr = descriptor_ptr
                    .strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, region_type)));

                let Ok(start_physical_address) = address_space.translate_virt(start_ptr) else {
                    return Status::INVALID_USAGE;
                };
                let Ok(count_physical_address) = address_space.translate_virt(count_ptr) else {
                    return Status::INVALID_USAGE;
                };
                let Ok(region_type_physical_address) =
                    address_space.translate_virt(region_type_ptr)
                else {
                    return Status::INVALID_USAGE;
                };

                write_u64_at(start_physical_address, descriptor.start);
                write_u64_at(count_physical_address, descriptor.count);
                write_u64_at(
                    region_type_physical_address,
                    u64::from(descriptor.region_type.0),
                );
            }

            write_u64_at(
                size_physical_address,
                usize_to_u64(map.size * mem::size_of::<MemoryDescriptor>()),
            );
            write_u64_at(key_physical_address, map.key);
            write_u64_at(
                descriptor_size_physical_address,
                usize_to_u64(mem::size_of::<MemoryDescriptor>()),
            );
            write_u64_at(
                descriptor_version_physical_address,
                MemoryDescriptor::VERSION,
            );

            Status::SUCCESS
        }
        MAP_ID => {
            let physical_address = arg_1;
            let virtual_address = arg_2;
            let count = arg_3;
            let flags = arg_4;

            if flags & MapFlags::VALID.0 != flags {
                return Status::INVALID_USAGE;
            }
            let mut protection = ProtectionFlags::READ;
            if flags & MapFlags::WRITE.0 == MapFlags::WRITE.0 {
                protection |= ProtectionFlags::WRITE
            }
            if flags & MapFlags::EXECUTE.0 == MapFlags::EXECUTE.0 {
                protection |= ProtectionFlags::EXECUTE;
            }

            let Some(total_size) = count.checked_mul(page_frame_size(address_space)) else {
                return Status::OUT_OF_MEMORY;
            };
            let result = address_space.map(
                virtual_address,
                physical_address,
                total_size.div_ceil(address_space.page_size()),
                protection,
            );
            match result {
                Ok(()) => Status::SUCCESS,
                Err(MapError::AllocationError) => Status::OUT_OF_MEMORY,
                Err(_) => Status::INVALID_USAGE,
            }
        }
        UNMAP_ID => {
            let virtual_address = arg_1;
            let count = arg_2;

            let Some(total_size) = count.checked_mul(page_frame_size(address_space)) else {
                return Status::OUT_OF_MEMORY;
            };

            // SAFETY:
            //
            // The application requested that said page was unmapped.
            unsafe {
                address_space.unmap(
                    virtual_address,
                    total_size.div_ceil(address_space.page_size()),
                )
            }

            Status::SUCCESS
        }
        TAKEOVER_ID => {
            let key = arg_1;
            let flags = TakeoverFlags(arg_2);
            if (flags.0 & TakeoverFlags::VALID.0) != flags.0 {
                return Status::INVALID_USAGE;
            }

            // Remove frame regions allocated by the executable to prevent them from being freed
            // when this application exits.
            for frame_region in FRAME_ALLOCATIONS.lock().descriptors() {
                unsafe { remove_region(frame_region.0, frame_region.1) }
            }

            takeover(key, flags)
        }
        _ => Status::NOT_SUPPORTED,
    }
}

/// Returns the larger of [`AddressSpaceImpl::page_size()`] and [`frame_size()`].
fn page_frame_size(address_space: &mut AddressSpaceImpl) -> u64 {
    address_space.page_size().max(frame_size())
}

/// Wrapper around simple updates of the memory map.
struct MemoryMapWrapper {
    /// Pointer to the start of the platform memory map as of the last update.
    ptr: Option<NonNull<MemoryDescriptor>>,
    /// The capacity, in [`MemoryDescriptor`]s, of the buffer.
    capacity: usize,
    /// The number of [`MemoryDescriptor`]s stored in the buffer.
    size: usize,
    /// A unique key for the current memory map.
    key: u64,
}

impl MemoryMapWrapper {
    /// Returns an empty [`MemoryMapWrapper`].
    pub const fn new() -> Self {
        Self {
            ptr: None,
            capacity: 0,
            size: 0,
            key: 0,
        }
    }

    /// Refreshes the active memory map.
    #[expect(clippy::missing_panics_doc)]
    pub fn update(&mut self) {
        loop {
            match memory_map(self.buffer_mut()) {
                Ok(map) => {
                    let size = map.descriptors().len();
                    let key = map.key();

                    // Work-around for borrowing issues.
                    self.size = size;
                    self.key = key;
                    return;
                }
                Err(BufferTooSmall { required_count }) => {
                    if let Some(active_ptr) = self.ptr.take() {
                        // If we've allocated a buffer, free it.

                        // SAFETY:
                        //
                        // The region of memory demarcated by `active_ptr` is no longer in use.
                        unsafe {
                            deallocate(
                                active_ptr.cast::<u8>(),
                                self.capacity.strict_mul(mem::size_of::<MemoryDescriptor>()),
                                mem::align_of::<MemoryDescriptor>(),
                            )
                        }
                    }

                    // Add additional entries to account for memory allocation.
                    let new_count = required_count.strict_add(4);
                    let total_size = new_count.strict_mul(mem::size_of::<MemoryDescriptor>());

                    self.capacity = new_count;
                    self.size = 0;
                    self.ptr = allocate(total_size, mem::align_of::<MemoryDescriptor>()).map(
                        |allocation| {
                            let ptr = allocation.ptr_nonnull().cast::<MemoryDescriptor>();

                            // Forget [`Allocation`] to prevent early drop.
                            mem::forget(allocation);
                            ptr
                        },
                    );
                    if self.ptr.is_none() {
                        panic!("allocation error while updating platform memory map")
                    }
                }
            }
        }
    }

    /// Returns an immutable slice of active [`MemoryDescriptor`]s.
    pub fn descriptors(&self) -> &[MemoryDescriptor] {
        if let Some(ptr) = self.ptr {
            // SAFETY:
            //
            // The region of memory described by `ptr` is controlled by `self` and is initialized.
            unsafe { slice::from_raw_parts(ptr.as_ptr(), self.size) }
        } else {
            &[]
        }
    }

    /// Returns the whole [`MemoryDescriptor`] buffer in an initialized state.
    ///
    /// Any currently active [`MemoryDescriptor`]s are left untouched, but any inactive
    /// [`MemoryDescriptor`]s are set to an initialized but arbitrary state.
    fn buffer_mut(&mut self) -> &mut [MemoryDescriptor] {
        if let Some(ptr) = self.ptr {
            // SAFETY:
            //
            // The region of memory described by `ptr` is controlled by `self`.
            let maybe_uninit_slice = unsafe {
                slice::from_raw_parts_mut(
                    ptr.as_ptr()
                        .cast::<MaybeUninit<MemoryDescriptor>>()
                        .wrapping_add(self.size),
                    self.capacity - self.size,
                )
            };
            for item in maybe_uninit_slice {
                item.write(MemoryDescriptor {
                    start: 0,
                    count: 0,
                    region_type: MemoryType::RESERVED,
                });
            }

            // SAFETY:
            //
            // The region of memory described by `ptr` is controlled by `self` and is initialized.
            unsafe { slice::from_raw_parts_mut(ptr.as_ptr().cast(), self.capacity) }
        } else {
            &mut []
        }
    }
}

// SAFETY:
//
// [`MemoryMapWrapper`] can be safely sent across threads.
unsafe impl Send for MemoryMapWrapper {}
// SAFETY:
//
// [`MemoryMapWrapper`] can be safely sent across threads.
unsafe impl Sync for MemoryMapWrapper {}

/// Vector of all of the frame regions that have been allocated by the application.
#[derive(Debug, PartialEq, Eq)]
pub struct FrameVec {
    /// The pointer to the buffer controlled by [`FrameVec`].
    ptr: Option<NonNull<(u64, u64)>>,
    /// The number of `(u64, u64)`s the buffer currently contains.
    count: usize,
    /// The number of `(u64, u64)`s the buffer can contain.
    capacity: usize,
}

impl FrameVec {
    /// Returns an empty [`FrameVec`].
    pub const fn new() -> Self {
        Self {
            ptr: None,
            count: 0,
            capacity: 0,
        }
    }

    /// Adds the frame region into the memory map.
    pub fn add_region(&mut self, physical_address: u64, count: u64) {
        if self.count == self.capacity {
            self.reallocate();
        }

        let Some(ptr) = self.ptr else {
            unreachable!("after reallocating, a buffer must have been allocated");
        };

        let write_ptr = ptr.as_ptr().wrapping_add(self.count);
        // SAFETY:
        //
        // The region of memory has been allocated and is under the exclusive control of this
        // [`FrameVec`].
        unsafe { write_ptr.write((physical_address, count)) }

        self.count += 1;
        self.descriptors_mut()
            .sort_unstable_by_key(|descriptor| descriptor.0);

        let mut index = 0;
        while index + 1 < self.count {
            let lower = self.descriptors_mut()[index];
            let upper = self.descriptors_mut()[index + 1];

            let lower_start = lower.0;
            let lower_end = lower_start + lower.1 * frame_size();

            let upper_start = upper.0;
            let upper_end = upper_start + upper.1 * frame_size();

            // Since all entries are sorted, if an ipper entry starts before this entry ends, it
            // must overlap.
            let needs_merge = upper_start <= lower_end;
            if needs_merge {
                let merged_start = lower_start;
                let merged_end = lower_end.max(upper_end);

                let merged_len_frames = (merged_end - merged_start) / frame_size();

                // Write merged descriptor into lower slot.
                self.descriptors_mut()[index] = (merged_start, merged_len_frames);

                // Shift remaining descriptors into the lower slots.
                for i in index + 1..self.count - 1 {
                    self.descriptors_mut()[i] = self.descriptors_mut()[i + 1];
                }

                self.count -= 1;
            } else {
                index += 1;
            }
        }
    }

    /// Removes a region of allocated frames from the memory map.
    pub fn remove_region(&mut self, physical_address: u64, count: u64) {
        let frame_size = frame_size();

        let remove_start = physical_address;
        let remove_end = remove_start + count * frame_size;

        let mut index = 0;
        while index < self.count {
            let (region_start, region_len_frames) = self.descriptors_mut()[index];
            let region_end = region_start + region_len_frames * frame_size;

            // No overlap
            if region_end <= remove_start || region_start >= remove_end {
                index += 1;
                continue;
            }

            // Case 1: fully covered → delete region
            if remove_start <= region_start && remove_end >= region_end {
                for i in index..self.count - 1 {
                    self.descriptors_mut()[i] = self.descriptors_mut()[i + 1];
                }
                self.count -= 1;
                continue; // do not increment index
            }

            // Case 2: remove middle → split
            if remove_start > region_start && remove_end < region_end {
                let left_len_frames = (remove_start - region_start) / frame_size;
                let right_start = remove_end;
                let right_len_frames = (region_end - remove_end) / frame_size;

                // Update left
                self.descriptors_mut()[index] = (region_start, left_len_frames);

                // Make room for right
                for i in (index + 1..=self.count).rev() {
                    self.descriptors_mut()[i] = self.descriptors_mut()[i - 1];
                }

                self.descriptors_mut()[index + 1] = (right_start, right_len_frames);

                self.count += 1;
                return; // removal fully handled
            }

            // Case 3: trim front
            if remove_start <= region_start {
                let new_start = remove_end;
                let new_len_frames = (region_end - remove_end) / frame_size;

                self.descriptors_mut()[index] = (new_start, new_len_frames);

                index += 1;
                continue;
            }

            // Case 4: trim back
            let new_len_frames = (remove_start - region_start) / frame_size;

            self.descriptors_mut()[index] = (region_start, new_len_frames);
            index += 1;
        }
    }

    /// Returns `true` if the region is completely contained within the [`FrameVec`].
    pub fn contains_region(&self, physical_address: u64, count: u64) -> bool {
        if count == 0 {
            return true;
        }

        let frame_size = frame_size();

        let query_start = physical_address;
        let query_end = query_start + count * frame_size;

        let mut covered_until = query_start;

        for i in 0..self.count {
            let (region_start, region_count) = self.descriptors()[i];
            let region_end = region_start + region_count * frame_size;

            // Region is completely before the uncovered part
            if region_end <= covered_until {
                continue;
            }

            // Gap detected
            if region_start > covered_until {
                return false;
            }

            // Extend coverage
            covered_until = region_end;

            if covered_until >= query_end {
                return true;
            }
        }

        false
    }

    /// Returns an immutable slice of the frame regions.
    pub fn descriptors(&self) -> &[(u64, u64)] {
        if let Some(ptr) = self.ptr {
            // SAFETY:
            //
            // The region of memory has been allocated, is under the exclusive control of this
            // [`FrameVec`], and has been initialized up to `self.count`.
            unsafe { slice::from_raw_parts(ptr.as_ptr(), self.count) }
        } else {
            &mut []
        }
    }

    /// Returns a mutable slice of the frame regions.
    pub fn descriptors_mut(&mut self) -> &mut [(u64, u64)] {
        if let Some(ptr) = self.ptr {
            // SAFETY:
            //
            // The region of memory has been allocated, is under the exclusive control of this
            // [`FrameVec`], and has been initialized up to `self.count`.
            unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), self.count) }
        } else {
            &mut []
        }
    }

    fn reallocate(&mut self) {
        let new_capacity = self.capacity.saturating_mul(2);
        let allocation = allocate(
            new_capacity.strict_mul(mem::size_of::<(u64, u64)>()),
            mem::align_of::<(u64, u64)>(),
        )
        .expect("allocation of application frame vector failed");

        let new_slice = unsafe {
            slice::from_raw_parts_mut(
                allocation.ptr().cast::<MaybeUninit<(u64, u64)>>(),
                self.count,
            )
        };

        for (index, &descriptor) in self.descriptors_mut().iter().enumerate() {
            new_slice[index].write(descriptor);
        }

        if let Some(ptr) = self.ptr {
            // If we've allocated a buffer, free it.

            // SAFETY:
            //
            // The region of memory demarcated by `active_ptr` is no longer in use.
            unsafe {
                deallocate(
                    ptr.cast::<u8>(),
                    self.capacity.strict_mul(mem::size_of::<MemoryDescriptor>()),
                    mem::align_of::<MemoryDescriptor>(),
                )
            }
        }

        self.capacity = new_capacity;
        self.ptr = Some(allocation.ptr_nonnull().cast::<(u64, u64)>());

        // Forget [`Allocation`] to prevent early free.
        mem::forget(allocation);
    }
}

// SAFETY:
//
// [`FrameVec`] can be safely sent across threads.
unsafe impl Send for FrameVec {}
// SAFETY:
//
// [`FrameVec`] can be safely sent across threads.
unsafe impl Sync for FrameVec {}

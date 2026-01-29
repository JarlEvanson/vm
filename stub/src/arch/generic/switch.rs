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
        deallocate_frames, frame_size, memory_map, read_bytes_at, read_u64_at, takeover,
        write_u64_at,
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

            // Forget the frame allocation to prevent it from being freed early.
            mem::forget(frame_allocation);
            Status::SUCCESS
        }
        DEALLOCATE_FRAMES_ID => {
            let physical_address = arg_1;
            let count = arg_2;
            // SAFETY:
            //
            // Application must have previously allocated said frames.
            unsafe { deallocate_frames(physical_address, count) }
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

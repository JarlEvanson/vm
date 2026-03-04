//! Cross address space switching function call related functionality.

use core::{
    mem::{self, MaybeUninit},
    ptr::NonNull,
    slice,
};

use conversion::{u64_to_usize_strict, usize_to_u64};
use memory::{
    address::{Address, AddressChunk, AddressChunkRange, Frame, FrameRange, PhysicalAddress},
    phys::PhysicalMemorySpace,
    translation::{MapError, MapFlags, TranslationScheme},
};
use stub_api::{AllocationFlags, MemoryDescriptor, MemoryType, Status, TakeoverFlags};
use sync::Spinlock;

use crate::{
    arch::paging::ArchScheme,
    platform::{
        AllocationPolicy, BufferTooSmall, StubPhysicalMemory, allocate, allocate_frames_aligned,
        deallocate, deallocate_frames, frame_size, memory_map, print, read_bytes_at, remove_range,
        takeover,
    },
    warn,
};

/// The `func_id` representing a return.
pub const RETURN_FUNC_ID: u16 = 0;

// Executable to bootloader calls.

/// The `func_id` of the `write` function.
pub const WRITE_FUNC_ID: u16 = RETURN_FUNC_ID + 1;
/// The `func_id` of the `allocate_frames` function.
pub const ALLOCATE_FRAMES_FUNC_ID: u16 = WRITE_FUNC_ID + 1;
/// The `func_id` of the `deallocate_frames` function.
pub const DEALLOCATE_FRAMES_FUNC_ID: u16 = ALLOCATE_FRAMES_FUNC_ID + 1;
/// The `func_id` of the `get_memory_map` function.
pub const GET_MEMORY_MAP_FUNC_ID: u16 = DEALLOCATE_FRAMES_FUNC_ID + 1;
/// The `func_id` of the `map` function.
pub const MAP_FUNC_ID: u16 = GET_MEMORY_MAP_FUNC_ID + 1;
/// The `func_id` of the `unmap` function.
pub const UNMAP_FUNC_ID: u16 = MAP_FUNC_ID + 1;
/// The `func_id` of the `takeover` function.
pub const TAKEOVER_FUNC_ID: u16 = UNMAP_FUNC_ID + 1;
/// The `func_id` of the `run_on_all_processors` function.
pub const RUN_ON_ALL_PROCESSORS_FUNC_ID: u16 = TAKEOVER_FUNC_ID + 1;

/// The maximum generic executable function ID.
pub const MAX_GENERIC_EXECUTABLE_ID: u16 = RUN_ON_ALL_PROCESSORS_FUNC_ID;

// Bootloader to executable calls.

/// The `func_id` representing entering the executable.
pub const ENTER_FUNC_ID: u16 = RETURN_FUNC_ID + 1;
/// The `func_id` representing `run_on_a_processor` handler.
pub const EXEC_ON_PROCESSOR_FUNC_ID: u16 = ENTER_FUNC_ID + 1;

/// The maximum generic bootloader function ID.
pub const MAX_GENERIC_BOOTLOADER_ID: u16 = RUN_ON_ALL_PROCESSORS_FUNC_ID;

/// The saved memory map.
static MEMORY_MAP: Spinlock<MemoryMapWrapper> = Spinlock::new(MemoryMapWrapper::new());
/// The frames allocated by the application.
pub static FRAME_ALLOCATIONS: Spinlock<FrameVec> = Spinlock::new(FrameVec::new());

/// Handles generic cross address space calls.
pub fn handle_call(
    arch_scheme: &mut ArchScheme,
    func_id: u16,
    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
) -> Status {
    let result = match func_id {
        WRITE_FUNC_ID => write_func(arch_scheme, arg_0, arg_1),
        ALLOCATE_FRAMES_FUNC_ID => allocate_frames_func(arch_scheme, arg_0, arg_1, arg_2, arg_3),
        DEALLOCATE_FRAMES_FUNC_ID => deallocate_frames_func(arch_scheme, arg_0, arg_1),
        GET_MEMORY_MAP_FUNC_ID => {
            get_memory_map_func(arch_scheme, arg_0, arg_1, arg_2, arg_3, arg_4)
        }
        MAP_FUNC_ID => map_func(arch_scheme, arg_0, arg_1, arg_2, arg_3),
        UNMAP_FUNC_ID => unmap_func(arch_scheme, arg_0, arg_1),
        TAKEOVER_FUNC_ID => takeover_func(arg_0, arg_1),
        _ => unreachable!("invalid func_id: {func_id}"),
    };

    result
        .map(|()| Status::SUCCESS)
        .unwrap_or_else(|status| status)
}

/// Implementation of [`stub_api::GenericTable::write`].
fn write_func(arch_scheme: &mut ArchScheme, arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let mut string_ptr = arg_0;
    let mut string_len = arg_1;
    if string_ptr == 0 {
        return Err(Status::INVALID_USAGE);
    }

    let start_ptr = Address::new(string_ptr);
    let Some(end_ptr) = start_ptr.checked_add(string_len - 1) else {
        return Err(Status::INVALID_USAGE);
    };

    if !arch_scheme
        .input_descriptor()
        .is_valid_range(start_ptr.value(), end_ptr.value())
    {
        return Err(Status::INVALID_USAGE);
    }

    let mut buffer = [0; 4096];
    while string_len != 0 {
        let Some((physical_address, _)) = arch_scheme.translate_input(Address::new(string_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };

        let offset = string_ptr % arch_scheme.chunk_size();
        let max_buffer_transfer_size =
            u64_to_usize_strict(string_len.min(usize_to_u64(buffer.len())));
        let max_page_transfer_size = u64_to_usize_strict(arch_scheme.chunk_size() - offset);
        let transfer_size = max_buffer_transfer_size.min(max_page_transfer_size);

        read_bytes_at(
            PhysicalAddress::new(physical_address.value()),
            &mut buffer[..transfer_size],
        );
        let Ok(str) = str::from_utf8(&buffer[..transfer_size]) else {
            crate::warn!("error writing to stub output device");
            return Err(Status::INVALID_USAGE);
        };

        print(format_args!("{str}"));
        string_ptr = string_ptr.wrapping_add(usize_to_u64(transfer_size));
        string_len = string_len.wrapping_sub(usize_to_u64(transfer_size));
    }

    Ok(())
}

/// Implementation of [`stub_api::GenericTable::allocate_frames`].
fn allocate_frames_func(
    arch_scheme: &mut ArchScheme,
    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
) -> Result<(), Status> {
    let count = arg_0;
    let alignment = arg_1;
    let flags = arg_2;
    let physical_address_ptr = arg_3;
    if count == 0 || physical_address_ptr == 0 || flags & AllocationFlags::VALID.0 != flags {
        return Err(Status::INVALID_USAGE);
    }

    let Some((physical_address_ptr_address, _)) =
        arch_scheme.translate_input(Address::new(physical_address_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let policy = match AllocationFlags(flags & 0b11) {
        AllocationFlags::ANY => AllocationPolicy::Any,
        AllocationFlags::BELOW => {
            // SAFETY:
            //
            // The [`ArchScheme`] successfully translated the given `physical_address_ptr`.
            let value = unsafe {
                StubPhysicalMemory
                    .read_u64_le(PhysicalAddress::new(physical_address_ptr_address.value()))
                    .map_err(infallible_to_status)?
            };
            AllocationPolicy::Below(value)
        }
        _ => return Err(Status::INVALID_USAGE),
    };

    let Some(frame_count) = count
        .checked_mul(page_frame_size(arch_scheme))
        .map(|total_bytes| total_bytes.div_ceil(frame_size()))
    else {
        return Err(Status::OUT_OF_MEMORY);
    };

    let Ok(frame_allocation) = allocate_frames_aligned(frame_count, alignment, policy) else {
        return Err(Status::OUT_OF_MEMORY);
    };

    // SAFETY:
    //
    // The [`ArchScheme`] successfully translated the given `physical_address_ptr`.
    unsafe {
        StubPhysicalMemory.write_u64_le(
            PhysicalAddress::new(physical_address_ptr_address.value()),
            frame_allocation
                .range()
                .start()
                .start_address(frame_size())
                .value(),
        )
    };

    FRAME_ALLOCATIONS
        .lock()
        .add_region(frame_allocation.range());

    // Forget the frame allocation to prevent it from being freed early.
    mem::forget(frame_allocation);
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::deallocate_frames`].
fn deallocate_frames_func(
    arch_scheme: &mut ArchScheme,
    arg_0: u64,
    arg_1: u64,
) -> Result<(), Status> {
    let physical_address = arg_0;
    let count = arg_1;

    let Some(frame_count) = count
        .checked_mul(page_frame_size(arch_scheme))
        .map(|total_bytes| total_bytes.div_ceil(frame_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let range = FrameRange::new(
        Frame::containing_address(PhysicalAddress::new(physical_address), frame_size()),
        frame_count,
    );
    let mut frame_allocation = FRAME_ALLOCATIONS.lock();
    if !frame_allocation.contains_region(range) {
        warn!("tried to deallocate unallocated frame region");
        return Err(Status::INVALID_USAGE);
    }

    frame_allocation.remove_region(range);

    // SAFETY:
    //
    // Application must have previously allocated said frames.
    unsafe { deallocate_frames(range) }

    // Ensure that the [`FRAME_ALLOCATIONS`] lock is held until the frames are successfully
    // deallocated.
    drop(frame_allocation);
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::get_memory_map`].
fn get_memory_map_func(
    arch_scheme: &mut ArchScheme,
    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
) -> Result<(), Status> {
    let size_ptr = arg_0;
    let map_buffer_ptr = arg_1;
    let key_ptr = arg_2;
    let descriptor_size_ptr = arg_3;
    let descriptor_version_ptr = arg_4;
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
        return Err(Status::INVALID_USAGE);
    }

    let mut map = MEMORY_MAP.lock();
    map.update();

    let Err(BufferTooSmall { required_count }) = memory_map(&mut []) else {
        // If the memory map is empty, then the memory map is clearly not implemented
        // correctly.
        warn!("empty platform memory map is unusual");
        return Err(Status::INVALID_USAGE);
    };

    let Some((size_physical_address, _)) = arch_scheme.translate_input(Address::new(size_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    // SAFETY:
    //
    // The [`ArchScheme`] successfully translated the given `size_ptr`.
    unsafe {
        StubPhysicalMemory
            .write_u64_le(
                PhysicalAddress::new(size_physical_address.value()),
                usize_to_u64(required_count),
            )
            .map_err(infallible_to_status)?
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
        return Err(Status::INVALID_USAGE);
    }

    let Some((key_physical_address, _)) = arch_scheme.translate_input(Address::new(key_ptr)) else {
        return Err(Status::INVALID_USAGE);
    };

    let Some((descriptor_size_physical_address, _)) =
        arch_scheme.translate_input(Address::new(descriptor_size_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let Some((descriptor_version_physical_address, _)) =
        arch_scheme.translate_input(Address::new(descriptor_version_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    for (index, descriptor) in map.descriptors().iter().enumerate() {
        let descriptor_ptr = map_buffer_ptr.strict_add(usize_to_u64(
            index.strict_mul(mem::size_of::<MemoryDescriptor>()),
        ));
        let number_ptr =
            descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, number)));
        let count_ptr =
            descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, count)));
        let region_type_ptr =
            descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(MemoryDescriptor, region_type)));

        let Some((number_physical_address, _)) =
            arch_scheme.translate_input(Address::new(number_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };
        let Some((count_physical_address, _)) =
            arch_scheme.translate_input(Address::new(count_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };
        let Some((region_type_physical_address, _)) =
            arch_scheme.translate_input(Address::new(region_type_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };

        // TODO: Implement varying frame sizes.

        // SAFETY:
        //
        // The [`ArchScheme`] successfully translated the given pointers.
        #[expect(clippy::multiple_unsafe_ops_per_block)]
        unsafe {
            StubPhysicalMemory
                .write_u64_le(
                    PhysicalAddress::new(number_physical_address.value()),
                    descriptor
                        .number
                        .strict_mul(frame_size())
                        .div_ceil(page_frame_size(arch_scheme)),
                )
                .map_err(infallible_to_status)?;

            StubPhysicalMemory
                .write_u64_le(
                    PhysicalAddress::new(count_physical_address.value()),
                    descriptor
                        .count
                        .strict_mul(frame_size())
                        .div_ceil(page_frame_size(arch_scheme)),
                )
                .map_err(infallible_to_status)?;

            StubPhysicalMemory
                .write_u64_le(
                    PhysicalAddress::new(region_type_physical_address.value()),
                    u64::from(descriptor.region_type.0),
                )
                .map_err(infallible_to_status)?;
        }
    }

    // SAFETY:
    //
    // The [`ArchScheme`] successfully translated the given pointers.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        StubPhysicalMemory
            .write_u64_le(
                PhysicalAddress::new(size_physical_address.value()),
                usize_to_u64(map.size.strict_mul(mem::size_of::<MemoryDescriptor>())),
            )
            .map_err(infallible_to_status)?;

        StubPhysicalMemory
            .write_u64_le(PhysicalAddress::new(key_physical_address.value()), map.key)
            .map_err(infallible_to_status)?;

        StubPhysicalMemory
            .write_u64_le(
                PhysicalAddress::new(descriptor_size_physical_address.value()),
                usize_to_u64(mem::size_of::<MemoryDescriptor>()),
            )
            .map_err(infallible_to_status)?;

        StubPhysicalMemory
            .write_u64_le(
                PhysicalAddress::new(descriptor_version_physical_address.value()),
                MemoryDescriptor::VERSION,
            )
            .map_err(infallible_to_status)?;
    }

    Ok(())
}

/// Implementation of [`stub_api::GenericTable::map`].
fn map_func(
    arch_scheme: &mut ArchScheme,
    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
) -> Result<(), Status> {
    let physical_address = arg_0;
    let virtual_address = arg_1;
    let count = arg_2;
    let flags = arg_3;

    if flags & stub_api::MapFlags::VALID.0 != flags {
        return Err(Status::INVALID_USAGE);
    }
    let mut mapping_flags = MapFlags::READ;
    if flags & stub_api::MapFlags::WRITE.0 == stub_api::MapFlags::WRITE.0 {
        mapping_flags |= MapFlags::WRITE
    }
    if flags & stub_api::MapFlags::EXEC.0 == stub_api::MapFlags::EXEC.0 {
        mapping_flags |= MapFlags::EXEC;
    }

    if flags & stub_api::MapFlags::MAY_OVERWRITE.0 == stub_api::MapFlags::MAY_OVERWRITE.0 {
        mapping_flags |= MapFlags::MAY_OVERWRITE;
    }

    let Some(page_count) = count
        .checked_mul(page_frame_size(arch_scheme))
        .map(|total_bytes| total_bytes.div_ceil(arch_scheme.chunk_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let virtual_chunk = AddressChunkRange::new(
        AddressChunk::containing_address(Address::new(virtual_address), arch_scheme.chunk_size()),
        page_count,
    );
    let physical_chunk = AddressChunkRange::new(
        AddressChunk::containing_address(Address::new(physical_address), arch_scheme.chunk_size()),
        page_count,
    );

    // SAFETY:
    //
    // The executable requested this operation.
    let result = unsafe { arch_scheme.map(virtual_chunk, physical_chunk, mapping_flags) };
    match result {
        Ok(()) => Ok(()),
        Err(MapError::OverlapError) => Err(Status::OVERLAP),
        Err(MapError::OutOfMemory) => Err(Status::OUT_OF_MEMORY),
        Err(_) => Err(Status::INVALID_USAGE),
    }
}
/// Implementation of [`stub_api::GenericTable::map`].
fn unmap_func(arch_scheme: &mut ArchScheme, arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let virtual_address = arg_0;
    let count = arg_1;

    let Some(page_count) = count
        .checked_mul(page_frame_size(arch_scheme))
        .map(|total_bytes| total_bytes.div_ceil(arch_scheme.chunk_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let virtual_chunk = AddressChunkRange::new(
        AddressChunk::containing_address(Address::new(virtual_address), arch_scheme.chunk_size()),
        page_count,
    );

    // SAFETY:
    //
    // The application requested that said page range was unmapped.
    unsafe { arch_scheme.unmap(virtual_chunk) }
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::takeover`].
fn takeover_func(arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let key = arg_0;
    let flags = TakeoverFlags(arg_1);
    if (flags.0 & TakeoverFlags::VALID.0) != flags.0 {
        return Err(Status::INVALID_USAGE);
    }

    // Remove frame regions allocated by the executable to prevent them from being freed
    // when this application exits.
    for range in FRAME_ALLOCATIONS.lock().descriptors() {
        // SAFETY:
        //
        // This region was allocated by the executable and thus it is safe to not free,
        // since the executable is taking over the program.
        unsafe { remove_range(*range) }
    }

    let result = takeover(key, flags);
    if result == Status::SUCCESS {
        Ok(())
    } else {
        Err(result)
    }
}

/// Wrapper around conversion from [`core::convert::Infallible`] to [`Status`].
fn infallible_to_status(_value: core::convert::Infallible) -> Status {
    unreachable!()
}

/// Returns the larger of [`ArchScheme::chunk_size()`] and [`frame_size()`].
fn page_frame_size(arch_scheme: &mut ArchScheme) -> u64 {
    arch_scheme.chunk_size().max(frame_size())
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
                    number: 0,
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
    ptr: Option<NonNull<FrameRange>>,
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
    pub fn add_region(&mut self, range: FrameRange) {
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
        unsafe { write_ptr.write(range) }

        self.count += 1;
        self.descriptors_mut()
            .sort_unstable_by_key(|descriptor| descriptor.start());

        let mut index = 0;
        while index + 1 < self.count {
            let lower = self.descriptors_mut()[index];
            let upper = self.descriptors_mut()[index + 1];

            if let Some(merged) = lower.merge(upper) {
                // Write merged descriptor into lower slot.
                self.descriptors_mut()[index] = merged;

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
    ///
    /// # Panics
    ///
    /// Panics if the provided `range` is not valid for removal.
    pub fn remove_region(&mut self, range: FrameRange) {
        let mut index = 0;
        while index < self.count {
            let region = self.descriptors_mut()[index];

            // No overlap
            if !range.overlaps(region) {
                index += 1;
                continue;
            }

            let (lower, overlaps, upper) = range.partition(region);
            assert_eq!(overlaps, range, "invalid range region unmarked");

            match (lower.is_empty(), upper.is_empty()) {
                (true, true) => {
                    for i in index..self.count - 1 {
                        self.descriptors_mut()[i] = self.descriptors_mut()[i + 1];
                    }
                    self.count -= 1;
                }
                (true, false) => self.descriptors_mut()[index] = upper,
                (false, true) => self.descriptors_mut()[index] = lower,
                (false, false) => {
                    // Shift descriptor to make room for the second [`FrameRange`].
                    for i in (index + 1..=self.count).rev() {
                        self.descriptors_mut()[i] = self.descriptors_mut()[i - 1];
                    }

                    self.descriptors_mut()[index] = lower;
                    self.descriptors_mut()[index + 1] = upper;
                    self.count += 1;
                }
            }

            return;
        }
    }

    /// Returns `true` if the region is completely contained within the [`FrameVec`].
    pub fn contains_region(&self, range: FrameRange) -> bool {
        if range.is_empty() {
            return true;
        }

        let query_start = range.start().start_address(frame_size());
        let query_end = range.end_exclusive().end_address_exclusive(frame_size());

        let mut covered_until = query_start;

        for i in 0..self.count {
            let region = self.descriptors()[i];

            let region_start = region.start().start_address(frame_size());
            let region_end = region.end_exclusive().end_address_exclusive(frame_size());

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
    pub fn descriptors(&self) -> &[FrameRange] {
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
    pub fn descriptors_mut(&mut self) -> &mut [FrameRange] {
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

    /// Reallocates the [`FrameVec`] buffer.
    fn reallocate(&mut self) {
        // Double the capacity or initialize the capacity to 8 entries.
        let new_capacity = self.capacity.saturating_mul(2).max(8);
        let allocation = allocate(
            new_capacity.strict_mul(mem::size_of::<FrameRange>()),
            mem::align_of::<FrameRange>(),
        )
        .expect("allocation of application frame vector failed");

        // SAFETY:
        //
        // The buffer is under the exclusive control of `self` and does not need to be initialized
        // since [`MaybeUninit`] does not require initialization.
        let new_slice = unsafe {
            slice::from_raw_parts_mut(
                allocation.ptr().cast::<MaybeUninit<FrameRange>>(),
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
                    self.capacity.strict_mul(mem::size_of::<FrameRange>()),
                    mem::align_of::<MemoryDescriptor>(),
                )
            }
        }

        self.capacity = new_capacity;
        self.ptr = Some(allocation.ptr_nonnull().cast::<FrameRange>());

        // Forget [`Allocation`] to prevent early free.
        mem::forget(allocation);
    }
}

impl Default for FrameVec {
    fn default() -> Self {
        Self::new()
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

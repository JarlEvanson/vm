//! Cross address space switching function call related functionality.

use core::{
    alloc::Layout,
    mem::{self, MaybeUninit},
    ptr::NonNull,
    slice,
};

use conversion::{u64_to_usize_strict, usize_to_u32_strict, usize_to_u64};
use stub_api::{AllocationFlags, Status, TakeoverFlags};
use sync::Spinlock;

use crate::{
    arch::generic::memory::paging::{
        ExternalFrame, ExternalFrameRange, ExternalPage, ExternalPageRange,
        ExternalPhysicalAddress, ExternalVirtualAddress, ExternalVirtualAddressRange,
        TranslationScheme,
    },
    platform::{
        AllocationPolicy, BufferTooSmall, Frame, FrameRange, MapError, MappingType,
        MemoryDescriptor, MemoryType, OutOfMemory, Permissions, PhysicalAddress,
        PhysicalAddressRange, allocate, allocate_frames_aligned, deallocate, deallocate_frames,
        frame_size, memory_map, read_bytes_at, read_u32_at, read_u64_at, write_u32_at,
        write_u64_at,
    },
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

/// Handles generic cross address space calls.
pub fn handle_call<T: TranslationScheme>(
    scheme: &mut T,
    func_id: u16,
    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
) -> Status {
    let result = match func_id {
        WRITE_FUNC_ID => write_func(scheme, arg_0, arg_1),
        ALLOCATE_FRAMES_FUNC_ID => allocate_frames_func(scheme, arg_0, arg_1, arg_2, arg_3),
        DEALLOCATE_FRAMES_FUNC_ID => deallocate_frames_func(scheme, arg_0, arg_1),
        GET_MEMORY_MAP_FUNC_ID => get_memory_map_func(scheme, arg_0, arg_1, arg_2, arg_3, arg_4),
        MAP_FUNC_ID => map_func(scheme, arg_0, arg_1, arg_2, arg_3),
        UNMAP_FUNC_ID => unmap_func(scheme, arg_0, arg_1),
        TAKEOVER_FUNC_ID => takeover_func(arg_0, arg_1),
        _ => unreachable!("invalid func_id: {func_id}"),
    };

    result
        .map(|()| Status::SUCCESS)
        .unwrap_or_else(|status| status)
}

/// Implementation of [`stub_api::GenericTable::write`].
fn write_func<T: TranslationScheme>(scheme: &mut T, arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let mut string_ptr = arg_0;
    let mut string_len = arg_1;
    if string_ptr == 0 {
        return Err(Status::INVALID_USAGE);
    }

    let start_ptr = ExternalVirtualAddress::new(string_ptr);
    let Some(end_ptr) = start_ptr.checked_add(string_len - 1) else {
        return Err(Status::INVALID_USAGE);
    };

    if !scheme
        .input_descriptor()
        .is_valid_range(start_ptr.value(), end_ptr.value())
    {
        return Err(Status::INVALID_USAGE);
    }

    let mut buffer = [0; 4096];

    let mut carry_over_length = 0;
    while string_len != 0 || carry_over_length != 0 {
        // Calculate remaining space in the buffer.
        let buffer_space = buffer.len() - carry_over_length;

        // Unify the incomplete message handling.
        if string_len == 0 && carry_over_length != 0 {
            crate::warn!("error writing to stub output: incomplete UTF-8 message");
            return Err(Status::INVALID_USAGE);
        }

        // Convert the pointer in `revm`'s address space to the corresponding physical address.
        let Some((_, _, physical_address)) =
            scheme.translate(ExternalVirtualAddress::new(string_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };

        // Compute the maximum size of a single read.
        let offset = string_ptr % scheme.chunk_size();
        let max_buffer_transfer_size =
            u64_to_usize_strict(string_len.min(usize_to_u64(buffer_space)));
        let max_page_transfer_size = u64_to_usize_strict(scheme.chunk_size() - offset);
        let transfer_size = max_buffer_transfer_size.min(max_page_transfer_size);

        // Read the message bytes.
        let succeeded = read_bytes_at(
            PhysicalAddress::new(physical_address.value()),
            &mut buffer[carry_over_length..][..transfer_size],
        );
        if !succeeded {
            crate::warn!("error writing to stub output: failed to read revm buffer");
            return Err(Status::INVALID_USAGE);
        }

        // Handle UTF-8 validation and partial sequences.
        let total_buffer_size = carry_over_length + transfer_size;
        let valid_str = match str::from_utf8(&buffer[..total_buffer_size]) {
            Ok(s) => {
                carry_over_length = 0;
                s
            }
            Err(e) => {
                if e.error_len().is_some() {
                    crate::warn!("error writing to stub output: malformed UTF-8 message");
                    return Err(Status::INVALID_USAGE);
                }

                let valid_up_to = e.valid_up_to();
                match str::from_utf8(&buffer[..valid_up_to]) {
                    Ok(valid_prefix) => valid_prefix,
                    Err(_) => unreachable!(),
                }
            }
        };

        if !valid_str.is_empty() {
            crate::info!("{valid_str}");
        }

        // Handle partial sequence carry over.
        if valid_str.len() < total_buffer_size {
            let valid_length = valid_str.len();
            buffer.copy_within(valid_length..total_buffer_size, 0);
            carry_over_length = total_buffer_size - valid_length;
        };

        string_ptr = string_ptr.wrapping_add(usize_to_u64(transfer_size));
        string_len = string_len.wrapping_sub(usize_to_u64(transfer_size));
    }

    Ok(())
}

/// Implementation of [`stub_api::GenericTable::allocate_frames`].
fn allocate_frames_func<T: TranslationScheme>(
    scheme: &mut T,
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

    let Some((_, _, physical_address_ptr_address)) =
        scheme.translate(ExternalVirtualAddress::new(physical_address_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };
    let physical_address_ptr_address = PhysicalAddress::new(physical_address_ptr_address.value());

    let policy = match AllocationFlags(flags & 0b11) {
        AllocationFlags::ANY => AllocationPolicy::Any,
        AllocationFlags::AT => {
            let Some(value) = read_u64_at(physical_address_ptr_address) else {
                return Err(Status::INVALID_USAGE);
            };

            AllocationPolicy::At(value)
        }
        AllocationFlags::INCLUSIVE_MAX => {
            let Some(value) = read_u64_at(physical_address_ptr_address) else {
                return Err(Status::INVALID_USAGE);
            };

            AllocationPolicy::InclusiveMax(value)
        }
        _ => return Err(Status::INVALID_USAGE),
    };

    let Some(frame_count) = count
        .checked_mul(page_frame_size(scheme))
        .map(|total_bytes| total_bytes.div_ceil(frame_size()))
    else {
        return Err(Status::OUT_OF_MEMORY);
    };

    let Ok(frame_allocation) = allocate_frames_aligned(frame_count, alignment, policy) else {
        return Err(Status::OUT_OF_MEMORY);
    };

    if !write_u64_at(
        physical_address_ptr_address,
        frame_allocation.range().start_address().value(),
    ) {
        return Err(Status::INVALID_USAGE);
    }

    // Forget the frame allocation to prevent it from being freed early.
    mem::forget(frame_allocation);
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::deallocate_frames`].
fn deallocate_frames_func<T: TranslationScheme>(
    scheme: &mut T,
    arg_0: u64,
    arg_1: u64,
) -> Result<(), Status> {
    let physical_address = arg_0;
    let count = arg_1;

    let Some(frame_count) = count
        .checked_mul(page_frame_size(scheme))
        .map(|total_bytes| total_bytes.div_ceil(frame_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let range = FrameRange::new(
        Frame::containing_address(PhysicalAddress::new(physical_address)),
        frame_count,
    );

    // SAFETY:
    //
    // Application must have previously allocated said frames.
    unsafe { deallocate_frames(range) }
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::get_memory_map`].
fn get_memory_map_func<T: TranslationScheme>(
    scheme: &mut T,
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
        "get_memory_map(\
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
        crate::warn!("empty platform memory map is unusual");
        return Err(Status::INVALID_USAGE);
    };

    let Some((_, _, size_physical_address)) =
        scheme.translate(ExternalVirtualAddress::new(size_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let bits_32 = bits_32(scheme);
    let buffer_size = if bits_32 {
        read_u32_at(PhysicalAddress::new(size_physical_address.value())).map(u64::from)
    } else {
        read_u64_at(PhysicalAddress::new(size_physical_address.value()))
    };
    let Some(buffer_size) = buffer_size else {
        crate::warn!("error reading get_memory_map() size");
        return Err(Status::INVALID_USAGE);
    };

    let write_success = if bits_32 {
        write_u32_at(
            PhysicalAddress::new(size_physical_address.value()),
            usize_to_u32_strict(required_count),
        )
    } else {
        write_u64_at(
            PhysicalAddress::new(size_physical_address.value()),
            usize_to_u64(required_count),
        )
    };
    if !write_success {
        crate::warn!("error writing to get_memory_map() size");
        return Err(Status::INVALID_USAGE);
    }

    let total_buffer_size = required_count
        .checked_mul(mem::size_of::<stub_api::MemoryDescriptor>())
        .expect("buffer is too large");
    let total_buffer_size = usize_to_u64(total_buffer_size);
    if buffer_size < total_buffer_size {
        return Err(Status::BUFFER_TOO_SMALL);
    }

    let buffer_range = ExternalVirtualAddressRange::new(
        ExternalVirtualAddress::new(map_buffer_ptr),
        ExternalVirtualAddress::new(map_buffer_ptr.strict_add(buffer_size.saturating_sub(1))),
    );
    if !scheme.input_descriptor().is_valid_range(
        buffer_range.start().value(),
        buffer_range.end_inclusive().value(),
    ) {
        crate::warn!("provided buffer is inaccessible");
        return Err(Status::INVALID_USAGE);
    }

    // Validate that the pointers are properly aligned and are not NULL.
    if map_buffer_ptr == 0
        || !map_buffer_ptr
            .is_multiple_of(usize_to_u64(mem::align_of::<stub_api::MemoryDescriptor>()))
        || key_ptr == 0
        || !key_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
        || descriptor_size_ptr == 0
        || !descriptor_size_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
        || descriptor_version_ptr == 0
        || !descriptor_version_ptr.is_multiple_of(usize_to_u64(mem::align_of::<u64>()))
    {
        return Err(Status::INVALID_USAGE);
    }

    let Some((_, _, key_physical_address)) = scheme.translate(ExternalVirtualAddress::new(key_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let Some((_, _, descriptor_size_physical_address)) =
        scheme.translate(ExternalVirtualAddress::new(descriptor_size_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let Some((_, _, descriptor_version_physical_address)) =
        scheme.translate(ExternalVirtualAddress::new(descriptor_version_ptr))
    else {
        return Err(Status::INVALID_USAGE);
    };

    for (index, descriptor) in map.descriptors().iter().enumerate() {
        let descriptor_ptr = map_buffer_ptr.strict_add(usize_to_u64(
            index.strict_mul(mem::size_of::<stub_api::MemoryDescriptor>()),
        ));

        let start_ptr = descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(
            stub_api::MemoryDescriptor,
            start
        )));
        let count_ptr = descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(
            stub_api::MemoryDescriptor,
            count
        )));
        let region_type_ptr = descriptor_ptr.strict_add(usize_to_u64(mem::offset_of!(
            stub_api::MemoryDescriptor,
            region_type
        )));

        let Some((_, _, start_physical_address)) =
            scheme.translate(ExternalVirtualAddress::new(start_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };
        let Some((_, _, count_physical_address)) =
            scheme.translate(ExternalVirtualAddress::new(count_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };
        let Some((_, _, region_type_physical_address)) =
            scheme.translate(ExternalVirtualAddress::new(region_type_ptr))
        else {
            return Err(Status::INVALID_USAGE);
        };

        if !write_u64_at(
            PhysicalAddress::new(start_physical_address.value()),
            descriptor.range.start().value(),
        ) {
            crate::warn!("error writing descriptor to get_memory_map buffer");
            return Err(Status::INVALID_USAGE);
        }

        if !write_u64_at(
            PhysicalAddress::new(count_physical_address.value()),
            descriptor.range.count(),
        ) {
            crate::warn!("error writing descriptor to get_memory_map buffer");
            return Err(Status::INVALID_USAGE);
        }

        let region_type = match descriptor.region_type {
            MemoryType::Free => stub_api::MemoryType::FREE,
            MemoryType::BootloaderReclaimable => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
            MemoryType::Bad => stub_api::MemoryType::BAD,
            MemoryType::Reserved => stub_api::MemoryType::RESERVED,
            MemoryType::AcpiReclaimable => stub_api::MemoryType::ACPI_RECLAIMABLE,
            MemoryType::AcpiNonVolatile => stub_api::MemoryType::ACPI_NON_VOLATILE,
        };
        if !write_u32_at(
            PhysicalAddress::new(region_type_physical_address.value()),
            region_type.0,
        ) {
            crate::warn!("error writing descriptor to get_memory_map buffer");
            return Err(Status::INVALID_USAGE);
        }
    }

    let size = map
        .size
        .strict_mul(mem::size_of::<stub_api::MemoryDescriptor>());
    let write_success = if bits_32 {
        write_u32_at(
            PhysicalAddress::new(size_physical_address.value()),
            usize_to_u32_strict(size),
        )
    } else {
        write_u64_at(
            PhysicalAddress::new(size_physical_address.value()),
            usize_to_u64(size),
        )
    };
    if !write_success {
        crate::warn!("error writing to get_memory_map() size");
        return Err(Status::INVALID_USAGE);
    }

    if !write_u64_at(PhysicalAddress::new(key_physical_address.value()), map.key) {
        crate::warn!("error writing to get_memory_map() key");
        return Err(Status::INVALID_USAGE);
    }

    let write_success = if bits_32 {
        write_u32_at(
            PhysicalAddress::new(descriptor_size_physical_address.value()),
            usize_to_u32_strict(mem::size_of::<stub_api::MemoryDescriptor>()),
        )
    } else {
        write_u64_at(
            PhysicalAddress::new(descriptor_size_physical_address.value()),
            usize_to_u64(mem::size_of::<stub_api::MemoryDescriptor>()),
        )
    };
    if !write_success {
        crate::warn!("error writing to get_memory_map() descriptor size");
        return Err(Status::INVALID_USAGE);
    }

    if !write_u64_at(
        PhysicalAddress::new(descriptor_version_physical_address.value()),
        stub_api::MemoryDescriptor::VERSION,
    ) {
        crate::warn!("error writing to get_memory_map() descriptor version");
        return Err(Status::INVALID_USAGE);
    }

    Ok(())
}

/// Implementation of [`stub_api::GenericTable::map`].
fn map_func<T: TranslationScheme>(
    scheme: &mut T,
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

    if count != 0 {
        return Err(Status::INVALID_USAGE);
    }

    let writable = flags & stub_api::MapFlags::WRITE.0 == stub_api::MapFlags::WRITE.0;
    let executable = flags & stub_api::MapFlags::EXEC.0 == stub_api::MapFlags::EXEC.0;
    let permissions = match (writable, executable) {
        (true, true) => Permissions::ReadWriteExecute,
        (true, false) => Permissions::ReadWrite,
        (false, true) => Permissions::ReadExecute,
        (false, false) => Permissions::Read,
    };

    let Some(page_count) = count
        .checked_mul(page_frame_size(scheme))
        .map(|total_bytes| total_bytes.div_ceil(scheme.chunk_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let start = ExternalPage::containing_address(
        ExternalVirtualAddress::new(virtual_address),
        scheme.chunk_size(),
    );
    let end = start.strict_add(page_count.saturating_sub(1));
    let page_range = ExternalPageRange::new(start, end);

    let frame_range = ExternalFrameRange::new(
        ExternalFrame::containing_address(
            ExternalPhysicalAddress::new(physical_address),
            scheme.chunk_size(),
        ),
        page_count,
    );

    let may_overwrite =
        flags & stub_api::MapFlags::MAY_OVERWRITE.0 == stub_api::MapFlags::MAY_OVERWRITE.0;
    if may_overwrite {
        // SAFETY:
        //
        // The executable requested this operation.
        unsafe { scheme.unmap(page_range) }
    }

    let result = scheme.map_at(page_range, frame_range, permissions, MappingType::Normal);
    match result {
        Ok(()) => Ok(()),
        Err(MapError::FindFreeRegionError) => Err(Status::OVERLAP),
        Err(MapError::FrameAllocation(OutOfMemory)) => Err(Status::OUT_OF_MEMORY),
    }
}

/// Implementation of [`stub_api::GenericTable::map`].
fn unmap_func<T: TranslationScheme>(scheme: &mut T, arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let virtual_address = arg_0;
    let count = arg_1;
    if count == 0 {
        return Err(Status::INVALID_USAGE);
    }

    let Some(page_count) = count
        .checked_mul(page_frame_size(scheme))
        .map(|total_bytes| total_bytes.div_ceil(scheme.chunk_size()))
    else {
        return Err(Status::INVALID_USAGE);
    };

    let start = ExternalPage::containing_address(
        ExternalVirtualAddress::new(virtual_address),
        scheme.chunk_size(),
    );
    let end = start.strict_add(page_count.saturating_sub(1));
    let virtual_chunk = ExternalPageRange::new(start, end);

    // SAFETY:
    //
    // The application requested that said page range was unmapped.
    unsafe { scheme.unmap(virtual_chunk) }
    Ok(())
}

/// Implementation of [`stub_api::GenericTable::takeover`].
fn takeover_func(arg_0: u64, arg_1: u64) -> Result<(), Status> {
    let key = arg_0;
    let flags = TakeoverFlags(arg_1);
    if (flags.0 & TakeoverFlags::VALID.0) != flags.0 {
        return Err(Status::INVALID_USAGE);
    }

    todo!("implement takeover({key:#x}, {flags:?})")
}

/// Returns the larger of [`TranslationScheme::chunk_size()`] and [`frame_size()`].
fn page_frame_size<T: TranslationScheme>(scheme: &T) -> u64 {
    scheme.chunk_size().max(frame_size())
}

/// Returns `true` if the [`TranslationScheme`] represents a 32-bit address space.
fn bits_32<T: TranslationScheme>(scheme: &T) -> bool {
    scheme
        .input_descriptor()
        .valid_ranges()
        .iter()
        .filter(|(start, end)| start <= end)
        .map(|(_, end)| end)
        .copied()
        .max()
        .expect("TranslationScheme must have valid ranges")
        <= u64::from(u32::MAX)
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

                        let layout = Layout::array::<MemoryDescriptor>(self.capacity).expect("Layout computation must have succeeded in order to allocate this pointer");
                        // SAFETY:
                        //
                        // The region of memory demarcated by `active_ptr` is no longer in use.
                        unsafe { deallocate(active_ptr.cast::<u8>(), layout) }
                    }

                    // Add additional entries to account for memory allocation.
                    let new_count = required_count.strict_add(4);
                    let layout = Layout::array::<MemoryDescriptor>(required_count)
                        .expect("memory map buffer is too large");

                    self.capacity = new_count;
                    self.size = 0;
                    self.ptr = allocate(layout).map(|ptr| ptr.cast::<MemoryDescriptor>());
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
                    range: PhysicalAddressRange::empty(),
                    region_type: MemoryType::Reserved,
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

//! Support for booting from an UEFI platform implementation.

use core::{
    ffi,
    fmt::{self, Write},
    hint, mem,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{AtomicPtr, Ordering},
};

use stub_api::TakeoverFlags;
use sync::Spinlock;
use uefi::{
    data_type::{Guid, Handle, Status},
    memory::{MemoryDescriptor, MemoryType},
    table::{
        boot::AllocateType,
        config::{ACPI, ACPI_2, DEVICE_TREE, SMBIOS, SMBIOS_3},
        system::SystemTable,
    },
};

use crate::{
    platform::{
        AllocationPolicy, BufferTooSmall, OutOfMemory, Platform, allocate, deallocate,
        deallocate_all, deallocate_all_frames,
        memory_structs::{Frame, FrameRange, PhysicalAddress, VirtualAddress},
        platform_initialize, platform_teardown,
    },
    util::{u64_to_usize_panicking, usize_to_u64},
};

/// The [`Handle`] representing the image.
static IMAGE_HANLDE: AtomicPtr<ffi::c_void> = AtomicPtr::new(ptr::null_mut());
/// The program's UEFI table.
static UEFI_SYSTEM_TABLE: Spinlock<Option<UefiSystemTable>> = Spinlock::new(None);
/// The saved UEFI memory map.
static MEMORY_MAP: Spinlock<MemoryMap> = Spinlock::new(MemoryMap::new());

/// Rust entrypoint for the UEFI environment.
pub extern "efiapi" fn uefi_main(
    _image_handle: Handle,
    system_table_ptr: *mut SystemTable,
) -> Status {
    *UEFI_SYSTEM_TABLE.lock() = NonNull::new(system_table_ptr).map(UefiSystemTable);
    // SAFETY:
    //
    // This call is made before any calls to the [`Platform`] APIs are made and there is no
    // multi-threading at this point.
    unsafe { platform_initialize(&Uefi) };
    *crate::PANIC_FUNC.lock() = panic_handler;

    crate::debug!("{:#x}", crate::util::image_start());
    let success = match crate::stub_main() {
        Ok(()) => true,
        Err(error) => {
            crate::error!("error loading from UEFI: {error}");
            false
        }
    };

    // SAFETY:
    //
    // Clean up any allocated frames before tearing down.
    unsafe { deallocate_all_frames() }
    // SAFETY:
    //
    // Clean up any allocated pool memory before tearing down.
    unsafe { deallocate_all() }
    // SAFETY:
    //
    // The only action performed after tearing the [`Platform`] down is returning.
    unsafe { platform_teardown() }
    if success {
        Status::SUCCESS
    } else {
        Status::LOAD_ERROR
    }
}

/// Implementation of [`Platform`] for UEFI.
pub struct Uefi;

impl Platform for Uefi {
    fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
        let system_table_ptr = (*UEFI_SYSTEM_TABLE.lock())
            .expect("illegal call of `allocate()`")
            .0;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `allocate_pool` function pointer.
        let allocate_pool_ptr = unsafe { (*boot_services_ptr).allocate_pool };
        if alignment <= 8 {
            let mut ptr = ptr::null_mut();

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `allocate_pool`.
            let result = unsafe { allocate_pool_ptr(MemoryType::LOADER_DATA, size, &mut ptr) };
            match result {
                Status::SUCCESS => NonNull::new(ptr.cast::<u8>()),
                Status::OUT_OF_RESOURCES => None,
                status => panic!("error allocating memory: {status:?}"),
            }
        } else {
            todo!("implement alignment greater than 8 bytes")
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _size: usize, alignment: usize) {
        let system_table_ptr = (*UEFI_SYSTEM_TABLE.lock())
            .expect("illegal call of `deallocate()`")
            .0;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `free_pool` function pointer.
        let free_pool_ptr = unsafe { (*boot_services_ptr).free_pool };
        if alignment <= 8 {
            // All UEFI pool allocations are 8-byte aligned.
            //
            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `free_pool`.
            let result = unsafe { free_pool_ptr(ptr.as_ptr().cast::<ffi::c_void>()) };
            assert_eq!(result, Status::SUCCESS, "error deallocating memory");
        } else {
            todo!("implement alignment greater than 8 bytes")
        }
    }

    fn frame_size(&self) -> u64 {
        // The UEFI specification states that the frame size is always 4096 bytes.
        4096
    }

    fn allocate_frames(
        &self,
        count: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory> {
        let (allocation_type, mut physical_address) = match policy {
            AllocationPolicy::Any => (AllocateType::ANY_PAGES, 0),
            AllocationPolicy::At(value) => (AllocateType::ADDRESS, value),
            AllocationPolicy::Below(value) => (AllocateType::MAX_ADDRESS, value),
        };

        let system_table_ptr = (*UEFI_SYSTEM_TABLE.lock())
            .expect("illegal call of `allocate_frames()`")
            .0;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `allocate_pages` function pointer.
        let allocate_pages_ptr = unsafe { (*boot_services_ptr).allocate_pages };

        let count = usize::try_from(count).map_err(|_| OutOfMemory)?;

        // SAFETY:
        //
        // `free_pages_ptr` came from a valid [`BootServices`] table and its arguments are
        // correct according to the UEFI specification.
        let status = unsafe {
            allocate_pages_ptr(
                allocation_type,
                MemoryType::LOADER_CODE,
                count,
                &mut physical_address,
            )
        };
        if status == Status::SUCCESS {
            let start = Frame::containing_address(PhysicalAddress::new(physical_address));

            Ok(FrameRange::new(start, usize_to_u64(count)))
        } else if status == Status::OUT_OF_RESOURCES {
            Err(OutOfMemory)
        } else {
            panic!("error allocating frame region of size {count}: {status:?}")
        }
    }

    unsafe fn deallocate_frames(&self, mut range: FrameRange) {
        let system_table_ptr = (*UEFI_SYSTEM_TABLE.lock())
            .expect("illegal call of `allocate_frames()`")
            .0;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `free_pages` function pointer.
        let free_pages_ptr = unsafe { (*boot_services_ptr).free_pages };

        while !range.is_empty() {
            let iter_count = u64_to_usize_panicking(range.count().min(usize_to_u64(usize::MAX)));

            // SAFETY:
            //
            // `free_pages_ptr` came from a valid [`BootServices`] table and its arguments are
            // correct according to the UEFI specification.
            let status =
                unsafe { free_pages_ptr(range.start().start_address().value(), iter_count) };
            if status.error() {
                crate::warn!("error deallocating frames: {status:?}");
            }

            let new_start = range.start().add(usize_to_u64(iter_count));
            range = FrameRange::from_exclusive(new_start, range.end());
        }
    }

    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [stub_api::MemoryDescriptor],
    ) -> Result<crate::platform::generic::MemoryMap<'buffer>, BufferTooSmall> {
        let mut memory_map = MEMORY_MAP.lock();

        memory_map.update();
        let total_entries_required = memory_map.descriptors().count();
        if buffer.len() < total_entries_required {
            return Err(BufferTooSmall {
                required_count: total_entries_required,
            });
        }

        for (index, descriptor) in memory_map.descriptors().enumerate() {
            let region_type = match descriptor.region_type {
                MemoryType::CONVENTIONAL => stub_api::MemoryType::FREE,
                MemoryType::LOADER_CODE => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
                MemoryType::LOADER_DATA => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
                MemoryType::BOOT_SERVICES_CODE => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
                MemoryType::BOOT_SERVICES_DATA => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
                MemoryType::UNUSABLE => stub_api::MemoryType::BAD,
                MemoryType::ACPI_RECLAIM => stub_api::MemoryType::ACPI_RECLAIMABLE,
                MemoryType::ACPI_NVS => stub_api::MemoryType::ACPI_NON_VOLATILE,
                _ => stub_api::MemoryType::RESERVED,
            };

            buffer[index] = stub_api::MemoryDescriptor {
                // This is safe to do because the UEFI specification requires that `physical_start`
                // is always 4 KiB aligned.
                number: descriptor.physical_start / self.frame_size(),
                count: descriptor.number_of_pages,
                region_type,
            };
        }

        Ok(crate::platform::generic::MemoryMap::new(
            &mut buffer[..total_entries_required],
            usize_to_u64(memory_map.key),
        ))
    }

    fn page_size(&self) -> usize {
        // The UEFI specification states that the page size is always 4096 bytes.
        4096
    }

    #[expect(clippy::as_conversions)]
    fn map_temporary(&self, address: PhysicalAddress) -> *mut u8 {
        assert_ne!(address.value(), 0, "can't map zero address on UEFI");
        assert_eq!(
            address.value() as usize as u64,
            address.value(),
            "can't map physical memory beyond virtual memory region on UEFI"
        );

        ptr::with_exposed_provenance_mut(u64_to_usize_panicking(address.value()))
    }

    #[expect(clippy::as_conversions)]
    fn map_identity(&self, address: PhysicalAddress, size: u64) -> *mut u8 {
        assert_ne!(address.value(), 0, "can't map zero address on UEFI");
        assert_eq!(
            address.value() as usize as u64,
            address.value(),
            "can't map physical memory beyond virtual memory region on UEFI"
        );

        let end_address = address.value().strict_add(size);
        assert_eq!(
            end_address as usize as u64, end_address,
            "can't map physical memory beyond virtual memory region on UEFI"
        );

        ptr::with_exposed_provenance_mut(u64_to_usize_panicking(address.value()))
    }

    fn translate_virtual(&self, address: VirtualAddress) -> Option<u64> {
        Some(usize_to_u64(address.value()))
    }

    fn takeover(&self, _key: u64, _flags: TakeoverFlags) -> stub_api::Status {
        todo!()
    }

    fn print(&self, args: core::fmt::Arguments) {
        struct Printer;

        impl fmt::Write for Printer {
            #[expect(clippy::as_conversions)]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                const BUFFER_SIZE: usize = 128;

                let system_table_ptr = (*UEFI_SYSTEM_TABLE.lock())
                    .expect("illegal call of `allocate_frames()`")
                    .0;

                // SAFETY:
                //
                // `system_table_ptr` was provided by the `efi_main` entry point.
                let con_out = unsafe { system_table_ptr.as_ref().con_out };
                if con_out.is_null() {
                    return Err(fmt::Error);
                }

                // SAFETY:
                //
                // `con_out` should be a valid SimpleTextOutputProcotol structure, which is guaranteed to
                // contain the `output_string` function.
                let output_string_func = unsafe { (*con_out).output_string };

                let mut buffer = [0u16; BUFFER_SIZE + 1];
                let mut index = 0;

                let mut chars = s.chars();
                let mut next_char = chars.next();

                let mut newline_processed = false;
                while let Some(mut c) = next_char.take() {
                    if c == '\n' && !newline_processed {
                        newline_processed = true;

                        next_char = Some(c);
                        c = '\r';
                    } else {
                        newline_processed = false;
                    }

                    if c.len_utf16() != 1 {
                        // Character is unrepresentable in UCS-2, replace with replacement character.
                        c = '\u{FFFD}';
                    }

                    buffer[index] = c as u16;
                    index += 1;

                    if index == BUFFER_SIZE {
                        let string = &mut buffer[..=index];
                        string[index] = 0;

                        // Ignore any warnings/errors (we can't fix them and logging them could cause a
                        // stack overflow).
                        //
                        // SAFETY:
                        //
                        // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
                        // pointer, which means it is safe to be called.
                        let _ = unsafe { output_string_func(con_out, string.as_mut_ptr()) };
                        index = 0;
                    }

                    if next_char.is_none() {
                        next_char = chars.next();
                    }
                }

                if index != 0 {
                    let string = &mut buffer[..=index];
                    string[index] = 0;

                    // Ignore any warnings/errors (we can't fix them and logging them could cause a
                    // stack overflow).
                    //
                    // SAFETY:
                    //
                    // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
                    // pointer, which means it is safe to be called.
                    let _ = unsafe { output_string_func(con_out, string.as_mut_ptr()) };
                }

                Ok(())
            }
        }

        static PRINTER: Spinlock<Printer> = Spinlock::new(Printer);

        let _ = PRINTER.lock().write_fmt(args);
    }

    fn uefi_system_table(&self) -> Option<PhysicalAddress> {
        UEFI_SYSTEM_TABLE
            .lock()
            .map(|table| PhysicalAddress::new(usize_to_u64(table.0.as_ptr().addr())))
    }

    fn rsdp(&self) -> Option<PhysicalAddress> {
        lookup_config_table((*UEFI_SYSTEM_TABLE.lock())?.0, ACPI)
    }

    fn xsdp(&self) -> Option<PhysicalAddress> {
        lookup_config_table((*UEFI_SYSTEM_TABLE.lock())?.0, ACPI_2)
    }

    fn device_tree(&self) -> Option<PhysicalAddress> {
        lookup_config_table((*UEFI_SYSTEM_TABLE.lock())?.0, DEVICE_TREE)
    }

    fn smbios_32(&self) -> Option<PhysicalAddress> {
        lookup_config_table((*UEFI_SYSTEM_TABLE.lock())?.0, SMBIOS)
    }

    fn smbios_64(&self) -> Option<PhysicalAddress> {
        lookup_config_table((*UEFI_SYSTEM_TABLE.lock())?.0, SMBIOS_3)
    }
}

/// Wrapper around simple updates of the UEFI memory map.
struct MemoryMap {
    /// Pointer to the start of the UEFI memory map buffer.
    ptr: Option<NonNull<u8>>,
    /// The capacity, in bytes, of the buffer.
    capacity: usize,
    /// The size, in bytes, of the valid portion of the buffer.
    size: usize,
    /// A unique key for the current memory map.
    key: usize,
    /// The size, in bytes, of each descriptor.
    descriptor_size: usize,
    /// The version of the UEFI memory descriptor.
    descriptor_version: u32,
}

impl MemoryMap {
    /// Returns an empty [`MemoryMap`].
    pub const fn new() -> Self {
        MemoryMap {
            ptr: None,
            capacity: 0,
            size: 0,
            key: 0,
            descriptor_size: 0,
            descriptor_version: 0,
        }
    }

    /// Refreshes the memory map.
    pub fn update(&mut self) {
        let system_table_ptr = UEFI_SYSTEM_TABLE
            .lock()
            .expect("illegal call of `update()`")
            .0;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` should point to a valid `BootServices1_0` structure, which
        // is guaranteed to contain the `get_memory_map` function.
        let get_memory_map = unsafe { (*boot_services_ptr).get_memory_map };

        let mut active_ptr = self.ptr.take();
        let mut buffer_capacity = self.capacity;
        let mut buffer_size = self.capacity;

        loop {
            let raw_ptr = active_ptr.map_or(ptr::null_mut(), |ptr| ptr.as_ptr());

            // SAFETY:
            //
            // `get_memory_map` was obtained from a valid UEFI Boot Services table, which means
            // that it is safe to call, since the arguments provided are as according to the UEFI
            // specification.
            let result = unsafe {
                get_memory_map(
                    &mut buffer_size,
                    raw_ptr.cast::<MemoryDescriptor>(),
                    &mut self.key,
                    &mut self.descriptor_size,
                    &mut self.descriptor_version,
                )
            };

            match result {
                Status::SUCCESS => {
                    self.ptr = active_ptr;
                    self.capacity = buffer_capacity;
                    self.size = buffer_size;
                    return;
                }
                Status::BUFFER_TOO_SMALL => {
                    // Deallocate the old buffer if it exists
                    if let Some(ptr) = active_ptr.take() {
                        // SAFETY:
                        //
                        // The region of memory demarcated by `active_ptr` is no longer in use.
                        unsafe {
                            deallocate(ptr, buffer_capacity, mem::align_of::<MemoryDescriptor>());
                        }
                    }

                    // Compute new buffer size (add some extra space)
                    let total_size = buffer_size
                        .checked_add(
                            2usize
                                .checked_mul(mem::size_of::<MemoryDescriptor>())
                                .unwrap(),
                        )
                        .unwrap();

                    // Allocate new buffer
                    active_ptr =
                        allocate(total_size, mem::align_of::<MemoryDescriptor>()).map(|record| {
                            let ptr = record.ptr_nonnull();
                            mem::forget(record);
                            ptr
                        });
                    buffer_capacity = total_size;
                    buffer_size = total_size;
                }
                result => panic!("memory map update failed: {result:?}"),
            }
        }
    }

    /// Returns an [`Iterator`] over the [`MemoryDescriptor`]s in this [`MemoryMap`].
    pub fn descriptors(&self) -> Iter<'_> {
        Iter {
            map: self,
            offset: 0,
        }
    }
}

impl fmt::Debug for MemoryMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryMap")
            .field("ptr", &self.ptr)
            .field("capacity", &self.capacity)
            .field("size", &self.size)
            .field("key", &self.key)
            .field("descriptor_size", &self.descriptor_size)
            .field("descriptor_version", &self.descriptor_version)
            .field("descriptors", &self.descriptors())
            .finish()
    }
}

impl Drop for MemoryMap {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr.take() {
            // SAFETY: The buffer is exclusively owned by this MemoryMap.
            unsafe {
                deallocate(ptr, self.capacity, mem::align_of::<MemoryDescriptor>());
            }
        }
    }
}

// SAFETY:
//
// [`MemoryMap`] can safely be read from multiple threads.
unsafe impl Send for MemoryMap {}
// SAFETY:
//
// [`MemoryMap`] can safely be sent across threads.
unsafe impl Sync for MemoryMap {}

/// UEFI memory map iterator.
#[derive(Clone)]
struct Iter<'map> {
    /// The [`MemoryMap`] to iterator over.
    map: &'map MemoryMap,
    /// The offset, in bytes, of the next [`MemoryDescriptor`] to emit.
    offset: usize,
}

impl Iterator for Iter<'_> {
    type Item = MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset.checked_add(self.map.descriptor_size)? > self.map.size {
            return None;
        }

        let ptr = self
            .map
            .ptr
            .map(|ptr| ptr.as_ptr())
            .unwrap_or_default()
            .wrapping_byte_add(self.offset);
        self.offset += self.map.descriptor_size;

        // SAFETY:
        //
        // The location read from is within the bounds of the buffer.
        unsafe { Some(ptr.cast::<MemoryDescriptor>().read()) }
    }
}

impl fmt::Debug for Iter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_list = f.debug_list();

        debug_list.entries(self.clone());

        debug_list.finish()
    }
}

/// Iterates though the UEFI Configuration Tables and returns the first table entry with the given
/// [`Guid`].
fn lookup_config_table(
    system_table_ptr: NonNull<SystemTable>,
    guid: Guid,
) -> Option<PhysicalAddress> {
    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let system_table = unsafe { system_table_ptr.as_ref() };

    let config_table_count = system_table.number_of_table_entries;
    let config_tables_ptr = system_table.configuration_table;

    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let config_tables = unsafe { slice::from_raw_parts(config_tables_ptr, config_table_count) };

    for table in config_tables {
        if table.vendor_guid == guid {
            return Some(PhysicalAddress::new(usize_to_u64(
                table.vendor_table.addr(),
            )));
        }
    }

    None
}

/// Wrapper around the UEFI [`SystemTable`] to ensure its [`Sync`] and [`Send`] properties.
#[derive(Clone, Copy)]
struct UefiSystemTable(NonNull<SystemTable>);

// SAFETY:
//
// It is always safe to read a pointer to a [`SystemTable`] across threads.
unsafe impl Sync for UefiSystemTable {}
// SAFETY:
//
// It is always safe to read a pointer to a [`SystemTable`] across threads.
unsafe impl Send for UefiSystemTable {}

/// The UEFI environemnt-specific panic handler.
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    // Acquire and match in two seperate actions to prevent the [`UEFI_SYSTEM_TABLE`] lock from
    // being held for the remainder of the function.
    let system_table_ptr_option = *UEFI_SYSTEM_TABLE.lock();
    let Some(system_table_ptr) = system_table_ptr_option else {
        loop {
            hint::spin_loop()
        }
    };
    let system_table_ptr = system_table_ptr.0;

    // SAFETY:
    //
    // `system_table_ptr` was provided by the `efi_main` entry point.
    let boot_services_ptr = unsafe { system_table_ptr.as_ref().boot_services };
    // SAFETY:
    //
    // `boot_services_ptr` should point to a valid `BootServices1_0` structure, which
    // is guaranteed to contain the `exit` function.
    let exit = unsafe { (*boot_services_ptr).exit };

    // SAFETY:
    //
    // Clean up any allocated frames before tearing down.
    unsafe { deallocate_all_frames() }
    // SAFETY:
    //
    // Clean up any allocated pool memory before tearing down.
    unsafe { deallocate_all() }
    // SAFETY:
    //
    // The only action performed after tearing the [`Platform`] down is returning.
    unsafe { platform_teardown() }

    let image_handle = Handle(IMAGE_HANLDE.load(Ordering::Relaxed));

    // Ignore the result of `exit`. If it returns, it failed but we've already shut everything
    // down.
    // SAFETY:
    //
    // All allocations have been freed and the executable does not open any protocols.
    let _ = unsafe { exit(image_handle, Status::LOAD_ERROR, 0, ptr::null_mut()) };
    loop {
        hint::spin_loop()
    }
}

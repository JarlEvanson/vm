//! Support for booting from an UEFI platform implementation.

use core::{
    alloc::Layout,
    ffi,
    fmt::{self, Write},
    mem,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{AtomicPtr, Ordering},
};

use conversion::{u64_to_usize_strict, usize_to_u64};
use sync::{ControlledModificationCell, Spinlock};
use uefi::{
    data_type::{Boolean, Event, Handle, Status, TaskPriorityLevel},
    protocol::{
        console::simple_text::output::SimpleTextOutputProtocol,
        mp::{MpServicesProtocol, ProcessorInformation, StatusFlag},
    },
    table::{
        boot::{AllocateType, BootServices2_0, EventType},
        config,
        system::SystemTable,
    },
};

use crate::{
    PANIC_HANDLER,
    arch::memory::physical_bits,
    platform::{
        AllocationPolicy, Allocator, BufferTooSmall, Console, Frame, FrameRange, MapError,
        MappingType, MemoryDescriptor, MemoryMap, MemoryType, Metadata, OutOfMemory, Page,
        PageRange, Permissions, PhysicalAddress, PhysicalMemoryManager, Procedure,
        ProcessorManager, VirtualAddress, VirtualAddressRange, VirtualMemoryManager, allocate,
        current_processor_id, deallocate, initialize_allocator, initialize_memory_config,
        initialize_physical_memory_manager, initialize_processor_management,
        initialize_virtual_memory_manager, page_size, register_console, set_device_tree, set_rsdp,
        set_smbios_32, set_smbios_64, set_uefi_system_table, set_xsdp,
    },
};

/// The [`Handle`] representing the image.
static IMAGE_HANDLE: AtomicPtr<ffi::c_void> = AtomicPtr::new(ptr::null_mut());
/// The program's UEFI table.
static UEFI_SYSTEM_TABLE: Spinlock<Option<UefiSystemTable>> = Spinlock::new(None);

/// Implementation of UEFI-specific functionality.
static UEFI_IMPL: ControlledModificationCell<UefiImpl> =
    ControlledModificationCell::new(UefiImpl::new());
/// Implementation of [`Console`] for the UEFI standard output.
static UEFI_CONSOLE: Console = Console::new(write);
/// Stored UEFI memory map.
static MEMORY_MAP: Spinlock<UefiMemoryMap> = Spinlock::new(UefiMemoryMap::new());
/// The active [`MpServicesProtocol`].
static MP_SERVICES: ControlledModificationCell<Option<&'static MpServicesProtocol>> =
    ControlledModificationCell::new(None);

/// Rust entrypoint for the UEFI environment.
pub extern "efiapi" fn uefi_main(
    image_handle: Handle,
    system_table_ptr: *mut SystemTable,
) -> Status {
    IMAGE_HANDLE.store(image_handle.0, Ordering::Relaxed);
    *UEFI_SYSTEM_TABLE.lock() = NonNull::new(system_table_ptr).map(UefiSystemTable);
    *PANIC_HANDLER.lock() = panic_handler;

    // SAFETY:
    //
    // This registration occurs first thing and thus cannot overlap with other printers.
    unsafe { register_console(NonNull::from_ref(&UEFI_CONSOLE)) };

    let (main_processor_id, processor_count) = 'mp: {
        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point and so according to the
        // UEFI specification, the pointer must be valid.
        let boot_services_ptr = unsafe { (*system_table_ptr).boot_services };

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point and so according to the
        // UEFI specification, it must contain a valid UEFI [`BootServices`] table.
        let boot_services_revision = unsafe { (*boot_services_ptr).header.revision };
        if boot_services_revision.major() != 2 {
            crate::warn!(
                "MpServices not found: UEFI implementation is too old for required function"
            );
            break 'mp (0, 1);
        }

        let boot_services_ptr = boot_services_ptr.cast::<BootServices2_0>();

        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices2_0`] table and that must
        // contain a `locate_protocol` function pointer, since the revision check was successful.
        let locate_protcool_ptr = unsafe { (*boot_services_ptr).v1_1.locate_protocol };

        let guid = MpServicesProtocol::GUID;
        let mut interface = ptr::null_mut();
        // SAFETY:
        //
        // The invariants of this function fulfill the invariants of `locate_protcool`.
        let result = unsafe { locate_protcool_ptr(&guid, ptr::null_mut(), &mut interface) };
        if result != Status::SUCCESS {
            crate::warn!("MpServices not found: single-threaded system");
            break 'mp (0, 1);
        }

        let interface = interface.cast::<MpServicesProtocol>();
        // SAFETY:
        //
        // [`MpServicesProtocol`] will be active until `exit_boot_services()` is called.
        let interface_val = unsafe { &*interface };
        // SAFETY:
        //
        // [`MpServicesProtocol`] will be active until `exit_boot_services()` is called.
        unsafe { *MP_SERVICES.get_mut() = Some(interface_val) };

        let mut processor_count = 0;
        let mut enabled_processor_count = 0;
        // SAFETY:
        //
        // The invariants of [`MpServicesProtocol::get_number_of_processors()`] have been fulfilled.
        let result = unsafe {
            (interface_val.get_number_of_processors)(
                interface,
                &mut processor_count,
                &mut enabled_processor_count,
            )
        };
        if result != Status::SUCCESS {
            crate::warn!("MpServices call failed: reverting to single-threaded system");
            break 'mp (0, 1);
        }

        let mut main_processor_id = 0;
        for processor_id in 0..processor_count {
            let mut processor_id_tmp = processor_id;
            let mut processor_info = ProcessorInformation::default();

            // SAFETY:
            //
            // The invariants of [`MpServicesProtocol::get_processor_info()`] have been fulfilled.
            let result = unsafe {
                (interface_val.get_processor_info)(
                    interface,
                    &mut processor_id_tmp,
                    &mut processor_info,
                )
            };
            if result != Status::SUCCESS {
                crate::warn!("MpServices call failed: reverting to single-threaded system");
                break 'mp (0, 1);
            }

            if (processor_info.status_flag.0 & StatusFlag::BSP.0) != 0 {
                main_processor_id = processor_id;
            }
        }

        (main_processor_id, processor_count)
    };

    // SAFETY:
    //
    // This access occurs before any reference to [`UEFI_IMPL`] is created.
    unsafe {
        let uefi_impl = UEFI_IMPL.get_mut();
        uefi_impl.main_processor_id = main_processor_id;
        uefi_impl.processor_count = processor_count;
    }

    // The UEFI specification states that frame sizes are always 4096 bytes, and as an identity
    // mapped platform, so too are page sizes.
    initialize_memory_config(4096, physical_bits(), 4096);

    // SAFETY:
    //
    // All initializations occur before any other calls are made.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_physical_memory_manager(UEFI_IMPL.get());
        initialize_virtual_memory_manager(UEFI_IMPL.get());
        initialize_allocator(UEFI_IMPL.get());
        initialize_processor_management(UEFI_IMPL.get());
    }

    let tables: [(::uefi::data_type::Guid, unsafe fn(PhysicalAddress)); 5] = [
        (config::ACPI, set_rsdp),
        (config::ACPI_2, set_xsdp),
        (config::DEVICE_TREE, set_device_tree),
        (config::SMBIOS, set_smbios_32),
        (config::SMBIOS_3, set_smbios_64),
    ];

    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let system_table = unsafe { &*system_table_ptr };

    let config_table_count = system_table.number_of_table_entries;
    let config_tables_ptr = system_table.configuration_table;

    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let config_tables = unsafe { slice::from_raw_parts(config_tables_ptr, config_table_count) };
    for table in config_tables {
        for (guid, set) in tables {
            if table.vendor_guid == guid {
                // SAFETY:
                //
                // There are zero overlapping calls to the getter and setter functions.
                unsafe {
                    set(PhysicalAddress::new(usize_to_u64(
                        table.vendor_table.addr(),
                    )));
                }
            }
        }
    }

    // SAFETY:
    //
    // There are zero overlapping calls to [`set_uefi_system_table()`] and [`uefi_system_table()`].
    unsafe { set_uefi_system_table(PhysicalAddress::new(usize_to_u64(system_table_ptr.addr()))) }

    crate::debug!("Image Start: {:#x}", crate::util::image_start());
    match crate::stub_main() {
        Ok(()) => Status::SUCCESS,
        Err(error) => {
            crate::warn!("{error}");
            Status::LOAD_ERROR
        }
    }
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

/// Zero-sized implementation of most platform abstractions.
struct UefiImpl {
    /// The ID of the main/bootstrap processor.
    main_processor_id: usize,
    /// The number of processors MP_SERVICES reports as existing on the system.
    processor_count: usize,
}

impl UefiImpl {
    /// Creates a baseline [`UefiImpl`].
    const fn new() -> Self {
        Self {
            main_processor_id: 0,
            processor_count: 1,
        }
    }
}

impl PhysicalMemoryManager for UefiImpl {
    fn allocate_frames(
        &self,
        count: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory> {
        let (allocation_type, mut physical_address) = match policy {
            AllocationPolicy::Any => (AllocateType::ANY_PAGES, 0),
            AllocationPolicy::At(value) => (AllocateType::ADDRESS, value),
            AllocationPolicy::InclusiveMax(value) => (AllocateType::MAX_ADDRESS, value),
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
                ::uefi::memory::MemoryType::LOADER_CODE,
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

    unsafe fn deallocate_frames(&self, range: FrameRange) {
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

        let mut range = range;
        while !range.is_empty() {
            let iter_count = u64_to_usize_strict(range.count().min(usize_to_u64(usize::MAX)));

            // SAFETY:
            //
            // `free_pages_ptr` came from a valid [`BootServices`] table and its arguments are
            // correct according to the UEFI specification.
            let status = unsafe { free_pages_ptr(range.start_address().value(), iter_count) };
            if status.error() {
                crate::warn!("error deallocating frames: {status:?}");
            }

            let new_start = range.start().strict_add(usize_to_u64(iter_count));
            range = FrameRange::new(new_start, range.count() - usize_to_u64(iter_count));
        }
    }

    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [MemoryDescriptor],
    ) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
        use uefi::memory::MemoryType as UefiMemoryType;

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
                UefiMemoryType::CONVENTIONAL => MemoryType::Free,
                UefiMemoryType::LOADER_CODE => MemoryType::BootloaderReclaimable,
                UefiMemoryType::LOADER_DATA => MemoryType::BootloaderReclaimable,
                UefiMemoryType::BOOT_SERVICES_CODE => MemoryType::BootloaderReclaimable,
                UefiMemoryType::BOOT_SERVICES_DATA => MemoryType::BootloaderReclaimable,
                UefiMemoryType::UNUSABLE => MemoryType::Bad,
                UefiMemoryType::ACPI_RECLAIM => MemoryType::AcpiReclaimable,
                UefiMemoryType::ACPI_NVS => MemoryType::AcpiNonVolatile,
                _ => MemoryType::Reserved,
            };

            let start = descriptor.physical_start;
            let length = descriptor.number_of_pages;
            let frame_range = FrameRange::new(
                Frame::containing_address(PhysicalAddress::new(start)),
                length,
            );

            buffer[index] = MemoryDescriptor {
                range: frame_range.address_range(),
                region_type,
            };
        }

        Ok(MemoryMap::new(buffer, usize_to_u64(memory_map.key)))
    }
}

impl VirtualMemoryManager for UefiImpl {
    fn max_physical_address(&self) -> PhysicalAddress {
        let bits = {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            match x86::paging::current_paging_mode() {
                x86::paging::PagingMode::Disabled => 32,
                x86::paging::PagingMode::Bits32 => 32,
                x86::paging::PagingMode::Pae => 52,
                x86::paging::PagingMode::Level4 => 52,
                x86::paging::PagingMode::Level5 => 52,
            }
            #[cfg(target_arch = "aarch64")]
            48 // TODO: Fix hardcoded address.
        };

        PhysicalAddress::new(1u64.strict_shl(bits))
    }

    fn map(
        &self,
        frames: FrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<PageRange, MapError> {
        assert_eq!(mapping_type, MappingType::Normal);
        self.map_identity(frames, permissions)
    }

    fn map_identity(&self, frames: FrameRange, _: Permissions) -> Result<PageRange, MapError> {
        assert!(!frames.is_empty(), "mapping zero-sized region is illegal");

        let base = frames.start_address().value();
        let size = frames.byte_count();
        let (Ok(base), Ok(size)) = (usize::try_from(base), usize::try_from(size)) else {
            return Err(MapError::FindFreeRegionError);
        };

        let base = VirtualAddress::new(base);
        let virtual_range = VirtualAddressRange::new(base, base.strict_add(size.saturating_sub(1)));
        let start_page = Page::containing_address(virtual_range.start());
        let end_page = Page::containing_address(virtual_range.end_inclusive());
        Ok(PageRange::new(start_page, end_page))
    }

    fn map_temporary(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
        let offset = u64_to_usize_strict(address.value() % usize_to_u64(page_size()));

        self.map_identity(
            FrameRange::new(Frame::containing_address(address), 1),
            Permissions::ReadWrite,
        )
        .map(|range| range.start_address().strict_add(offset))
        .ok()
    }

    fn translate_virtual(
        &self,
        address: VirtualAddress,
    ) -> Option<(Permissions, MappingType, PhysicalAddress)> {
        Some((
            Permissions::ReadWriteExecute,
            MappingType::Normal,
            PhysicalAddress::new(address.value() as u64),
        ))
    }

    unsafe fn unmap(&self, _: PageRange) {
        // There is no need to unmap anything since the system is identity mapped.
    }
}

impl Allocator for UefiImpl {
    fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
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
        if layout.align() <= 8 {
            let mut ptr = ptr::null_mut();

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `allocate_pool`.
            let result = unsafe {
                allocate_pool_ptr(
                    ::uefi::memory::MemoryType::LOADER_DATA,
                    layout.size(),
                    &mut ptr,
                )
            };
            match result {
                Status::SUCCESS => NonNull::new(ptr.cast::<u8>()),
                Status::OUT_OF_RESOURCES => None,
                status => panic!("error allocating memory: {status:?}"),
            }
        } else {
            todo!("implement alignment greater than 8 bytes")
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
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
        if layout.align() <= 8 {
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
}

impl ProcessorManager for UefiImpl {
    fn main_processor_id(&self) -> u64 {
        usize_to_u64(self.main_processor_id)
    }

    fn current_processor_id(&self) -> u64 {
        let Some(mp_services) = MP_SERVICES.get() else {
            return 0;
        };

        let mut proc_number = 0;
        // SAFETY:
        //
        // The invariants of [`MpServicesProtocol::who_am_i()`] have been fulfilled.
        let result = unsafe {
            (mp_services.who_am_i)(ptr::from_ref(*mp_services).cast_mut(), &mut proc_number)
        };
        if result == Status::SUCCESS {
            usize_to_u64(proc_number)
        } else {
            0
        }
    }

    fn processor_count(&self) -> u64 {
        usize_to_u64(self.processor_count)
    }

    fn run_on_all_processors(&self, procedure: Procedure, argument: *mut ()) {
        if self.processor_count() == 1 {
            procedure(self.main_processor_id(), argument);
            return;
        }

        let Some(mp_services) = MP_SERVICES.get() else {
            procedure(self.main_processor_id(), argument);
            return;
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
        // `create_event` function pointer.
        let create_event_ptr = unsafe { (*boot_services_ptr).create_event };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table that must contain a
        // `wait_for_event` function pointer.
        let wait_for_event = unsafe { (*boot_services_ptr).wait_for_event };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `close_event` function pointer.
        let close_event_ptr = unsafe { (*boot_services_ptr).close_event };

        let mut event = Event(ptr::null_mut());
        // SAFETY:
        //
        // `create_event` was called correctly and has valid pointers.
        let result = unsafe {
            create_event_ptr(
                EventType(0),
                TaskPriorityLevel::CALLBACK,
                None,
                ptr::null_mut(),
                &mut event,
            )
        };
        assert_eq!(result, Status::SUCCESS);

        unsafe extern "efiapi" fn interpose(arg: *mut ffi::c_void) {
            // SAFETY:
            //
            // The format of the `arg` is as specified.
            let (func, arg) = unsafe { *arg.cast_const().cast::<(super::Procedure, *mut ())>() };

            func(current_processor_id(), arg)
        }

        let storage = (procedure, argument);
        let result = loop {
            // SAFETY:
            //
            // The invariants of [`MpServicesProtocol::startup_all_aps()`] have been fulfilled.
            let result = unsafe {
                (mp_services.startup_all_aps)(
                    ptr::from_ref(*mp_services).cast_mut(),
                    interpose,
                    Boolean::FALSE,
                    event,
                    0,
                    ptr::from_ref(&storage).cast_mut().cast::<ffi::c_void>(),
                    ptr::null_mut(),
                )
            };
            if result != Status::NOT_READY {
                break result;
            }
        };
        assert_eq!(result, Status::SUCCESS);

        procedure(self.main_processor_id(), argument);

        let mut index = 0;
        // SAFETY:
        //
        // `wait_for_event` was obtained from a valid [`BootServices`] table and that table has not
        // been exited.
        let result = unsafe { wait_for_event(1, &mut event, &mut index) };
        assert_eq!(result, Status::SUCCESS);

        // SAFETY:
        //
        // Force `storage` to live at least this long.
        unsafe { core::arch::asm!("/* {0} */", in(reg) &storage) }

        // SAFETY:
        //
        // `close_event` was obtained from a valid [`BootServices`] table and that table has not
        // been exited yet.
        let result = unsafe { close_event_ptr(event) };
        assert_eq!(result, Status::SUCCESS);
    }
}

/// Implementation of [`Console::write`] for the UEFI standard output.
fn write(_: NonNull<Console>, metadata: Metadata, message: &str) {
    const BUFFER_SIZE: usize = 128;

    struct Printer(*mut SimpleTextOutputProtocol);

    impl fmt::Write for Printer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // SAFETY:
            //
            // `con_out` should be a valid SimpleTextOutputProcotol structure, which is guaranteed to
            // contain the `output_string` function.
            let output_string_func = unsafe { (*self.0).output_string };

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
                    let _ = unsafe { output_string_func(self.0, string.as_mut_ptr()) };
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
                let _ = unsafe { output_string_func(self.0, string.as_mut_ptr()) };
            }

            Ok(())
        }
    }

    let uefi_system_table = UEFI_SYSTEM_TABLE.lock();
    let system_table_ptr = uefi_system_table
        .expect("illegal call of `allocate_frames()`")
        .0;

    // SAFETY:
    //
    // `system_table_ptr` was provided by the `efi_main` entry point.
    let con_out = unsafe { system_table_ptr.as_ref().con_out };
    if con_out.is_null() {
        return;
    }

    let _ = write!(Printer(con_out), "[{:?}]: {message}", metadata.level);

    drop(uefi_system_table);
}

/// Wrapper around simple updates of the UEFI memory map.
struct UefiMemoryMap {
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

impl UefiMemoryMap {
    /// Returns an empty [`MemoryMap`].
    pub const fn new() -> Self {
        Self {
            ptr: None,
            capacity: 0,
            size: 0,
            key: 0,
            descriptor_size: mem::size_of::<::uefi::memory::MemoryDescriptor>(),
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

        let entry_align = mem::align_of::<::uefi::memory::MemoryDescriptor>();
        let additional_entry_size = 2usize.saturating_mul(self.descriptor_size);

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
                    raw_ptr.cast::<::uefi::memory::MemoryDescriptor>(),
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
                        let size = buffer_capacity;
                        let Ok(layout) = Layout::from_size_align(size, entry_align) else {
                            unreachable!()
                        };

                        // SAFETY:
                        //
                        // The region of memory demarcated by `active_ptr` is no longer in use.
                        unsafe {
                            deallocate(ptr, layout);
                        }
                    }

                    // Compute new buffer size (add some extra space)
                    let total_size = buffer_size.saturating_add(additional_entry_size);
                    let Ok(layout) = Layout::from_size_align(total_size, entry_align) else {
                        panic!("required UEFI memory map buffer size is too large");
                    };

                    // Allocate new buffer
                    active_ptr = allocate(layout);
                    if active_ptr.is_none() {
                        panic!(
                            "UEFI memory map buffer allocation failed: {} bytes",
                            layout.size()
                        );
                    }

                    buffer_capacity = layout.size();
                    buffer_size = layout.size();
                }
                result => panic!("memory map update failed: {result:?}"),
            }
        }
    }

    /// Returns an [`Iterator`] over the [`MemoryDescriptor`][md]s in this [`MemoryMap`].
    ///
    /// [md]: ::uefi::memory::MemoryDescriptor
    pub fn descriptors(&self) -> Iter<'_> {
        Iter {
            map: self,
            offset: 0,
        }
    }
}

impl fmt::Debug for UefiMemoryMap {
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

impl Drop for UefiMemoryMap {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr.take() {
            let size = self.capacity;
            let entry_align = mem::align_of::<::uefi::memory::MemoryDescriptor>();
            let Ok(layout) = Layout::from_size_align(size, entry_align) else {
                unreachable!()
            };

            // SAFETY: The buffer is exclusively owned by this MemoryMap.
            unsafe { deallocate(ptr, layout) }
        }
    }
}

// SAFETY:
//
// [`MemoryMap`] can safely be read from multiple threads.
unsafe impl Send for UefiMemoryMap {}
// SAFETY:
//
// [`MemoryMap`] can safely be sent across threads.
unsafe impl Sync for UefiMemoryMap {}

/// UEFI memory map iterator.
#[derive(Clone)]
struct Iter<'map> {
    /// The [`UefiMemoryMap`] to iterator over.
    map: &'map UefiMemoryMap,
    /// The offset, in bytes, of the next [`MemoryDescriptor`] to emit.
    offset: usize,
}

impl Iterator for Iter<'_> {
    type Item = ::uefi::memory::MemoryDescriptor;

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
        unsafe { Some(ptr.cast::<::uefi::memory::MemoryDescriptor>().read()) }
    }
}

impl fmt::Debug for Iter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_list = f.debug_list();

        debug_list.entries(self.clone());

        debug_list.finish()
    }
}

/// UEFI-specific panic handler.
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    // Acquire and match in two seperate actions to prevent the [`UEFI_SYSTEM_TABLE`] lock from
    // being held for the remainder of the function.
    let system_table_ptr_option = *UEFI_SYSTEM_TABLE.lock();
    let Some(system_table_ptr) = system_table_ptr_option else {
        loop {
            core::hint::spin_loop()
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

    let image_handle = Handle(IMAGE_HANDLE.load(Ordering::Relaxed));

    for _ in 0..3 {
        // SAFETY:
        //
        // The executable does not open any protocols.
        let result = unsafe { exit(image_handle, Status::LOAD_ERROR, 0, ptr::null_mut()) };
        crate::error!("error exiting executable: {result:?}");
    }

    loop {
        core::hint::spin_loop();
    }
}

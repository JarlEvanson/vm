//! Support for booting from the Limine boot protocol.

use core::{
    alloc::Layout,
    fmt::Write,
    ptr::{self, NonNull},
    slice,
    sync::atomic::Ordering,
};

use conversion::{u64_to_usize, u64_to_usize_strict, usize_to_u64};
#[cfg(target_arch = "aarch64")]
use limine::mp::aarch64::{MpInfo, MpResponse};
#[cfg(target_arch = "x86_64")]
use limine::mp::x86_64::{MpInfo, MpResponse};
use limine::{
    BaseRevisionTag, RequestsEndMarker, RequestsStartMarker,
    device_tree::{DEVICE_TREE_REQUEST_MAGIC, DeviceTreeRequest},
    efi_sys_table::{EFI_SYSTEM_TABLE_REQUEST_MAGIC, EfiSystemTableRequest},
    executable_addr::{EXECUTABLE_ADDRESS_REQUEST_MAGIC, ExecutableAddressRequest},
    framebuffer::{FRAMEBUFFER_REQUEST_MAGIC, FramebufferRequest, FramebufferV0},
    hhdm::{HHDM_REQUEST_MAGIC, HhdmRequest},
    memory_map::{MEMORY_MAP_REQUEST_MAGIC, MemoryMapEntry, MemoryMapRequest, MemoryType},
    mp::{MP_REQUEST_MAGIC, MpRequest, MpRequestFlags},
    rsdp::{RSDP_REQUEST_MAGIC, RsdpRequest},
    smbios::{SMBIOS_REQUEST_MAGIC, SmbiosRequest},
};
use sync::{ControlledModificationCell, Spinlock};

use crate::{
    PANIC_HANDLER,
    arch::{
        generic::memory::paging::{
            ExternalFrame, ExternalFrameRange, ExternalPage, ExternalPageRange,
            ExternalPhysicalAddress, ExternalVirtualAddress, SearchStrategy, TranslationScheme,
        },
        memory::{ArchTranslationScheme, physical_bits},
    },
    platform::{
        AllocationPolicy, Allocator, BufferTooSmall, FrameRange, MapError, MappingType,
        MemoryDescriptor, MemoryMap, OutOfMemory, Page, PageRange, Permissions, PhysicalAddress,
        PhysicalAddressRange, PhysicalMemoryManager, Procedure, ProcessorManager, VirtualAddress,
        VirtualMemoryManager, frame_size,
        graphics::{
            console::TextConsole,
            font::{FONT_MAP, GLYPH_ARRAY},
        },
        initialize_allocator, initialize_memory_config, initialize_physical_memory_manager,
        initialize_processor_management, initialize_virtual_memory_manager,
        limine::graphics::{
            create_surface, initialize_primary_framebuffer, primary_framebuffer_initialized,
        },
        page_size, set_device_tree, set_rsdp, set_smbios_32, set_smbios_64, set_uefi_system_table,
        set_xsdp,
    },
};

mod graphics;

/// Indicates the start of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.start")]
static REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

/// Tag used to communicate the information regarding the base revision of the Limine protocol.
#[used]
#[unsafe(link_section = ".limine.base_tag")]
static BASE_REVISION_TAG: ControlledModificationCell<BaseRevisionTag> =
    ControlledModificationCell::new(BaseRevisionTag::new_current());

/// Request for the memory map to be provided to the application.
#[used]
#[unsafe(link_section = ".limine.requests")]
static MEMORY_MAP_REQUEST: ControlledModificationCell<MemoryMapRequest> =
    ControlledModificationCell::new(MemoryMapRequest {
        id: MEMORY_MAP_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the higher half direct map offset.
#[used]
#[unsafe(link_section = ".limine.requests")]
static HHDM_REQUEST: ControlledModificationCell<HhdmRequest> =
    ControlledModificationCell::new(HhdmRequest {
        id: HHDM_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address (both virtual and physical) of the executable.
#[used]
#[unsafe(link_section = ".limine.requests")]
static EXECUTABLE_ADDRESS_REQUEST: ControlledModificationCell<ExecutableAddressRequest> =
    ControlledModificationCell::new(ExecutableAddressRequest {
        id: EXECUTABLE_ADDRESS_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the other processors to be initialized and waiting on a spinloop.
#[used]
#[unsafe(link_section = ".limine.requests")]
static MP_REQUEST: ControlledModificationCell<MpRequest> =
    ControlledModificationCell::new(MpRequest {
        id: MP_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
        flags: MpRequestFlags::DEFAULT,
    });

/// Request for the system framebuffers
#[used]
#[unsafe(link_section = ".limine.requests")]
static FRAMEBUFFER_REQUEST: ControlledModificationCell<FramebufferRequest> =
    ControlledModificationCell::new(FramebufferRequest {
        id: FRAMEBUFFER_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the UEFI system table.
#[used]
#[unsafe(link_section = ".limine.requests")]
static UEFI_SYSTEM_TABLE_REQUEST: ControlledModificationCell<EfiSystemTableRequest> =
    ControlledModificationCell::new(EfiSystemTableRequest {
        id: EFI_SYSTEM_TABLE_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the RSDP table.
#[used]
#[unsafe(link_section = ".limine.requests")]
static RSDP_REQUEST: ControlledModificationCell<RsdpRequest> =
    ControlledModificationCell::new(RsdpRequest {
        id: RSDP_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the device tree.
#[used]
#[unsafe(link_section = ".limine.requests")]
static DEVICE_TREE_REQUEST: ControlledModificationCell<DeviceTreeRequest> =
    ControlledModificationCell::new(DeviceTreeRequest {
        id: DEVICE_TREE_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the addresses of the SMBIOS tables.
#[used]
#[unsafe(link_section = ".limine.requests")]
static SMBIOS_REQUEST: ControlledModificationCell<SmbiosRequest> =
    ControlledModificationCell::new(SmbiosRequest {
        id: SMBIOS_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

/// The [`MemoryMapEntry`]s provided by [`MEMORY_MAP_REQUEST`].
static MEMORY_MAP_ENTRIES: ControlledModificationCell<&[&MemoryMapEntry]> =
    ControlledModificationCell::new(&[]);
/// The offset of the Higher Half Direct Map.
static HHDM_OFFSET: ControlledModificationCell<u64> = ControlledModificationCell::new(0);
/// The executable's virtual base address.
static EXECUTABLE_VIRTUAL_BASE: ControlledModificationCell<u64> =
    ControlledModificationCell::new(0);
/// The executable's physical base address.
static EXECUTABLE_PHYSICAL_BASE: ControlledModificationCell<u64> =
    ControlledModificationCell::new(0);
/// The [`MpInfo`] structures representing all CPUs available on the system.
static CPUS: ControlledModificationCell<&[&MpInfo]> = ControlledModificationCell::new(&[]);
/// The [`MpInfo`] associated with the primary processor.
static BSP: ControlledModificationCell<Option<&MpInfo>> = ControlledModificationCell::new(None);
/// The [`FramebufferV0`]s provided by [`FRAMEBUFFER_REQUEST`].
static FRAMEBUFFERS: ControlledModificationCell<&[&FramebufferV0]> =
    ControlledModificationCell::new(&[]);

/// The [`TranslationScheme`] used to implement page mapping and unmapping.
static TRANSLATION_SCHEME: Spinlock<Option<ArchTranslationScheme>> = Spinlock::new(None);

/// Rust entrypoint for the Limine boot protocol.
pub extern "C" fn limine_main() -> ! {
    *PANIC_HANDLER.lock() = panic_handler;

    let (
        memory_map_entries,
        hhdm_offset,
        executable_physical_base,
        executable_virtual_base,
        framebuffers,
        cpu_info_buffer,
    ) = validate_required_tables();

    // SAFETY:
    //
    // These operations occur before any other accesses to the variables occur.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        *MEMORY_MAP_ENTRIES.get_mut() = memory_map_entries;
        *HHDM_OFFSET.get_mut() = hhdm_offset;
        *EXECUTABLE_VIRTUAL_BASE.get_mut() = executable_virtual_base;
        *EXECUTABLE_PHYSICAL_BASE.get_mut() = executable_physical_base;
        *FRAMEBUFFERS.get_mut() = framebuffers;
        *CPUS.get_mut() = cpu_info_buffer;
    }

    // SAFETY:
    //
    // This function occurs before processor bring-up.
    unsafe { initialize_primary_framebuffer() }

    let mut address_impl = TRANSLATION_SCHEME.lock();

    let chunk_size = {
        // SAFETY:
        //
        // This takeover of [`ArchAddressSpace`] is only performed once and this program has exclusive
        // control over the system.
        let scheme = unsafe {
            ArchTranslationScheme::active_current().expect("failed to initialize address space")
        };

        let chunk_size = scheme.chunk_size();
        *address_impl = Some(scheme);
        chunk_size
    };

    drop(address_impl);

    initialize_memory_config(chunk_size, physical_bits(), u64_to_usize_strict(chunk_size));

    // SAFETY:
    //
    // All initializations occur before any other calls are made.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_physical_memory_manager(&LimineImpl);
        initialize_virtual_memory_manager(&LimineImpl);
        initialize_allocator(&LimineImpl);
        initialize_processor_management(&LimineImpl);
    }

    crate::platform::frame_allocator::initialize(memory_map_entries.iter().map(|entry| {
        let start = PhysicalAddress::new(entry.base);
        let range = PhysicalAddressRange::new(start, entry.length);

        let region_type = match entry.mem_type {
            MemoryType::RESERVED => crate::platform::MemoryType::Reserved,
            MemoryType::USABLE => crate::platform::MemoryType::Free,
            MemoryType::BOOTLOADER_RECLAIMABLE => {
                crate::platform::MemoryType::BootloaderReclaimable
            }
            MemoryType::EXECUTABLE_AND_MODULES => {
                crate::platform::MemoryType::BootloaderReclaimable
            }
            MemoryType::BAD_MEMORY => crate::platform::MemoryType::Bad,
            MemoryType::ACPI_RECLAIMABLE => crate::platform::MemoryType::AcpiReclaimable,
            MemoryType::ACPI_NVS => crate::platform::MemoryType::AcpiNonVolatile,
            MemoryType::RESERVED_MAPPED => crate::platform::MemoryType::Reserved,
            _ => crate::platform::MemoryType::Reserved,
        };
        MemoryDescriptor { range, region_type }
    }));

    if !cpu_info_buffer.is_empty() {
        for (index, cpu_info) in cpu_info_buffer.iter().enumerate() {
            cpu_info
                .extra_argument
                .store(usize_to_u64(index), Ordering::Relaxed);
            cpu_info.goto_address.store(
                usize_to_u64(ap_loop as *const () as usize),
                Ordering::Relaxed,
            );
        }

        loop {
            let mut all = 0;
            for cpu in CPUS.get().iter() {
                if cpu.goto_address.load(Ordering::Acquire) == 0 {
                    all += 1;
                }
            }

            if all + 1 == CPUS.get().len() {
                break;
            }
        }

        'proc_loop: {
            for cpu in CPUS.get().iter() {
                if cpu.goto_address.load(Ordering::Acquire) != 0 {
                    // SAFETY:
                    //
                    // No calls to [`run_on_all_processors`] have been made.
                    unsafe { *BSP.get_mut() = Some(*cpu) }
                    let proc_id = cpu.extra_argument.swap(0, Ordering::AcqRel);

                    cpu.goto_address.store(proc_id, Ordering::Release);
                    break 'proc_loop;
                }
            }

            unreachable!()
        }
    }

    'uefi_system_table: {
        let uefi_system_table_response_ptr = UEFI_SYSTEM_TABLE_REQUEST.get().response;

        // SAFETY:
        //
        // The Limine bootloader specification states that if the response pointer has changed (and it
        // has if it isn't NULL), then the uefi system table response is valid.
        let Some(uefi_system_table_response) = (unsafe { uefi_system_table_response_ptr.as_ref() })
        else {
            break 'uefi_system_table;
        };

        // SAFETY:
        //
        // No other cores are active at this time and thus no calls to [`set_uefi_system_table()`] or
        // [`uefi_system_table()`] can overlap.
        unsafe {
            set_uefi_system_table(PhysicalAddress::new(
                uefi_system_table_response.address - HHDM_OFFSET.get(),
            ))
        }
    }

    'rsdp: {
        let rsdp_response_ptr = RSDP_REQUEST.get().response;

        // SAFETY:
        //
        // The Limine bootloader specification states that if the response pointer has changed (and it
        // has if it isn't NULL), then the rsdp table response is valid.
        let Some(rsdp_response) = (unsafe { rsdp_response_ptr.as_ref() }) else {
            break 'rsdp;
        };

        let address = PhysicalAddress::new(rsdp_response.address as u64 - HHDM_OFFSET.get());
        // SAFETY:
        //
        // No other cores are active at this time and thus no calls to [`set_rsdp()`],
        // [`set_xsdp()`], [`rsdp()`], or [`xsdp()`] can overlap.
        #[expect(clippy::multiple_unsafe_ops_per_block)]
        unsafe {
            set_rsdp(address);
            set_xsdp(address);
        }
    }

    'device_tree: {
        let device_tree_response_ptr = DEVICE_TREE_REQUEST.get().response;

        // SAFETY:
        //
        // The Limine bootloader specification states that if the response pointer has changed (and it
        // has if it isn't NULL), then the device tree response is valid.
        let Some(device_tree_response) = (unsafe { device_tree_response_ptr.as_ref() }) else {
            break 'device_tree;
        };

        // SAFETY:
        //
        // No other cores are active at this time and thus no calls to [`set_device_tree()`] or
        // [`device_tree()`] can overlap.
        unsafe {
            set_device_tree(PhysicalAddress::new(
                device_tree_response.dtb_ptr as u64 - HHDM_OFFSET.get(),
            ))
        }
    }

    'smbios: {
        let smbios_response_ptr = SMBIOS_REQUEST.get().response;

        // SAFETY:
        //
        // The Limine bootloader specification states that if the response pointer has changed (and it
        // has if it isn't NULL), then the SMBIOS table response is valid.
        let Some(smbios_response) = (unsafe { smbios_response_ptr.as_ref() }) else {
            break 'smbios;
        };

        // SAFETY:
        //
        // No other cores are active at this time and thus no calls to [`set_smbios_32()`],
        // [`set_smbios_64()`], [`smbios_32()`], or [`smbios_64()`] can overlap.
        #[expect(clippy::multiple_unsafe_ops_per_block)]
        unsafe {
            set_smbios_32(PhysicalAddress::new(
                (smbios_response.entry_32 as u64).saturating_sub(*HHDM_OFFSET.get()),
            ));
            set_smbios_64(PhysicalAddress::new(
                (smbios_response.entry_64 as u64).saturating_sub(*HHDM_OFFSET.get()),
            ));
        }
    }

    crate::debug!("Image Start: {:#x}", crate::util::image_start());
    match crate::stub_main() {
        Ok(()) => {}
        Err(error) => crate::error!("{error}"),
    };

    loop {
        core::hint::spin_loop()
    }
}

/// Zero-sized implementation of most platform abstractions.
struct LimineImpl;

impl PhysicalMemoryManager for LimineImpl {
    fn allocate_frames(
        &self,
        count: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory> {
        crate::platform::frame_allocator::allocate_frames(count, policy)
    }

    unsafe fn deallocate_frames(&self, range: FrameRange) {
        // SAFETY:
        //
        // The invariants of [`PhysicalMemoryManager::deallocate_frames()`] fulfill the invariants
        // of [`deallocate_frames()`].
        unsafe { crate::platform::frame_allocator::deallocate_frames(range) }
    }

    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [MemoryDescriptor],
    ) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
        crate::platform::frame_allocator::memory_map(buffer)
    }
}

impl VirtualMemoryManager for LimineImpl {
    fn max_physical_address(&self) -> PhysicalAddress {
        let max_physical_address = TRANSLATION_SCHEME
            .lock()
            .as_ref()
            .expect("virtual memory management initialization failed")
            .output_descriptor()
            .valid_ranges()
            .into_iter()
            .filter(|&(start, end)| start <= end)
            .map(|(_, end)| end)
            .max()
            .expect("virtual memory management initialization failed");
        PhysicalAddress::new(max_physical_address)
    }

    fn map(
        &self,
        frames: FrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<PageRange, MapError> {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let frame_count = if usize_to_u64(page_size()) >= frame_size() {
            frames
                .count()
                .div_ceil(usize_to_u64(page_size()) / frame_size())
        } else {
            frames
                .count()
                .strict_mul(frame_size() / usize_to_u64(page_size()))
        };

        let frames = ExternalFrameRange::new(
            ExternalFrame::containing_address(
                ExternalPhysicalAddress::new(frames.start_address().value()),
                scheme.chunk_size(),
            ),
            frame_count,
        );
        let pages = scheme.map(SearchStrategy::TopDown, frames, permissions, mapping_type)?;

        let range = PageRange::new(
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.start_address(scheme.chunk_size()).value(),
            ))),
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.end_address_inclusive(scheme.chunk_size()).value(),
            ))),
        );

        #[cfg(target_arch = "x86_64")]
        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }

        Ok(range)
    }

    fn map_identity(
        &self,
        frames: FrameRange,
        permissions: Permissions,
    ) -> Result<PageRange, MapError> {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let frame_count = if usize_to_u64(page_size()) >= frame_size() {
            frames
                .count()
                .div_ceil(usize_to_u64(page_size()) / frame_size())
        } else {
            frames
                .count()
                .strict_mul(frame_size() / usize_to_u64(page_size()))
        };

        let frames = ExternalFrameRange::new(
            ExternalFrame::containing_address(
                ExternalPhysicalAddress::new(frames.start_address().value()),
                scheme.chunk_size(),
            ),
            frame_count,
        );
        let pages = scheme.map_identity(frames, permissions)?;

        let range = PageRange::new(
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.start_address(scheme.chunk_size()).value(),
            ))),
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.end_address_inclusive(scheme.chunk_size()).value(),
            ))),
        );

        #[cfg(target_arch = "x86_64")]
        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }

        Ok(range)
    }

    fn map_temporary(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
        let hhdm_offset = *HHDM_OFFSET.get();
        for entry in MEMORY_MAP_ENTRIES.get().iter() {
            match entry.mem_type {
                MemoryType::USABLE
                | MemoryType::BOOTLOADER_RECLAIMABLE
                | MemoryType::EXECUTABLE_AND_MODULES
                | MemoryType::FRAMEBUFFER
                | MemoryType::RESERVED_MAPPED
                | MemoryType::ACPI_RECLAIMABLE
                | MemoryType::ACPI_NVS => {}
                _ => continue,
            }

            let range = PhysicalAddressRange::new(PhysicalAddress::new(entry.base), entry.length);
            if range.contains(address) {
                let raw_address = address.value().strict_add(hhdm_offset);
                let virt_address = VirtualAddress::new(u64_to_usize(raw_address));
                return Some(virt_address);
            }
        }

        todo!("implement arbitary memory mapping")
    }

    fn translate_virtual(
        &self,
        address: VirtualAddress,
    ) -> Option<(Permissions, MappingType, PhysicalAddress)> {
        let virtual_address_u64 = usize_to_u64(address.value());

        let hhdm_offset = *HHDM_OFFSET.get();
        for entry in MEMORY_MAP_ENTRIES.get().iter() {
            match entry.mem_type {
                MemoryType::USABLE
                | MemoryType::BOOTLOADER_RECLAIMABLE
                | MemoryType::EXECUTABLE_AND_MODULES
                | MemoryType::FRAMEBUFFER
                | MemoryType::RESERVED_MAPPED
                | MemoryType::ACPI_RECLAIMABLE
                | MemoryType::ACPI_NVS => {}
                _ => continue,
            }

            let entry_virtual_start = entry.base.strict_add(hhdm_offset);
            let entry_virtual_end = entry_virtual_start.strict_add(entry.length);
            if entry_virtual_start <= virtual_address_u64 && virtual_address_u64 < entry_virtual_end
            {
                return Some((
                    Permissions::ReadWrite,
                    MappingType::Normal,
                    PhysicalAddress::new(virtual_address_u64.strict_sub(hhdm_offset)),
                ));
            }
        }

        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        scheme
            .translate(ExternalVirtualAddress::new(virtual_address_u64))
            .map(|(permissions, mapping_type, address)| {
                (
                    permissions,
                    mapping_type,
                    PhysicalAddress::new(address.value()),
                )
            })
    }

    unsafe fn unmap(&self, range: PageRange) {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let input = ExternalPageRange::new(
            ExternalPage::containing_address(
                ExternalVirtualAddress::new(usize_to_u64(range.start_address().value())),
                scheme.chunk_size(),
            ),
            ExternalPage::containing_address(
                ExternalVirtualAddress::new(usize_to_u64(range.end_address_inclusive().value())),
                scheme.chunk_size(),
            ),
        );

        // SAFETY:
        //
        // The invariants of [`VirtualMemoryManager::unmap()`] fulfill the invariants of
        // [`ArchTranslationScheme::unmap()`].
        unsafe { scheme.unmap(input) }

        #[cfg(target_arch = "x86_64")]
        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }
    }
}

impl Allocator for LimineImpl {
    fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
        crate::platform::heap_allocator::allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // SAFETY:
        //
        // The invariants of [`Allocator::deallocate()`] fulfill the invariants of
        // [`deallocate()`].
        unsafe { crate::platform::heap_allocator::deallocate(ptr, layout) }
    }
}

impl ProcessorManager for LimineImpl {
    fn main_processor_id(&self) -> u64 {
        let proc = BSP.get().expect("initialization failure");
        proc.goto_address.load(Ordering::Relaxed)
    }

    fn current_processor_id(&self) -> u64 {
        todo!()
    }

    fn processor_count(&self) -> u64 {
        usize_to_u64(CPUS.get().len())
    }

    fn run_on_all_processors(&self, procedure: Procedure, argument: *mut ()) {
        if CPUS.get().len() > 1 {
            let proc = BSP.get().expect("failure in initialization");
            let proc_id = self.main_processor_id();

            let func_description = (procedure, argument);
            {
                for cpu in CPUS.get().iter() {
                    assert_eq!(
                        cpu.extra_argument.load(Ordering::Acquire),
                        0,
                        "all CPUs have not finished"
                    );
                }
                for cpu in CPUS.get().iter() {
                    cpu.extra_argument.store(
                        usize_to_u64(ptr::from_ref(&func_description).addr()),
                        Ordering::Release,
                    )
                }

                procedure(proc_id, argument);
                proc.extra_argument.store(0, Ordering::Release);

                loop {
                    let mut all = 0;
                    for cpu in CPUS.get().iter() {
                        if cpu.extra_argument.load(Ordering::Acquire) == 0 {
                            all += 1;
                        }
                    }

                    if all == CPUS.get().len() {
                        break;
                    }
                }
            }
            core::hint::black_box(func_description);
        } else {
            // Single-threaded.
            procedure(0, argument)
        }
    }
}

/// Validates that the required Limine requests have been fulfilled and returns the contents of
/// those responses.
fn validate_required_tables() -> (
    &'static [&'static MemoryMapEntry],
    u64,
    u64,
    u64,
    &'static [&'static FramebufferV0],
    &'static [&'static MpInfo],
) {
    let base_revision_tag = BASE_REVISION_TAG.get();
    if !base_revision_tag.is_supported() {
        // If the base revision this executable was loaded using is greater than or equal to 3,
        // then [`BaseRevisionTag::loaded_revision`] contains the base revision used to load the
        // executable. Otherwise, the base revision must be either 0, 1, or 2.
        if let Some(loaded_revision) = base_revision_tag.loaded_revision() {
            panic!("Loaded using unsupported base revision {loaded_revision}",)
        }

        panic!("Loaded using unsupported base revision (possible revisions are 0, 1, and 2)")
    }

    let memory_map_response_ptr = MEMORY_MAP_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`MEMORY_MAP_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `memory_map_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer.
    let Some(memory_map_response) = (unsafe { memory_map_response_ptr.as_ref() }) else {
        panic!("Limine memory map was not provided");
    };

    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`MEMORY_MAP_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `memory_map_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer. Moreover, the
    // request must have been fulfilled according to the specification and as such, this slice is
    // valid.
    let memory_map_entries = unsafe {
        slice::from_raw_parts(
            memory_map_response.entries.cast::<&MemoryMapEntry>(),
            u64_to_usize(memory_map_response.entry_count),
        )
    };

    let hhdm_response_ptr = HHDM_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`HHDM_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `hhdm_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer.
    let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
        panic!("Limine higher half direct map was not provided");
    };

    let executable_address_response_ptr = EXECUTABLE_ADDRESS_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`EXECUTABLE_ADDRESS_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `executable_address_response_ptr` was
    // initialized to `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid
    // pointer.
    let Some(executable_address_response) = (unsafe { executable_address_response_ptr.as_ref() })
    else {
        panic!("Limine executable address was not provided");
    };

    let framebuffer_response = FRAMEBUFFER_REQUEST.get().response;
    // SAFETY:
    //
    // The framebuffer response can be read and should not change if it is not NULL.
    let framebuffers = if let Some(framebuffer_response) = unsafe { framebuffer_response.as_ref() }
    {
        // SAFETY:
        //
        // The Limine protocol specification specifies that this operation must be valid.
        unsafe {
            slice::from_raw_parts(
                framebuffer_response.framebuffers.cast::<&FramebufferV0>(),
                u64_to_usize(framebuffer_response.framebuffer_count),
            )
        }
    } else {
        &[]
    };

    #[cfg(target_arch = "aarch64")]
    let mp_response_ptr = MP_REQUEST.get().response.cast::<MpResponse>();
    #[cfg(target_arch = "x86_64")]
    let mp_response_ptr = MP_REQUEST.get().response.cast::<MpResponse>();

    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`MP_REQUST`] pointer
    // changes, the request has been fulfilled. Since `mp_response_ptr` was
    // initialized to `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid
    // pointer.
    let cpu_info_buffer = if let Some(mp_response) = unsafe { mp_response_ptr.as_ref() } {
        // SAFETY:
        //
        // The Limine protocol specification specifies that this operation must be valid.
        unsafe {
            slice::from_raw_parts(
                mp_response.cpus.cast::<&'static MpInfo>(),
                u64_to_usize(mp_response.cpu_count),
            )
        }
    } else {
        &[]
    };

    (
        memory_map_entries,
        hhdm_response.offset,
        executable_address_response.physical_base,
        executable_address_response.virtual_base,
        framebuffers,
        cpu_info_buffer,
    )
}

/// Implementation of acquiring a CPU ID number and waiting for functions to run.
extern "C" fn ap_loop(mp_info: &'static MpInfo) -> ! {
    let cpu_id = mp_info.extra_argument.swap(0, Ordering::Relaxed);
    mp_info.goto_address.store(0, Ordering::Release);

    loop {
        let mut next_func;
        loop {
            next_func = mp_info.extra_argument.load(Ordering::Acquire);
            if next_func != 0 {
                break;
            }
        }

        let func_description_ptr = u64_to_usize(next_func) as *mut (Procedure, *mut ());
        // SAFETY:
        //
        // The AP calling convention ensures that `func_description_ptr` is properly initialized
        // and points to an address that outlasts all processors running `ap_loop`.
        let (func, arg) = unsafe { *func_description_ptr };
        func(cpu_id, arg);
        mp_info.extra_argument.store(0, Ordering::Release);
    }
}

/// Limine-specific panic handler.
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    let framebuffers = if FRAMEBUFFERS.get().is_empty() {
        let framebuffer_response = FRAMEBUFFER_REQUEST.get().response;

        // SAFETY:
        //
        // The framebuffer response can be read and should not change if it is not NULL.
        if let Some(framebuffer_response) = unsafe { framebuffer_response.as_ref() } {
            // SAFETY:
            //
            // The Limine protocol specification specifies that this operation must be valid.
            unsafe {
                slice::from_raw_parts(
                    framebuffer_response.framebuffers.cast::<&FramebufferV0>(),
                    u64_to_usize(framebuffer_response.framebuffer_count),
                )
            }
        } else {
            &[]
        }
    } else {
        FRAMEBUFFERS.get()
    };

    if !primary_framebuffer_initialized() {
        for framebuffer in framebuffers.iter() {
            // SAFETY:
            //
            // We are panicking: we steal control over the framebuffers and overwrite all data.
            let Some(framebuffer) = (unsafe { create_surface(framebuffer) }) else {
                continue;
            };

            let mut console =
                TextConsole::new(framebuffer, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0x00);
            let _ = writeln!(console, "{info}");
        }
    }

    loop {
        core::hint::spin_loop()
    }
}

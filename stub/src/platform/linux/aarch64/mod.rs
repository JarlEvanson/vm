//! Support for booting using the Linux `aarch64` boot protocol.

use core::{
    alloc::Layout,
    arch::global_asm,
    iter, mem,
    ptr::{self, NonNull},
    slice,
};

use conversion::{u32_to_usize, u64_to_usize, u64_to_usize_strict, usize_to_u64};
use device_tree::{Fdt, raw::FdtHeader};
use pe::raw::{DosHeader, NtHeaders64, SectionHeader};
use uefi::table::{config, system::SystemTable};

use crate::{
    PANIC_HANDLER,
    arch::memory::physical_bits,
    platform::{
        AllocationPolicy, Allocator, BufferTooSmall, Frame, FrameRange, MapError, MappingType,
        MemoryDescriptor, MemoryMap, MemoryType, OutOfMemory, Page, PageRange, Permissions,
        PhysicalAddress, PhysicalAddressRange, PhysicalMemoryManager, Procedure, ProcessorManager,
        VirtualAddress, VirtualAddressRange, VirtualMemoryManager, frame_allocator,
        initialize_allocator, initialize_memory_config, initialize_physical_memory_manager,
        initialize_processor_management, initialize_virtual_memory_manager, page_size,
        set_device_tree, set_rsdp, set_smbios_32, set_smbios_64, set_uefi_system_table, set_xsdp,
    },
};

/// Rust entry point for the Linux boot protocol on `aarch64`.
pub extern "C" fn linux_main(
    dtb_ptr: *mut FdtHeader,
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
) -> ! {
    *PANIC_HANDLER.lock() = panic_handler;

    initialize_memory_config(512, physical_bits(), 512);
    let fdt_phys_addr = PhysicalAddress::new(dtb_ptr as u64);

    // SAFETY:
    //
    // This function is called before any call to the matching [`device_tree()`].
    unsafe {
        set_device_tree(fdt_phys_addr);
    }

    // SAFETY:
    //
    // These functions are called before any calls to their respective subsystems.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_virtual_memory_manager(&LinuxImpl);
        initialize_processor_management(&LinuxImpl);
    }

    // SAFETY:
    //
    // The `linux` boot protocol requires that the FDT is valid, while `revm-stub` properly manages
    // memory to ensure that `fdt` has exclusive access.
    let fdt = unsafe { Fdt::from_ptr(dtb_ptr).expect("flattened device tree must be valid") };
    let root = fdt.root();
    let chosen = root.find_node(c"chosen").expect("`chosen` node must exist");

    let image_iter = iter::once_with(|| MemoryDescriptor {
        range: PhysicalAddressRange::new(PhysicalAddress::new(image_start), image_size),
        region_type: MemoryType::BootloaderReclaimable,
    });
    let stack_iter = iter::once_with(|| MemoryDescriptor {
        range: PhysicalAddressRange::new(PhysicalAddress::new(stack_start), stack_size),
        region_type: MemoryType::BootloaderReclaimable,
    });
    if let Some(mmap_start) = chosen.find_property(c"linux,uefi-mmap-start") {
        let mmap_start = mmap_start
            .read_u64_at(0)
            .expect("linux,uefi-mmap-size must be 64-bits");

        let mmap_size = chosen
            .find_property(c"linux,uefi-mmap-size")
            .expect("UEFI DT node requires linux,uefi-mmap-size");
        let mmap_size = mmap_size
            .read_u32_at(0)
            .expect("linux,uefi-mmap-size must be 32-bits");

        let mmap_desc_size = chosen
            .find_property(c"linux,uefi-mmap-desc-size")
            .expect("UEFI DT node requires linux,uefi-mmap-desc-size");
        let mmap_desc_size = mmap_desc_size
            .read_u32_at(0)
            .expect("linux,uefi-mmap-desc-size must be 32-bits");

        let entry_count = mmap_size / mmap_desc_size;
        let mmap_ptr = mmap_start as *const u8;

        let entry_iter = (0..u64::from(entry_count)).map(|index| {
            let byte_offset = u64_to_usize_strict(index * u64::from(mmap_desc_size));
            // SAFETY:
            //
            // The `linux` boot protocol requires that the FDT is valid.
            let ptr = unsafe { mmap_ptr.add(byte_offset) };
            // SAFETY:
            //
            // The `linux` boot protocol requires that the FDT is valid.
            let desc = unsafe { ptr.cast::<uefi::memory::MemoryDescriptor>().read() };
            let region_type = match desc.region_type {
                uefi::memory::MemoryType::CONVENTIONAL => MemoryType::Free,
                uefi::memory::MemoryType::LOADER_CODE => MemoryType::BootloaderReclaimable,
                uefi::memory::MemoryType::LOADER_DATA => MemoryType::BootloaderReclaimable,
                uefi::memory::MemoryType::BOOT_SERVICES_CODE => MemoryType::BootloaderReclaimable,
                uefi::memory::MemoryType::BOOT_SERVICES_DATA => MemoryType::BootloaderReclaimable,
                uefi::memory::MemoryType::UNUSABLE => MemoryType::Bad,
                uefi::memory::MemoryType::ACPI_RECLAIM => MemoryType::AcpiReclaimable,
                uefi::memory::MemoryType::ACPI_NVS => MemoryType::AcpiNonVolatile,
                _ => MemoryType::Reserved,
            };

            MemoryDescriptor {
                range: PhysicalAddressRange::new(
                    PhysicalAddress::new(desc.physical_start),
                    desc.number_of_pages.strict_mul(4096),
                ),
                region_type,
            }
        });

        frame_allocator::initialize(entry_iter.chain(image_iter).chain(stack_iter));
    } else {
        let address_cells = root
            .find_property(c"#address-cells")
            .expect("#address-cells must be present");
        let address_cells = address_cells
            .read_u32_at(0)
            .expect("#address-cells must be 32-bits");

        let size_cells = root
            .find_property(c"#size-cells")
            .expect("#size-cells must be present");
        let size_cells = size_cells
            .read_u32_at(0)
            .expect("#size-cells must be 32-bits");
        let entry_size =
            (u32_to_usize(address_cells) + u32_to_usize(size_cells)) * mem::size_of::<u32>();

        let memory_iter = root
            .nodes()
            .filter_map(|node| {
                let node_name_bytes = node.name().to_bytes_with_nul();
                let bare_memory = node_name_bytes == c"memory".to_bytes_with_nul();
                let unit_memory = node_name_bytes.len() > c"memory@".to_bytes_with_nul().len()
                    && &node_name_bytes[0..c"memory@".to_bytes().len()] == c"memory@".to_bytes();
                if !bare_memory && !unit_memory {
                    return None;
                }

                let reg = node
                    .find_property(c"reg")
                    .expect("/memory nodes require the reg property");
                let entry_count = reg.data().len() / entry_size;

                Some((0..entry_count).flat_map(move |index| {
                    let address = match address_cells {
                        1 => u64::from(reg.read_u32_at(index * entry_count)?),
                        2 => reg.read_u64_at(index * entry_count)?,
                        _ => unimplemented!(),
                    };

                    let offset = u32_to_usize(address_cells) * mem::size_of::<u32>();
                    let size = match size_cells {
                        1 => u64::from(reg.read_u32_at(offset + index * entry_count)?),
                        2 => reg.read_u64_at(offset + index * entry_count)?,
                        _ => unimplemented!(),
                    };

                    let range = PhysicalAddressRange::new(PhysicalAddress::new(address), size);

                    Some(MemoryDescriptor {
                        range,
                        region_type: MemoryType::Free,
                    })
                }))
            })
            .flatten();

        let fdt_range = fdt.fdt_region();
        let fdt_range = PhysicalAddressRange::new(
            PhysicalAddress::new(usize_to_u64(fdt_range.as_ptr().addr())),
            usize_to_u64(fdt_range.len()),
        );

        let mut fdt_range_type = MemoryType::Reserved;
        let mut temp_iter = memory_iter.clone();
        while let Some(descriptor) = temp_iter.next() {
            if descriptor.region_type != MemoryType::Free {
                continue;
            }

            let mut range = descriptor.range;

            for test_descriptor in temp_iter.clone() {
                if test_descriptor.region_type != MemoryType::Free {
                    continue;
                }

                if let Some(merged_range) = range.merge(test_descriptor.range) {
                    range = merged_range;
                }
            }

            if range.intersection(fdt_range) == Some(fdt_range) {
                fdt_range_type = MemoryType::BootloaderReclaimable;
                break;
            }
        }

        let fdt_iter = iter::once(MemoryDescriptor {
            range: fdt_range,
            region_type: fdt_range_type,
        });

        let rsvmap_iter = fdt.reserve_entries().map(|entry| {
            let range = PhysicalAddressRange::new(PhysicalAddress::new(entry.address), entry.size);

            MemoryDescriptor {
                range,
                region_type: MemoryType::Reserved,
            }
        });

        frame_allocator::initialize(
            memory_iter
                .chain(fdt_iter)
                .chain(rsvmap_iter)
                .chain(image_iter)
                .chain(stack_iter),
        );
    }

    // SAFETY:
    //
    // These functions are called before any calls to their respective subsystems.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_physical_memory_manager(&LinuxImpl);
        initialize_allocator(&LinuxImpl);
    }

    'uefi_system_table: {
        let Some(uefi_system_table) = chosen.find_property(c"linux,uefi-system-table") else {
            break 'uefi_system_table;
        };

        let Some(uefi_system_table_address) = uefi_system_table.read_u64_at(0) else {
            crate::error!("non-compliant UEFI device tree");
            break 'uefi_system_table;
        };
        let uefi_system_table =
            ptr::without_provenance::<SystemTable>(u64_to_usize(uefi_system_table_address));

        let tables: [(::uefi::data_type::Guid, unsafe fn(PhysicalAddress)); 5] = [
            (config::ACPI, set_rsdp),
            (config::ACPI_2, set_xsdp),
            (config::DEVICE_TREE, set_device_tree),
            (config::SMBIOS, set_smbios_32),
            (config::SMBIOS_3, set_smbios_64),
        ];

        // SAFETY:
        //
        // `uefi_system_table` is not NULL and so according to the UEFI specification, the
        // configuration tables should be present.
        let system_table = unsafe { &*uefi_system_table };

        let config_table_count = system_table.number_of_table_entries;
        let config_tables_ptr = system_table.configuration_table;

        // SAFETY:
        //
        // `uefi_system_table` is not NULL and so according to the UEFI specification, the
        // configuration tables should be present.
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
        // This function is called before any calls to the matching [`uefi_system_table()`].
        unsafe { set_uefi_system_table(PhysicalAddress::new(uefi_system_table_address)) }
    }

    match crate::stub_main() {
        Ok(()) => {}
        Err(error) => crate::error!("{error}"),
    }

    loop {
        core::hint::spin_loop()
    }
}

/// Zero-sized implementation of most platform abstractions.
struct LinuxImpl;

impl PhysicalMemoryManager for LinuxImpl {
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

impl VirtualMemoryManager for LinuxImpl {
    fn max_physical_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(usize_to_u64(usize::MAX))
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
        // There is no need to unmap anything since the system has virtual memory disabled.
    }
}

impl Allocator for LinuxImpl {
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

impl ProcessorManager for LinuxImpl {
    fn main_processor_id(&self) -> u64 {
        0
    }

    fn current_processor_id(&self) -> u64 {
        0
    }

    fn processor_count(&self) -> u64 {
        1
    }

    fn run_on_all_processors(&self, procedure: Procedure, argument: *mut ()) {
        if self.processor_count() != 1 {
            todo!("implement processor bring-up")
        } else {
            procedure(self.main_processor_id(), argument)
        }
    }
}

/// Linux boot protocol-specific panic handler.
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    loop {
        core::hint::spin_loop()
    }
}

global_asm! {
    ".pushsection .linux-efi-header, \"ax\"",

    "header_start:",

    "ccmp	x18, #0, #0xd, pl",
    "b start",
    ".8byte 0", // Image load offset (little-endian).
    ".8byte 50 * 1024 * 1024", // Effective image size (little-endian).
    // Kernel Flags:
    //
    // Little endian
    // Unspecified page size
    // 2 MB aligned base should be as low as possible.
    ".8byte 0 | (0 << 1) | (1 << 3)",
    ".8byte 0", // Reserved 2.
    ".8byte 0", // Reserved 3.
    ".8byte 0", // Reserved 4.
    ".byte 0x41, 0x52, 0x4d, 0x64", // Magic number.
    ".4byte 0", // Reserved (UEFI binaries use this for PE COFF offset).

    // Arguments:
    //
    // x0: Physical address of device tree blob.
    "start:",

    // Acquire start of header.
    "adrp x5, header_start",
    "add x5, x5, :lo12:header_start",

    "ldr w8, [x5, #{PE_DOS_HEADER_LFANEW}]",
    "add x8, x5, x8",

    "ldrh w1, [x8, #{PE_NT_HEADERS_OPTIONAL_HEADER_SIZE}]",
    "add x9, x8, #{PE_NT_HEADERS_OPTIONAL_HEADER_OFFSET}",
    "add x9, x9, x1",

    "ldrh w10, [x8, #{PE_NT_HEADERS_SECTION_COUNT}]",
    "ldr w11, [x8, #{PE_NT_HEADERS_IMAGE_SIZE}]",
    "ldr w12, [x8, #{PE_NT_HEADERS_ENTRY_POINT}]",

    // Allocate PE image.

    // Load number of free bytes after `header_start`.
    "ldr x1, [x5, #{LINUX_HEADER_IMAGE_SIZE}]",

    // `byte_count - PE image size`.
    "sub x16, x1, x11",
    "add x16, x16, x5",

    // Align PE image base down to nearest 64 KiB address.
    "mov x2, #(64 * 1024 - 1)",
    "mvn x2, x2",
    "and x1, x16, x2",

    "mov x2, x11",

    // Allocate stack.

    // Take the 64 KiB below the PE image.
    "sub x3, x1, #(64 * 1024)",
    "mov x4, #(64 * 1024)",

    // Load the PE file.

    // Initialize temporary registers.
    "mov x5, x1",
    "mov x6, x2",

    // Set the PE region to all zeros.
    "5:",

    "mov x7, #0",
    "strb w2, [x5], 1",
    "sub x6, x6, #1",

    "cbnz x6, 5b",

    // Initialize section tracking registers.
    "mov x5, 0",
    "mov x6, x9",

    "adrp x7, header_start",
    "add x7, x7, :lo12:header_start",

    "section_loop:",

    "cmp x5, x10",
    "b.hs section_loop.finished",

    "ldr w16, [x6, #{PE_SECTION_HEADER_VIRTUAL_ADDRESS}]",
    "ldr w17, [x6, #{PE_SECTION_HEADER_FILE_OFFSET}]",
    "ldr w18, [x6, #{PE_SECTION_HEADER_FILE_SIZE}]",

    "add x16, x16, x1",
    "add x17, x17, x7",

    "section_loop.byte_copy:",

    "ldrb w19, [x17], 1",
    "strb w19, [x16], 1",
    "sub x18, x18, 1",

    "cbnz x18, section_loop.byte_copy",

    "add x5, x5, #1",
    "add x6, x6, #{PE_SECTION_HEADER_SIZE}",
    "b section_loop",

    "section_loop.finished:",

    // Calculate the stack top and initialize the stack.
    "add x5, x3, x4",
    "mov sp, x5",

    // Calculate the entry point.
    "add x5, x12, x1",

    // Clear x30 (Link register).
    "mov x30, 0",

    // Jump to kernel entry point.
    "br x5",

    ".popsection",

    LINUX_HEADER_IMAGE_SIZE = const { mem::offset_of!(linux::aarch64::Header, image_size) },

    PE_DOS_HEADER_LFANEW = const { mem::offset_of!(DosHeader, lfanew) },
    PE_NT_HEADERS_SECTION_COUNT = const { mem::offset_of!(NtHeaders64, file_header.number_of_sections) },
    PE_NT_HEADERS_OPTIONAL_HEADER_OFFSET = const { mem::offset_of!(NtHeaders64, optional_header) },
    PE_NT_HEADERS_OPTIONAL_HEADER_SIZE = const { mem::offset_of!(NtHeaders64, file_header.optional_header_size) },
    PE_NT_HEADERS_ENTRY_POINT = const { mem::offset_of!(NtHeaders64, optional_header.entry_point) },
    PE_NT_HEADERS_IMAGE_SIZE = const { mem::offset_of!(NtHeaders64, optional_header.image_size) },

    PE_SECTION_HEADER_SIZE = const { mem::size_of::<SectionHeader>() },
    PE_SECTION_HEADER_VIRTUAL_ADDRESS = const {  mem::offset_of!(SectionHeader, virtual_address) },
    PE_SECTION_HEADER_FILE_OFFSET = const {  mem::offset_of!(SectionHeader, pointer_to_raw_data) },
    PE_SECTION_HEADER_FILE_SIZE = const {  mem::offset_of!(SectionHeader, size_of_raw_data) },
}

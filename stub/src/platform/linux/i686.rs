//! Support for booting using the Linux `i686` boot protocol.

use core::{
    alloc::Layout,
    arch::global_asm,
    fmt::Write,
    iter, mem,
    ptr::{self, NonNull},
};

use conversion::{u16_to_usize, u32_to_usize, u64_to_usize_strict, usize_to_u64};
use linux::x86::{BootParams, ScreenInfo, VideoCapabilities};
use pe::raw::{NtHeaders64, SectionHeader};
use sync::Spinlock;

use crate::{
    PANIC_HANDLER,
    arch::{arch_specific::load_gdt, memory::physical_bits},
    platform::{
        AllocationPolicy, Allocator, BufferTooSmall, Console, Frame, FrameRange, MapError,
        MappingType, MemoryDescriptor, MemoryMap, MemoryType, Metadata, OutOfMemory, Page,
        PageRange, Permissions, PhysicalAddress, PhysicalAddressRange, PhysicalMemoryManager,
        Procedure, ProcessorManager, VirtualAddress, VirtualAddressRange, VirtualMemoryManager,
        graphics::{
            console::TextConsole,
            font::{FONT_MAP, GLYPH_ARRAY},
            surface::GenericSurface,
        },
        initialize_allocator, initialize_memory_config, initialize_physical_memory_manager,
        initialize_processor_management, initialize_virtual_memory_manager, page_size,
        register_console, set_rsdp, set_uefi_system_table, set_xsdp,
        shared::linux::E820Iter,
    },
};

/// Implementation of [`Console`] for [`ScreenInfo`].
static CONSOLE: Console = Console::new(write);

/// Location of the [`TextConsole`] that utilizes [`ScreenInfo`].
static TEXT_CONSOLE: Spinlock<Option<TextConsole<GenericSurface>>> = Spinlock::new(None);

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn linux_main(
    boot_params_ptr: *mut BootParams,
    image_start: u32,
    image_size: u32,
    stack_start: u32,
    stack_size: u32,
) -> ! {
    // SAFETY:
    //
    // This system has exclusive control over its system state.
    unsafe { load_gdt() }

    *PANIC_HANDLER.lock() = panic_handler;

    // SAFETY:
    //
    // The linux boot protocol ensure that we have exclusive control over the system and that the
    // provided pointer is non-zero and properly aligned.
    let boot_params = unsafe { &mut *boot_params_ptr };

    // SAFETY:
    //
    // This occurs before any accesses to [`SURFACE`] and provides exclusive access to [`CONSOLE`].
    if let Some(surface) = unsafe { screen_surface(boot_params.screen_info) } {
        let console = TextConsole::new(surface, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0);
        *TEXT_CONSOLE.lock() = Some(console);

        // SAFETY:
        //
        // This registration occurs before SMP and thus cannot overlap with other logging subsystem
        // operations.
        unsafe { register_console(NonNull::from_ref(&CONSOLE)) }
    }

    crate::debug!("Image Start: {:#x}", crate::util::image_start());
    initialize_memory_config(4096, physical_bits(), 4096);

    // SAFETY:
    //
    // These functions are called before any other function calls.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_physical_memory_manager(&LinuxImpl);
        initialize_virtual_memory_manager(&LinuxImpl);
        initialize_allocator(&LinuxImpl);
        initialize_processor_management(&LinuxImpl);
    }

    let e820_iter = E820Iter::new(PhysicalAddress::new(boot_params_ptr as u64));
    crate::platform::frame_allocator::initialize(
        e820_iter
            .map(|entry| {
                let start = PhysicalAddress::new(entry.addr);
                let range = PhysicalAddressRange::new(start, entry.size);

                let region_type = match entry.entry_type {
                    1 => MemoryType::Free,
                    2 => MemoryType::Reserved,
                    3 => MemoryType::AcpiReclaimable,
                    4 => MemoryType::AcpiNonVolatile,
                    5 => MemoryType::Bad,
                    _ => MemoryType::Reserved,
                };

                MemoryDescriptor { range, region_type }
            })
            .chain(iter::once_with(|| MemoryDescriptor {
                range: PhysicalAddressRange::new(
                    PhysicalAddress::new(u64::from(image_start)),
                    u64::from(image_size),
                ),
                region_type: MemoryType::BootloaderReclaimable,
            }))
            .chain(iter::once_with(|| MemoryDescriptor {
                range: PhysicalAddressRange::new(
                    PhysicalAddress::new(u64::from(stack_start)),
                    u64::from(stack_size),
                ),
                region_type: MemoryType::BootloaderReclaimable,
            })),
    );

    let rsdp_xsdp = PhysicalAddress::new(boot_params.acpi_rsdp_addr);
    // SAFETY:
    //
    // There exist zero overlapping calls to [`set_rsdp()`] and [`rsdp()`].
    unsafe { set_rsdp(rsdp_xsdp) };
    // SAFETY:
    //
    // There exist zero overlapping calls to [`set_rsdp()`] and [`rsdp()`].
    unsafe { set_xsdp(rsdp_xsdp) };

    if boot_params.efi_info.system_table != 0 || boot_params.efi_info.system_table_high != 0 {
        let address = u64::from(boot_params.efi_info.system_table)
            | (u64::from(boot_params.efi_info.system_table_high) << 32);

        // SAFETY:
        //
        // There exist zero overlapping calls to [`set_uefi_system_table()`] and
        // [`uefi_system_table()`]
        unsafe { set_uefi_system_table(PhysicalAddress::new(address)) };
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

/// Creates a new [`GenericSurface`] as specified by [`ScreenInfo`].
///
/// # Safety
///
/// The produced [`GenericSurface`] must have exclusive access to the underlying region it is
/// manipulating.
unsafe fn screen_surface(screen_info: ScreenInfo) -> Option<GenericSurface> {
    if screen_info.lfb_base == 0
        || (screen_info.ext_lfb_base != 0
            && screen_info.capabilities.0 & VideoCapabilities::BASE_64_BIT.0 != 0)
    {
        // Invalid base (either zero or outside of 4 GiB).
        return None;
    }

    let address = ptr::without_provenance_mut::<u8>(u32_to_usize(screen_info.lfb_base));
    let width = u16_to_usize(screen_info.lfb_width);
    let height = u16_to_usize(screen_info.lfb_height);
    let pitch = u16_to_usize(screen_info.lfb_line_length);
    let bpp = screen_info.lfb_depth;

    // SAFETY:
    //
    // The invariants of [`screen_surface()`] ensure that the invariants of
    // [`GenericSurface::new()`] are fulfilled.
    unsafe {
        GenericSurface::new(
            address,
            width,
            height,
            pitch,
            bpp,
            screen_info.red_size,
            screen_info.red_pos,
            screen_info.green_size,
            screen_info.green_pos,
            screen_info.blue_size,
            screen_info.blue_pos,
        )
    }
}

/// Implementation of [`Console::write`] for [`ScreenInfo`].
fn write(_: NonNull<Console>, metadata: Metadata, message: &str) {
    let mut console = TEXT_CONSOLE.lock();
    let Some(console) = console.as_mut() else {
        return;
    };
    let _ = write!(console, "[{:?}]: {message}", metadata.level);
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

    "_image_start:",
    ".skip {LINUX_HEADER_BASE_OFFSET}",

    ".byte (_real_mode_end - _real_mode_start) / 512", // setup_sects
    ".2byte 0",      // root_flags
    ".4byte 0",      // syssize (set by `xtask package`)
    ".2byte 0",      // ram_size
    ".2byte 0",      // vid_mode
    ".2byte 0",      // root_dev
    ".2byte 0xAA55", // boot_flag

    "_real_mode_start:",

    ".byte 0xEB, 0x6A",  // jump
    ".ascii \"HdrS\"",   // header
    ".2byte 0x020f",     // version (2.15)
    ".4byte 0",          // realmode_swtch
    ".2byte 0",          // start_sys_seg
    ".2byte 0",          // kernel_version
    ".byte 0",           // type_of_loader
    ".byte (1 << 0)",    // loadflags (LOADED_HIGH)
    ".2byte 0",          // setup_move_size
    ".4byte 0x100000",   // code32_start
    ".4byte 0",          // ramdisk_image
    ".4byte 0",          // ramdisk_size
    ".4byte 0",          // bootsect_kludge
    ".2byte 0",          // heap_end_ptr
    ".byte 0",           // ext_loader_ver
    ".byte 0",           // ext_loader_type
    ".4byte 0",          // cmd_line_ptr
    ".4byte 0xFFFFFFFF", // initrd_addr_max
    ".4byte 64 * 1024",  // kernel_alignment
    ".byte 1",           // relocatable_kernel
    ".byte 21",          // min_alignment (2 MiB)
    ".2byte 0",          // xloadflags
    ".4byte 0",          // cmdline_size
    ".4byte 0",          // hardware_subarch
    ".4byte 0",          // hardware_subarch_data
    ".4byte 0",          // payload_offset
    ".4byte 0",          // payload_length
    ".8byte 0",          // setup_data
    ".8byte 0x100000",   // pref_address
    ".4byte 0",          // init_size (set by `xtask package`)
    ".4byte 0",          // handover_offset
    ".4byte 0",          // kernel_info_offset

    // This is the target of the jump located at the start of `code16`.
    "5:", "hlt", "jmp 5b", // Spin forever; the 16-bit entrypoint is not supported.

    ".align 512",
    "_real_mode_end:",

    // 32-bit boot protocol entrypoint.
    //
    // Arguments:
    //
    // esi: pointer to [`BootParams`]
    "entry_32:",

    ".code32", // Force code to be interpreted as 32-bit.

    // Save [`BootParams`] pointer.
    "mov ebp, esi",

    // Acquire base address of the image.
    "call 1f",
    "1:",

    ".equ call_offset, 1b - entry_32",

    "pop eax",
    "sub eax, offset call_offset",

    // Calculate base offset of the PE header structure.
    ".equ _pe_header_offset, _pe_header - entry_32",
    "lea ebx, [eax + _pe_header_offset]",

    // Acquire total size of the loaded PE image.
    "mov ecx, [ebx + {PE_NT_HEADERS_IMAGE_SIZE}]",

    // Allocate PE image.
    //
    // eax: image_base
    // ebx: pe_header address
    // ecx: image_size
    // ebp: pointer to [`BootParams`]

    // Load the number of guaranteed free bytes after `entry_32`.
    "mov edx, [ebp + {LINUX_HEADER_BASE_OFFSET} + {LINUX_HEADER_INIT_SIZE}]",

    // Compute exclusive maximum free byte.
    "mov esi, eax",
    "add esi, edx",

    // Compute PE image base (aligned down to nearest 64 KiB address).
    "sub esi, ecx",
    "mov edi, (64 * 1024 - 1)",
    "not edi",
    "and esi, edi",

    // Set PE image base to be stack top.
    "mov esp, esi",

    // Load the PE file.
    //
    // ebx: pe_header address
    // ecx: image_size
    // esi: PE image base
    // ebp: pointer to [`BootParams`].

    // Zero out the PE allocation region.
    "push ecx",

    "cld",
    "xor eax, eax",
    "mov edi, esi",
    "rep stosb",

    "pop ecx",

    "mov eax, ebx",
    "mov ebx, ecx",
    "mov ecx, esi",

    // Parse PE header.
    //
    // eax: pe_header address
    // ebx: image_size
    // ecx: PE image base
    // ebp: pointer to [`BootParams`]

    // Compute address of entrypoint.
    "mov edx, [eax + {PE_NT_HEADERS_ENTRY_POINT}]",
    "add edx, ecx",
    "push edx",

    // Compute address of first PE section header.
    "movzx edx, word ptr [eax + {PE_NT_HEADERS_OPTIONAL_HEADER_SIZE}]",
    "lea edx, [eax + edx + {PE_NT_HEADERS_OPTIONAL_HEADER_OFFSET}]",
    "push edx",

    "movzx edx, word ptr [eax + {PE_NT_HEADERS_SECTION_COUNT}]",
    "push edx",

    // Load PE sections
    "mov edx, 0",
    "mov esi, [esp + 4]",

    // Arguments:
    //
    // eax: pe_header address
    // ebx: image_size
    // ecx: PE image base
    // edx: section_index
    // esi: section_header_address
    // ebp: pointer to [`BootParams`]
    //
    // Stack:
    //
    // section_count
    // pe_section_header_address
    // entrypoint
    "section_loop:",

    "mov edi, [esp]",
    "cmp edx, edi",
    "je section_loop.finished",

    "push ecx",
    "push esi",

    "mov edi, [esi + {PE_SECTION_HEADER_VIRTUAL_ADDRESS}]",
    "add edi, ecx",

    "mov ecx, [esi + {PE_SECTION_HEADER_FILE_SIZE}]",

    "mov esi, [esi + {PE_SECTION_HEADER_FILE_OFFSET}]",

    "push ecx",
    "call 1f",
    "1:", ".equ file_offset, 1b - _image_start",

    "pop ecx",
    "sub ecx, offset file_offset",

    "add esi, ecx",
    "pop ecx",

    "rep movsb",

    "pop esi",
    "pop ecx",

    "inc edx",
    "add esi, {PE_SECTION_HEADER_SIZE}",
    "jmp section_loop",

    "section_loop.finished:",

    // Retrieve entry point.
    "pop edi",
    "pop edi",
    "pop edi",

    "mov esi, 64 * 1024",
    "push esi",

    "mov edx, ecx",
    "sub edx, esi",
    "push edx",

    "push ebx",
    "push ecx",

    "push ebp",

    "xor eax, eax",
    "push eax",

    "push edi",

    "xor eax, eax",
    "mov ebx, eax",
    "mov ecx, eax",
    "mov edx, eax",
    "mov esi, eax",
    "mov edi, eax",
    "mov ebp, eax",

    "add esp, 4",
    "jmp [esp - 4]",

    ".align 8",
    "_pe_header:",

    ".popsection",

    LINUX_HEADER_BASE_OFFSET = const { linux::x86::Header::BASE_OFFSET },
    LINUX_HEADER_INIT_SIZE = const { mem::offset_of!(linux::x86::Header, init_size) },

    PE_NT_HEADERS_SECTION_COUNT = const { mem::offset_of!(NtHeaders64, file_header.number_of_sections) }  ,
    PE_NT_HEADERS_OPTIONAL_HEADER_OFFSET = const { mem::offset_of!(NtHeaders64, optional_header) },
    PE_NT_HEADERS_OPTIONAL_HEADER_SIZE = const { mem::offset_of!(NtHeaders64, file_header.optional_header_size) },
    PE_NT_HEADERS_ENTRY_POINT = const { mem::offset_of!(NtHeaders64, optional_header.entry_point) },
    PE_NT_HEADERS_IMAGE_SIZE = const { mem::offset_of!(NtHeaders64, optional_header.image_size) },

    PE_SECTION_HEADER_SIZE = const { mem::size_of::<SectionHeader>() },
    PE_SECTION_HEADER_VIRTUAL_ADDRESS = const {  mem::offset_of!(SectionHeader, virtual_address) },
    PE_SECTION_HEADER_FILE_OFFSET = const {  mem::offset_of!(SectionHeader, pointer_to_raw_data) },
    PE_SECTION_HEADER_FILE_SIZE = const {  mem::offset_of!(SectionHeader, size_of_raw_data) },
}

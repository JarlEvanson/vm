//! Support for booting using the Linux `x86_64` boot protocol.

use core::{
    alloc::Layout,
    arch::global_asm,
    fmt::Write,
    iter, mem,
    ptr::{self, NonNull},
};

use conversion::{u16_to_usize, usize_to_u64};
use linux::x86::{BootParams, ScreenInfo, VideoCapabilities};
use pe::raw::{NtHeaders64, SectionHeader};
use sync::Spinlock;

use crate::{
    PANIC_HANDLER,
    arch::{arch_specific::load_gdt, memory::physical_bits},
    platform::{
        AllocationPolicy, Allocator, BufferTooSmall, Console, Frame, FrameRange, MemoryDescriptor,
        MemoryMap, MemoryType, Metadata, OutOfMemory, Permissions, PhysicalAddress,
        PhysicalAddressRange, PhysicalMemoryManager, Procedure, ProcessorManager, frame_size,
        graphics::{
            console::TextConsole,
            font::{FONT_MAP, GLYPH_ARRAY},
            surface::GenericSurface,
        },
        initialize_allocator, initialize_memory_config, initialize_physical_memory_manager,
        initialize_processor_management, initialize_virtual_memory_manager,
        linux::x86_64::virt::setup_initial_mappings,
        map, register_console,
        shared::linux::E820Iter,
    },
};

/// Implementation of [`Console`] for [`ScreenInfo`].
static CONSOLE: Console = Console::new(write);

/// Location of the [`TextConsole`] that utilizes [`ScreenInfo`].
static TEXT_CONSOLE: Spinlock<Option<TextConsole<GenericSurface>>> = Spinlock::new(None);

mod virt;

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn linux_main(
    boot_params: *mut BootParams,
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
) -> ! {
    // SAFETY:
    //
    // This system has exclusive control over its system state.
    unsafe { load_gdt() }
    *PANIC_HANDLER.lock() = panic_handler;

    initialize_memory_config(4096, physical_bits(), 4096);
    setup_initial_mappings(image_start, image_size, stack_start, stack_size);

    // SAFETY:
    //
    // [`initialize_virtual_memory_manager()`] is called before any call to its subsystem.
    unsafe {
        initialize_virtual_memory_manager(&LinuxImpl);
    }

    // Initialize physical memory management.
    let e820_iter = E820Iter::new(PhysicalAddress::new(boot_params as u64));
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
                range: PhysicalAddressRange::new(PhysicalAddress::new(image_start), image_size),
                region_type: MemoryType::BootloaderReclaimable,
            }))
            .chain(iter::once_with(|| MemoryDescriptor {
                range: PhysicalAddressRange::new(PhysicalAddress::new(stack_start), stack_size),
                region_type: MemoryType::BootloaderReclaimable,
            })),
    );

    // SAFETY:
    //
    // [`initialize_physical_memory_manager()`] is called before any call to its subsystem.
    unsafe {
        initialize_physical_memory_manager(&LinuxImpl);
    }

    let boot_params_mapping = map(
        FrameRange::new(
            Frame::containing_address(PhysicalAddress::new(boot_params as u64)),
            1,
        ),
        Permissions::ReadWrite,
    )
    .expect("failed to map boot_params");
    // SAFETY:
    //
    // The `boot_params_mapping` ensures that `boot_params` is properly managed.
    let boot_params = unsafe {
        &mut *ptr::without_provenance_mut::<BootParams>(
            boot_params_mapping.range().start_address().value(),
        )
    };

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

    drop(boot_params_mapping);

    // SAFETY:
    //
    // These functions are called before any calls to their respective subsystems.
    #[expect(clippy::multiple_unsafe_ops_per_block)]
    unsafe {
        initialize_allocator(&LinuxImpl);
        initialize_processor_management(&LinuxImpl);
    }

    crate::debug!("Image Start: {:#x}", crate::util::image_start());
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
    if screen_info.lfb_base == 0 {
        return None;
    }

    let base_addr = if screen_info.capabilities.0 & VideoCapabilities::BASE_64_BIT.0 != 0 {
        (u64::from(screen_info.ext_lfb_base) << 32) | u64::from(screen_info.lfb_base)
    } else {
        u64::from(screen_info.lfb_base)
    };

    let width = u16_to_usize(screen_info.lfb_width);
    let height = u16_to_usize(screen_info.lfb_height);
    let pitch = u16_to_usize(screen_info.lfb_line_length);
    let bpp = screen_info.lfb_depth;

    let size = usize_to_u64(pitch * height);
    let frames = FrameRange::new(
        Frame::containing_address(PhysicalAddress::new(base_addr)),
        size.div_ceil(frame_size()),
    );

    // TODO: Map framebuffer as write combining.
    let mapping = map(frames, Permissions::ReadWrite).ok()?;
    let address = ptr::without_provenance_mut(mapping.range().start_address().value());

    // SAFETY:
    //
    // The invariants of [`screen_surface()`] ensure that the invariants of
    // [`GenericSurface::new()`] are fulfilled.
    let result = unsafe {
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
        )?
    };

    mem::forget(mapping);
    Some(result)
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

    ".byte 0xEB, 0x6A",            // jump
    ".ascii \"HdrS\"",             // header
    ".2byte 0x020f",               // version (2.15)
    ".4byte 0",                    // realmode_swtch
    ".2byte 0",                    // start_sys_seg
    ".2byte 0",                    // kernel_version
    ".byte 0",                     // type_of_loader
    ".byte (1 << 0)",              // loadflags (LOADED_HIGH)
    ".2byte 0",                    // setup_move_size
    ".4byte 0x100000",             // code32_start
    ".4byte 0",                    // ramdisk_image
    ".4byte 0",                    // ramdisk_size
    ".4byte 0",                    // bootsect_kludge
    ".2byte 0",                    // heap_end_ptr
    ".byte 0",                     // ext_loader_ver
    ".byte 0",                     // ext_loader_type
    ".4byte 0",                    // cmd_line_ptr
    ".4byte 0xFFFFFFFF",           // initrd_addr_max
    ".4byte 64 * 1024",            // kernel_alignment
    ".byte 1",                     // relocatable_kernel
    ".byte 21",                    // min_alignment (2 MiB)
    ".2byte (1 << 0) | (1 << 1)",  // xloadflags
    ".4byte 0",                    // cmdline_size
    ".4byte 0",                    // hardware_subarch
    ".4byte 0",                    // hardware_subarch_data
    ".4byte 0",                    // payload_offset
    ".4byte 0",                    // payload_length
    ".8byte 0",                    // setup_data
    ".8byte 0x100000",             // pref_address
    ".4byte 0",                    // init_size (set by `xtask package`)
    ".4byte 0",                    // handover_offset
    ".4byte 0",                    // kernel_info_offset

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

    "5:", "hlt", "jmp 5b", // Spin forever; the 32-bit entrypoint is not supported.

    ".align 0x200",

    // 64-bit boot protocol entrypoint.
    //
    // Arguments:
    //
    // rsi: pointer to [`BootParams`]
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.

    // Save [`BootParams`] pointer.
    "mov r15, rsi",

    // Acquire total size of the loaded PE image.
    "mov eax, [rip + _pe_header + {PE_NT_HEADERS_IMAGE_SIZE}]",

    // Load number of guaranteed free bytes after `entry_32`.
    "mov ecx, [r15 + {LINUX_HEADER_BASE_OFFSET} + {LINUX_HEADER_INIT_SIZE}]",

    // Compute exclusive maximum free byte.
    "lea ebx, [rip + entry_32]",
    "add ebx, ecx",

    // Allocate PE image.
    "sub rbx, rax",
    "mov rcx, (64 * 1024 - 1)",
    "not rcx",
    "and rbx, rcx",

    // Set PE image base to be stack top.
    "mov rsp, rbx",

    // Zero out the PE allocation region.
    "push rax",
    "push rbx",

    "cld",
    "mov rdi, rbx",
    "mov rcx, rax",
    "xor rax, rax",
    "rep stosb",

    "pop rbx",
    "pop rax",

    // Parse PE header.
    //
    // rax: PE image size
    // rbx: PE image base
    // r15: pointer to [`BootParams`]

    // Compute address of entrypoint.
    "mov ecx, [rip + _pe_header + {PE_NT_HEADERS_ENTRY_POINT}]",
    "add rcx, rbx",

    // Compute address of first PE section header.
    "movzx esi, word ptr [rip + _pe_header + {PE_NT_HEADERS_OPTIONAL_HEADER_SIZE}]",
    "lea rdx, [rip + _pe_header + {PE_NT_HEADERS_OPTIONAL_HEADER_OFFSET}]",
    "lea rdx, [rdx + rsi]",

    "movzx esi, word ptr [rip + _pe_header + {PE_NT_HEADERS_SECTION_COUNT}]",

    // Load PE sections.

    "mov rdi, 0",

    // Arguments:
    //
    // rax: PE image size
    // rbx: PE image base
    // rcx: PE entrypoint
    // rdx: PE section header address
    // rsi: PE section header count
    // rdi: PE section index
    "section_loop:",

    "cmp rdi, rsi",
    "je section_loop.finished",

    "push rcx",
    "push rsi",
    "push rdi",

    "mov ecx, [rdx + {PE_SECTION_HEADER_FILE_SIZE}]",
    "mov edi, [rdx + {PE_SECTION_HEADER_VIRTUAL_ADDRESS}]",
    "mov esi, [rdx + {PE_SECTION_HEADER_FILE_OFFSET}]",

    // Adjust source and destinations addresses.
    "lea rbp, [rip + _image_start]",
    "add rsi, rbp",

    "add rdi, rbx",

    "rep movsb",

    "pop rdi",
    "pop rsi",
    "pop rcx",

    "inc rdi",
    "add rdx, {PE_SECTION_HEADER_SIZE}",
    "jmp section_loop",

    "section_loop.finished:",

    // Save entrypoint.
    "mov r14, rcx",

    // Assemble argument registers.

    "mov rdi, r15", // boot_params
    "mov rsi, rbx", // image_start
    "mov rdx, rax", // image_size

    "mov r8, 64 * 1024", // stack_size
    "mov rcx, rsi",
    "sub rcx, r8",       // stack_start

    "xor rax, rax",
    "xor rbx, rbx",
    "xor rbp, rbp",
    "xor r9, r9",
    "xor r10, r10",
    "xor r11, r11",
    "xor r12, r12",
    "xor r13, r13",

    "push r13",
    "push r14",

    "xor r14, r14",
    "xor r15, r15",

    "add rsp, 8",
    "jmp [rsp - 8]",

    ".align 8",
    "_pe_header:",

    ".popsection",

    LINUX_HEADER_BASE_OFFSET = const { linux::x86::Header::BASE_OFFSET },
    LINUX_HEADER_INIT_SIZE = const { mem::offset_of!(linux::x86::Header, init_size) },

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

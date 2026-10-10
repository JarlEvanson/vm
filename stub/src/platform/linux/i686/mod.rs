//! Support for booting using the Linux `i686` boot protocol.

use core::{arch::global_asm, mem};

use linux::x86::BootParams;
use pe::raw::{NtHeaders64, SectionHeader};

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn main(
    boot_params_ptr: *mut BootParams,
    image_start: u32,
    image_size: u32,
    stack_start: u32,
    stack_size: u32,
) -> ! {
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

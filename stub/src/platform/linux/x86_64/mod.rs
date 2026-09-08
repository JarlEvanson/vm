//! Support for booting using the Linux `x86_64` boot protocol.

use core::{arch::global_asm, mem};

use linux::x86::BootParams;
use pe::raw::{NtHeaders64, SectionHeader};

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn main(
    boot_params: *mut BootParams,
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
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

    // TODO: Enable the 32-bit entry point.
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

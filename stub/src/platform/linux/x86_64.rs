//! Support for booting using the Linux x86 boot protocol.

use core::arch::global_asm;

global_asm! {
    ".pushsection .linux-efi-header, \"ax\"",

    ".equ section_count_offset, 4 + 2",
    ".equ optional_header_size_offset, 4 + 16",
    ".equ entry_point_offset, 4 + 20 + 16",
    ".equ image_size_offset, 4 + 20 + 56",

    ".equ file_size_offset, 16",
    ".equ file_offset_offset, 20",
    ".equ section_header_size, 40",

    "_section_start:",
    ".skip 0x1F1",

    ".byte _real_sectors",      // setup_sects
    ".2byte 0",                 // root_flags
    ".4byte 0x01000000",        // syssize
    ".2byte 0",                 // ram_size
    ".2byte 0",                 // vid_mode
    ".2byte 0",                 // root_dev
    ".2byte 0xAA55",            // boot_flag

    ".global _real_start",
    "_real_start:",

    ".byte 0xEB, 0x6a",         // jump
    ".ascii \"HdrS\"",          // header
    ".2byte 0x020f",            // version
    ".4byte 0",                 // realmode_swtch
    ".2byte 0",                 // start_sys_seg
    ".2byte 0",                 // kernel_version
    ".byte 0",                  // type_of_loader
    ".byte 0x1",                // loadflags
    ".2byte 0",                 // setup_move_size
    ".4byte 0x100000",          // code32_start
    ".4byte 0",                 // ramdisk_image
    ".4byte 0",                 // ramdisk_size
    ".4byte 0",                 // bootsect_kludge
    ".2byte 0",                 // heap_end_ptr
    ".byte 0",                  // ext_loader_ver
    ".byte 0",                  // ext_loader_type
    ".4byte 0",                 // cmd_line_ptr
    ".4byte 0xAFFFFFFF",        // initrd_addr_max
    ".4byte 0x200000",          // kernel_alignment
    ".byte 1",                  // relocatable_kernel
    ".byte 21",                 // min_alignment
    ".2byte 0x3",               // xloadflags
    ".4byte 0xFFF",             // cmdline_size
    ".4byte 0",                 // hardware_subarch
    ".8byte 0",                 // hardware_subarch_data
    ".4byte 0",                 // payload_offset
    ".4byte 0",                 // payload_length
    ".8byte 0",                 // setup_data
    ".8byte 0x100000",          // pref_address
    ".4byte 0x200000",          // init_size
    ".4byte 0",                 // handover_offset
    ".4byte _kernel_info - _section_start", // kernel_info_offset

    // This is the target of the jump located at the start of `code16`.
    ".byte 0xEB, 0xFE", // Spin forever; we don't support the 16-bit entry point.

    ".align 512",
    ".global _real_end",
    "_real_end:",

    // 32-bit boot protocol entry point.
    "entry_32:",

    ".code32", // Force code to be interpreted as 32-bit.

    // Initialize stack pointer.
    ".equ stack_offset, _stack_top - _section_start",
    "mov esp, 0x100000 + stack_offset",

    "5:",
    "jmp 5b",

    // Force `entry_64` to be located precisely 0x200 after `entry_32`.
    "8:", ".space 512 - (8b - entry_32)",

    // 64-bit boot protocol entry point.
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.

    // Initialize stack pointer.
    "lea rsp, [rip + _stack_top]",

    "cld", // Clear direction flag.
    "mov [rip + boot_params_pointer], rsi", // Save RSI (boot_params).

    // We need to map the file data that we care about, the boot_params structure, and provide a
    // slot for temporary mappings using our own page tables in order to allocate without possibly
    // overwriting the data.
    //
    // Next, we need to iterate through the e820 map to build a memory map, then iterate through
    // again in order to mark reserved regions (the stub file, the boot_params structure, the
    // command line, and any setup_data structures).
    //
    // We can then use our memory map to allocate enough pages to load the stub file using the PE
    // file format and provide access to a temporary mapping. This allows us to transition to Rust
    // code.

    // First, we calcuate maximum offset of the file we need to map.

    // Calculate location of section headers.
    "xor rsi, rsi", // Clear upper bits of rsi.
    "mov si, [rip + _pe_header + optional_header_size_offset]", // Load optional header size.
    "add rsi, 4 + 20", // Add PE signature and PE file header sizes.

    // Load section count and calculate total size of section headers.
    "xor rcx, rcx", // Clear upper bits of rcx.
    "mov cx, [rip + _pe_header + section_count_offset]", // Load section count.
    "mov rax, section_header_size", // Load size of each section header (constant).
    "mul rcx", // Calculate total size of section header.

    // Caculate maximum offset of PE header data we care about.
    "mov rbx, rsi",
    "add rbx, rax",

    // Currently, we have the maximum file offset we care about in rbx, the section count in rcx,
    // and the offset of the section headers from the start of the PE header in rsi.
    "lea rdx, [rip + _pe_header]", // Calculate address of PE header.
    "add rdx, rsi", // Calculate the address of the section header table.

    ".pe_header_loop:",
    "mov eax, [rdx + file_offset_offset]", // Load offset of the section data in the file.
    "add eax, [rdx + file_size_offset]", // Add size of the section data in the file.
    "sub eax, 0x400", // Subtract the truncated data from the offset.

    "cmp rax, rbx", // Compare the maximum secton data offset to the maximum relevant file offset.
    "cmovg rbx, rax", // Update the maximum relevant file offset to the larger of the two.

    "add rbx, section_header_size", // Set forward to the next section header.
    "loop .pe_header_loop", // Loop if any more section headers remain.

    // Store the start of the file and the maximum relevant offset of the file.
    "lea rax, [rip + entry_32]",
    "mov [rip + file_pointer], rax",
    "mov [rip + file_size], rbx",

    "5:",
    "jmp 5b",

    "linux_header_end:",

    "_kernel_info:",
    ".ascii \"LTop\"",
    ".4byte _kernel_info_var_len_data - _kernel_info",
    ".4byte _kernel_info_end - _kernel_info",
    ".4byte 0x01234567",

    "_kernel_info_var_len_data:",
    "_kernel_info_end:",

    ".align 8",
    "boot_params_pointer:", ".8byte 0",
    "file_pointer:", ".8byte 0",
    "file_size:", ".8byte 0",

    ".space 8192",
    "_stack_top:",

    ".align 8",
    "_pe_header:",

    ".popsection",
}

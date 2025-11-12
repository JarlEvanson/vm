//! Support for booting using the Linux x86 boot protocol.

use core::arch::global_asm;

global_asm! {
    ".pushsection .linux-efi-header, \"ax\"",

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
    "mov edi, [esi + 0x214]", // eax: Load address.

    ".equ stack_top_offset, (stack_top - entry_32)",
    "lea esp, [edi + stack_top_offset]",

    // Check for CPUID support.
    "pushfd",
    "pop ebx",
    "mov ecx, ebx",
    "xor ebx, 1 << 21",

    "push ebx",
    "popfd",
    "pushfd",
    "pop ebx",

    "push ecx",
    "popfd",

    "xor ebx, ecx",
    "je fail32",

    // CPUID is supported: check that the CPUID request indicating support for long mode exists.
    "mov eax, 0x80000000",
    "cpuid",
    "cmp eax, 0x80000000",
    "jb fail32",

    // Check that long mode is supported.
    "mov eax, 0x80000001",
    "cpuid",
    "test edx, 1 << 29",
    "jz fail32",

    ".equ page_table_start_offset, (page_tables_start - entry_32)",

    "9:", "jmp 9b",

    "fail32:", "jmp fail32",

    // Force `entry_64` to be located precisely 0x200 after `entry_32`.
    "8:", ".space 512 - (8b - entry_32)",

    // 64-bit boot protocol entry point.
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.
    "jmp entry_64",

    ".align 512",
    "stack:",
    ".skip 4096",
    "stack_top:",

    ".align 4096",
    "page_tables_start:",

    "pml4:", // We need a single PML4 tables.
    ".skip 4096",
    "pml3:", // To support alignment issues, we need 6 PML3 tables.
    ".skip 4096 * 6",
    "pml2:", // To support alignment issues, we need 21 PML2 tables
    ".skip 4096 * 21",

    "page_tables_end:",

    "linux_header_end:",

    "_kernel_info:",
    ".ascii \"LTop\"",
    ".4byte _kernel_info_var_len_data - _kernel_info",
    ".4byte _kernel_info_end - _kernel_info",
    ".4byte 0x01234567",

    "_kernel_info_var_len_data:",
    "_kernel_info_end:",

    ".popsection",
}

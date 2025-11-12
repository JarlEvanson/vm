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
    "jmp entry_32",

    // Force `entry_64` to be located precisely 0x200 after `entry_32`.
    "8:", ".space 512 - (8b - entry_32)",

    // 64-bit boot protocol entry point.
    "entry_64:",

    ".code64", // Force code to be interpreted as 64-bit.
    "jmp entry_64",

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

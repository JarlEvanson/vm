//! Support for booting using the Linux `aarch64` boot protocol.

use core::{arch::global_asm, mem};

use device_tree::raw::FdtHeader;
use pe::raw::{DosHeader, NtHeaders64, SectionHeader};

/// Rust entry point for the Linux boot protocol on `aarch64`.
pub extern "C" fn main(
    dtb_ptr: *mut FdtHeader,
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

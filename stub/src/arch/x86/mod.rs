//! Structures and functionality that are shared between `i686` and `x86_64`.

use x86::paging::{PagingMode, current_paging_mode};

use crate::arch::x86::switch::TablePointer;

pub mod memory;
pub mod relocation;
pub mod switch;

/// Loads in a default GDT.
///
/// # Safety
///
/// It must not violate system invariants to change the GDT or segmentation registers.
pub unsafe fn load_gdt() {
    static GDT: [u64; 5] = [
        0x0000_0000_0000_0000, // Null Segment
        0x00CF_9B00_0000_FFFF, // Kernel 32-bit code segment
        0x00CF_9300_0000_FFFF, // Kernel 32-bit data segment
        0x00AF_9B00_0000_FFFF, // Kernel 64-bit code segment
        0x00CF_9300_0000_FFFF, // Kernel 64-bit data segment
    ];

    let (code_segment, data_segment) = match current_paging_mode() {
        PagingMode::Disabled | PagingMode::Bits32 | PagingMode::Pae => (8usize, 16usize),
        PagingMode::Level4 | PagingMode::Level5 => (24, 32),
    };

    let gdtr = TablePointer {
        size: 8 * 5,
        pointer: &raw const GDT as u64,
    };

    // SAFETY:
    //
    // The invariants of [`load_gdt()`] ensure that this operation is safe.
    unsafe {
        core::arch::asm!(
            "lgdt [{gdtr}]",
            "mov ds, {data:x}",
            "mov es, {data:x}",
            "mov fs, {data:x}",
            "mov gs, {data:x}",
            "mov ss, {data:x}",

            "push {code}",

            "call 99f",
            "99:",

            ".equ call_offset, 99f - 99b",
            "pop {tmp}",
            "add {tmp}, offset call_offset",

            "push {tmp}",

            #[cfg(target_arch = "x86")]
            "retf",
            #[cfg(target_arch = "x86_64")]
            "retfq",
            "99:",
            gdtr = inout(reg) &raw const gdtr => _, code = inout(reg) code_segment => _, data = in(reg) data_segment, tmp = lateout(reg) _)
    }
}

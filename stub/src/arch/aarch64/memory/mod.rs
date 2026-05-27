//! Structures and functionality that are specific to `aarch64` memory manipulation.

use aarch64::msr::raw::read_id_aa64mmfr0_el1;

pub mod paging;

/// Returns the number of physical address bits that the processor supports.
pub fn physical_bits() -> u8 {
    // SAFETY:
    //
    // This program runs at EL1 or EL2 and thus is safe to execute.
    let id_aa64mmfr0_el1 = unsafe { read_id_aa64mmfr0_el1() };
    match id_aa64mmfr0_el1 & 0b1111 {
        0b0000 => 32,
        0b0001 => 36,
        0b0010 => 40,
        0b0011 => 42,
        0b0100 => 44,
        0b0101 => 48,
        0b0110 => 52,
        0b0111 => 56,
        val => unimplemented!("unknown ID_AA64MMFR0_EL1.PARange value: {val:#b}"),
    }
}

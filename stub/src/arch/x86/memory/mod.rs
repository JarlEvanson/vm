//! Structures and functionality that are specific to `i686` or `x86_64` memory manipulation.

pub mod paging;

use x86::cpuid::{Cpuid, cpuid_unchecked, supports_cpuid};

/// Returns the number of physical address bits that the processor supports.
pub fn physical_bits() -> u8 {
    if !supports_cpuid() {
        return 32;
    }

    // SAFETY:
    //
    // The `CPUID` instruction is supported.
    let Cpuid {
        eax: max_extended,
        ebx: _,
        ecx: _,
        edx: _,
    } = unsafe { cpuid_unchecked(0x80000000, 0) };

    if max_extended < 0x80000008 {
        let Cpuid {
            eax: max_basic,
            ebx: _,
            ecx: _,
            edx: _,
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
        } = unsafe { cpuid_unchecked(0x0, 0) };

        if max_basic < 0x1 {
            return 32;
        }

        // SAFETY:
        //
        // The `CPUID` instruction is supported.
        let Cpuid {
            eax: _,
            ebx: _,
            ecx: _,
            edx,
        } = unsafe { cpuid_unchecked(0x1, 0) };
        if ((edx >> 6) & 0b1) == 0b1 {
            return 36;
        } else {
            return 32;
        }
    }

    // SAFETY:
    //
    // The `CPUID` instruction is supported.
    let Cpuid {
        eax,
        ebx: _,
        ecx: _,
        edx: _,
    } = unsafe { cpuid_unchecked(0x80000008, 0) };

    (eax & 0xFF) as u8
}

//! Model-Specific Register-related functions.

use core::arch::asm;

use crate::cpuid::{cpuid_unchecked, supports_cpuid};

/// Returns `true` if the processor supports the RDMSR and WRMSR instructions.
pub fn supports_msr() -> bool {
    if !supports_cpuid() {
        return false;
    }

    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    let result = unsafe { cpuid_unchecked(1, 0) };

    ((result.edx >> 5) & 1) == 1
}

/// Returns the contents of the 64-bit MSR specified by `msr`.
///
/// # Safety
///
/// The RDMSR instruction must be safe to perform on this processor.
#[expect(clippy::as_conversions)]
pub unsafe fn read_msr(msr: u32) -> u64 {
    debug_assert!(supports_msr());

    let eax: u32;
    let edx: u32;

    // SAFETY:
    //
    // According to the invariants of this function, the RDMSR instruction is safe to perform.
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            lateout("eax") eax,
            lateout("edx") edx,
            options(preserves_flags)
        )
    }

    ((edx as u64) << 32) | (eax as u64)
}

/// Returns the contents of the 64-bit MSR specified by `msr`.
///
/// # Safety
///
/// The WRMSR instruction must be safe to perform on this processor.
#[expect(clippy::as_conversions)]
pub unsafe fn write_msr(msr: u32, value: u64) {
    debug_assert!(supports_msr());

    let eax = value as u32;
    let edx = (value >> 32) as u32;

    // SAFETY:
    //
    // According to the invariants of this function, the WRMSR instruction is safe to perform.
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") eax,
            in("edx") edx,
            options(preserves_flags)
        )
    }
}

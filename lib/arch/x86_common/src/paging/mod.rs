//! Paging-related utilities.

use core::arch;

use crate::{
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::read_msr,
};

pub mod tlb;

/// Returns the currently active [`PagingMode`].
pub fn current_paging_mode() -> PagingMode {
    let max_paging_mode = max_supported_paging_mode();
    let cr0: u32;

    // SAFETY:
    //
    // Accessing the CR0 register is safe.
    #[cfg(target_arch = "x86")]
    unsafe {
        arch::asm!("mov {}, cr0", lateout(reg) cr0)
    };
    // SAFETY:
    //
    // Accessing the CR0 register is safe.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        arch::asm!("mov {:r}, cr0", lateout(reg) cr0)
    };

    if (cr0 >> 31) & 1 != 1 || max_paging_mode == PagingMode::Disabled {
        return PagingMode::Disabled;
    }

    let cr4: u32;

    // SAFETY:
    //
    // Accessing the CR4 register is safe.
    #[cfg(target_arch = "x86")]
    unsafe {
        arch::asm!("mov {}, cr4", lateout(reg) cr4)
    };
    // SAFETY:
    //
    // Accessing the CR4 register is safe.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        arch::asm!("mov {:r}, cr4", lateout(reg) cr4)
    };

    if (cr4 >> 5) & 1 != 1 || max_paging_mode == PagingMode::Bits32 {
        return PagingMode::Bits32;
    }

    if max_supported_paging_mode() == PagingMode::Pae {
        return PagingMode::Pae;
    }

    // SAFETY:
    //
    // The RDMSR instruction is available on this processor since 4-level or 5-level paging is
    // supported.
    let efer = unsafe { read_msr(0xC000_0080) };
    if (efer >> 10) & 1 != 1 {
        return PagingMode::Pae;
    }

    if (cr4 >> 12) & 1 != 1 {
        return PagingMode::Level4;
    }

    PagingMode::Level5
}

/// Returns the maximum supported [`PagingMode`] for the processor.
pub fn max_supported_paging_mode() -> PagingMode {
    if !supports_cpuid() {
        return PagingMode::Bits32;
    }

    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    let cpuid_1 = unsafe { cpuid_unchecked(1, 0) };
    if (cpuid_1.edx >> 6) & 1 == 0 {
        return PagingMode::Bits32;
    }

    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    let cpuid_80000000 = unsafe { cpuid_unchecked(0x80000000, 0) };
    if cpuid_80000000.eax < 0x80000001 {
        return PagingMode::Pae;
    }

    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    let cpuid_80000001 = unsafe { cpuid_unchecked(0x80000001, 0) };
    if (cpuid_80000001.edx >> 29) & 1 == 0 {
        return PagingMode::Pae;
    }

    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    let cpuid_7 = unsafe { cpuid_unchecked(7, 0) };
    if (cpuid_7.ecx >> 16) & 1 != 1 {
        return PagingMode::Level4;
    }

    PagingMode::Level5
}

/// A paging mode in the `x86` and `x86_64` architectures.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PagingMode {
    /// Paging is disabled.
    #[default]
    Disabled,
    /// Paging mode is 32-bit.
    Bits32,
    /// Paging mode is PAE (Physical Address Extension)
    Pae,
    /// Paging mode has 4 levels.
    Level4,
    /// Paging mode has 5 levels.
    Level5,
}

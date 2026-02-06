//! CPUID-related structures and functions.

use core::arch::asm;

/// Returns `true` if the processor supports the CPUID instruction.
pub fn supports_cpuid() -> bool {
    #[cfg(target_arch = "x86")]
    let result: u32;
    #[cfg(target_arch = "x86_64")]
    let result: u64;

    // SAFETY:
    //
    // This assembly block does not cause UB.
    unsafe {
        asm!(
            // Byte encodings are used to prevent code duplication for 4-byte vs 8-byte flags
            // operations.
            ".byte 0x9C", // Byte encoding of PUSHF
            "pop {flags}",
            "mov {save}, {flags}",
            "xor {flags}, 0x200000",
            "push {flags}",
            ".byte 0x9D", // Byte encoding of POPF
            ".byte 0x9C", // Byte encoding of PUSHF
            "pop {flags}",
            "xor {flags}, {save}",
            flags = lateout(reg) result,
            save = lateout(reg) _,
        )
    }

    (result & 0x20_0000) == 0x20_0000
}

/// Returns the [`Cpuid`] result associated with `leaf` and `subleaf` on this processor.
///
/// # Safety
///
/// The CPUID instruction must be safe to perform on this processor.
pub unsafe fn cpuid_unchecked(leaf: u32, subleaf: u32) -> Cpuid {
    debug_assert!(supports_cpuid());

    let mut result = Cpuid {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    #[cfg(target_arch = "x86")]
    // SAFETY:
    //
    // The CPUID instruction is safe to perform on this processor.
    unsafe {
        asm!(
            "mov {scratch}, ebx",
            "cpuid",
            "xchg {scratch}, ebx",
            inout("eax") leaf => result.eax,
            scratch = lateout(reg) result.ebx,
            inout("ecx") subleaf => result.ecx,
            lateout("edx") result.edx,
            options(nostack, nomem, preserves_flags)
        )
    }
    #[cfg(target_arch = "x86_64")]
    // SAFETY:
    //
    // The CPUID instruction is safe to perform on this processor.
    unsafe {
        asm!(
            "mov {scratch:r}, rbx",
            "cpuid",
            "xchg {scratch:r}, rbx",
            inout("eax") leaf => result.eax,
            scratch = lateout(reg) result.ebx,
            inout("ecx") subleaf => result.ecx,
            lateout("edx") result.edx,
            options(nostack, nomem, preserves_flags)
        )
    }

    result
}

/// Result of performing a CPUID instruction.
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cpuid {
    /// Value stored in the EAX registers when executing CPUID.
    pub eax: u32,
    /// Value stored in the EBX registers when executing CPUID.
    pub ebx: u32,
    /// Value stored in the ECX registers when executing CPUID.
    pub ecx: u32,
    /// Value stored in the EDX registers when executing CPUID.
    pub edx: u32,
}

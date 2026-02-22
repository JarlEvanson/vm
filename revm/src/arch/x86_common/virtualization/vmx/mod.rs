//! VMX-related functionality.

use x86_common::{
    control::{Cr0, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr},
};

const IA32_FEATURE_CONTROL_MSR: u32 = 0x3A;

const IA32_VMX_CR0_FIXED0: u32 = 0x486;
const IA32_VMX_CR0_FIXED1: u32 = 0x487;
const IA32_VMX_CR4_FIXED0: u32 = 0x488;
const IA32_VMX_CR4_FIXED1: u32 = 0x489;

/// Returns `true` if virtualization is supported.
pub fn supported() -> bool {
    if !supports_cpuid() {
        return false;
    }

    // SAFETY:
    //
    // `CPUID` is supported.
    let cpuid_result = unsafe { cpuid_unchecked(1, 0) };
    if (cpuid_result.ecx >> 5) & 0b1 != 0b1 {
        return false;
    }

    if !supports_msr() {
        return false;
    }

    // SAFETY:
    //
    // `RDMSR` is supported.
    let ia32_feature_control = unsafe { read_msr(IA32_FEATURE_CONTROL_MSR) };
    if ia32_feature_control & 0b1 == 0b1 {
        // Locked.
        if (ia32_feature_control >> 2) & 0b1 != 0b1 {
            return false;
        }
    }

    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr0_fixed_0 = unsafe { read_msr(IA32_VMX_CR0_FIXED0) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr0_fixed_1 = unsafe { read_msr(IA32_VMX_CR0_FIXED1) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr4_fixed_0 = unsafe { read_msr(IA32_VMX_CR4_FIXED0) };
    // SAFETY:
    //
    // `RDMSR` is supported.
    let cr4_fixed_1 = unsafe { read_msr(IA32_VMX_CR4_FIXED1) };

    let cr0_forced_on = Cr0::from_bits(cr0_fixed_0);
    let cr0_forced_off = Cr0::from_bits(!cr0_fixed_1);
    let cr0_flexible = Cr0::from_bits(cr0_fixed_1 & !cr0_fixed_0);

    let cr4_forced_on = Cr4::from_bits(cr4_fixed_0);
    let cr4_forced_off = Cr4::from_bits(!cr4_fixed_1);
    let cr4_flexible = Cr4::from_bits(cr4_fixed_1 & !cr4_fixed_0);

    crate::debug!("Forced On: {cr0_forced_on}");
    crate::debug!("Forced Off: {cr0_forced_off}");
    crate::debug!("Flexible: {cr0_flexible}");

    crate::debug!("Forced On: {cr4_forced_on}");
    crate::debug!("Forced Off: {cr4_forced_off}");
    crate::debug!("Flexible: {cr4_flexible}");

    true
}

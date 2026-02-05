//! Implementations and definitions of virtualization code utilizing VMX.

use x86_common::{
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr},
};

pub const IA32_FEATURE_CONTROL_MSR: u32 = 0x0000_003A;

/// Returns `true` if VMX is supported on this processor.
pub fn supported() -> bool {
    if !supports_cpuid() || !supports_msr() {
        return false;
    }

    // SAFETY:
    //
    // `CPUID` is supported on this processor.
    let cpuid_result = unsafe { cpuid_unchecked(0x0000_0001, 0x0000_0000) };
    if ((cpuid_result.ecx >> 5) & 0b1) == 0 {
        return false;
    }

    // SAFETY:
    //
    // `RDMSR` is supported on this processor.
    let ia32_feature_control = unsafe { read_msr(0x0000_003A) };
    let locked = ia32_feature_control & 0b1 == 0b1;
    let vmx_outside_smx = ia32_feature_control & 0b100 == 0b100;
    !locked || vmx_outside_smx
}

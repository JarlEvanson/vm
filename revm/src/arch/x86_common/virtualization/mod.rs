//! Virtualization-related functionality.

use x86_common::cpuid::{cpuid_unchecked, supports_cpuid};

use crate::arch::x86_common::virtualization::vmx::VmxConfig;

pub mod vmx;

/// Returns `true` if virtualization is supported.
pub fn supported() -> Option<VirtualizationConfig> {
    if !supports_cpuid() {
        return None;
    }

    // SAFETY:
    //
    // `CPUID` is supported.
    let cpuid_result = unsafe { cpuid_unchecked(0, 0) };

    if cpuid_result.ebx == 0x756E_6547
        && cpuid_result.ecx == 0x6C65_746E
        && cpuid_result.edx == 0x4965_6E69
    {
        // Intel.
        vmx::supported().map(VirtualizationConfig::Vmx)
    } else if cpuid_result.ebx == 0x6874_7541
        && cpuid_result.ecx == 0x444D_4163
        && cpuid_result.edx == 0x6974_6E65
    {
        // AMD.
        crate::warn!("virtualization on AMD is not currently supported");
        None
    } else {
        crate::warn!("unknown processor vendor");
        None
    }
}

/// TODO:
pub fn enable(config: VirtualizationConfig) {
    match config {
        VirtualizationConfig::Vmx(config) => vmx::enable(config).expect("failed"),
    }
}

pub enum VirtualizationConfig {
    Vmx(VmxConfig),
}

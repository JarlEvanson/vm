//! Architecture-specific code.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

pub mod generic;

/// Architecture-dependent paging code.
pub mod paging {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::paging::{
        X86CommonScheme as ArchScheme, X86CommonSchemeError as ArchSchemeError,
    };

    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::paging::{
        Aarch64Scheme as ArchScheme, Aarch64SchemeError as ArchSchemeError,
    };
}

/// Architecture-dependent relocation code.
pub mod relocation {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::relocation::{read_size, relocate};

    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::relocation::{read_size, relocate};
}

/// Architecture-dependent cross address space switching code.
pub mod switch {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::switch::{
        ArchCodeLayout, CpuStorage, allocate_code, arch_policy, arch_table_64_bit, arch_table_size,
        base_cpu_storage, enter, finalize_cpu_data, handle_stack_allocation,
        handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
    };

    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::switch::{
        ArchCodeLayout, CpuStorage, allocate_code, arch_policy, arch_table_64_bit, arch_table_size,
        base_cpu_storage, enter, finalize_cpu_data, handle_stack_allocation,
        handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
    };
}

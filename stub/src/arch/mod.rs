//! Architecture-specific code.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

pub mod generic;

/// Architecture-specific memory management code.
pub mod memory {
    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::memory::{
        paging::Aarch64TranslationScheme as ArchTranslationScheme, physical_bits,
    };

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86::memory::{
        paging::X86TranslationScheme as ArchTranslationScheme, physical_bits,
    };
}

/// Architecture-dependent relocation code.
pub mod relocation {
    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::relocation::{read_size, relocate};

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86::relocation::{read_size, relocate};
}

/// Architecture-dependent cross address space switching code.
pub mod switch {
    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::switch::{
        ArchCodeLayout, CpuStorage, allocate_code, arch_policy, arch_table_64_bit, arch_table_size,
        base_cpu_storage, enter, finalize_cpu_data, handle_stack_allocation,
        handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
    };

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86::switch::{
        ArchCodeLayout, CpuStorage, allocate_code, arch_policy, arch_table_64_bit, arch_table_size,
        base_cpu_storage, enter, finalize_cpu_data, handle_stack_allocation,
        handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
    };
}

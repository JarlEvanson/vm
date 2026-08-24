//! Architecture-specific code.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

#[cfg(target_arch = "aarch64")]
use aarch64 as arch;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86 as arch;

pub mod interface;

/// Architecture-specific memory management code.
pub mod memory {
    pub use super::arch::memory::{paging::ArchTranslationScheme, physical_bits};
}

/// Architecture-dependent relocation code.
pub mod relocation {
    pub use super::arch::relocation::{read_size, relocate};
}

/// Architecture-dependent cross address space switching code.
pub mod switch {
    pub use super::arch::switch::{
        ArchCodeLayout, CpuStorage, allocate_code, arch_policy, arch_table_64_bit, arch_table_size,
        base_cpu_storage, enter, finalize_cpu_data, handle_stack_allocation,
        handle_storage_allocation, write_protocol_table_32, write_protocol_table_64,
    };
}

/// Architecture-specific functionality.
pub mod arch_specific {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86::load_gdt;
}

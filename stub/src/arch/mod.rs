//! Architecture-specific code.

pub mod interface;

/// Architecture-specific memory management code.
pub mod memory {
    pub use super::arch::memory::{
        paging::Aarch64TranslationScheme as ArchTranslationScheme, physical_bits,
    };
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

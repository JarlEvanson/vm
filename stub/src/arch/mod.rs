//! Architecture-specific code.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

pub mod generic;

/// Architecture-dependent paging code.
pub mod paging {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::paging::{
        X86CommonScheme as ArchScheme, X86CommonSchemeError as ArchSchemeError,
    };
}

/// Architecture-dependent relocation code.
pub mod relocation {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::relocation::{read_size, relocate};
}

//! Architecture-specific functionality.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

/// Virtualization-related functionality.
pub mod virtualization {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use super::x86_common::virtualization::supported;
}

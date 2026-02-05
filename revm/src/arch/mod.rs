//! Architecture-specific code.

#[cfg(target_arch = "x86_64")]
mod x86_64;

/// Implementations and definitions related to virtualization support.
pub mod virtualization {
    #[cfg(target_arch = "x86_64")]
    pub use super::x86_64::virtualization::supported;
}

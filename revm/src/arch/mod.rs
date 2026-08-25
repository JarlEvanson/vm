//! Architecture-specific functionality.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86")]
mod i686;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
use aarch64 as arch;
#[cfg(target_arch = "x86")]
use i686 as arch;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86 as arch;
#[cfg(target_arch = "x86_64")]
use x86_64 as arch;

pub mod interface;

/// Architecture-dependent memory management code.
pub mod memory {
    pub use super::arch::memory::compute_page_frame_size;
}

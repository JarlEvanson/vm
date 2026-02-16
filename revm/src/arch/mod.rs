//! Architecture-specific functionality.

pub mod generic;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86")]
mod x86_32;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

#[cfg(target_arch = "aarch64")]
pub use aarch64::compute_page_size;

#[cfg(target_arch = "x86_64")]
pub use x86_64::paging::find_free_region;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86_common::compute_page_size;

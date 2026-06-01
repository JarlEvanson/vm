//! Architecture-specific functionality.

// ARCHITECTURE-SPECIFC FUNCTIONALITY IMPLEMENTATIONS.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86")]
mod i686;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
use aarch64 as arch_impl;
#[cfg(target_arch = "x86")]
use i686 as arch_impl;
#[cfg(target_arch = "x86_64")]
use x86_64 as arch_impl;

// ARCHITECTURE-SPECIFC FUNCTIONALITY WRAPPERS.

pub mod capabilities;

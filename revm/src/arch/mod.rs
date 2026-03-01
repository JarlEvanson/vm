//! Architecture-specific functionality.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86")]
mod x86_32;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

#[cfg(target_arch = "aarch64")]
use aarch64 as arch_impl;
#[cfg(target_arch = "x86")]
use x86_32 as arch_impl;
#[cfg(target_arch = "x86_64")]
use x86_64 as arch_impl;

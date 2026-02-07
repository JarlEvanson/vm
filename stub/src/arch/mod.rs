//! Architecture-specific code.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

pub mod generic;

//! Code shared between protocols for purposes of interacting with the `linux` boot protocol.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86::*;

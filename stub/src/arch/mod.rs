//! Architecture-specific code.

#[cfg(target_arch = "x86_64")]
mod x86_64;

pub mod generic;

#[cfg(target_arch = "x86_64")]
pub use x86_64::address_space::X86_64AddressSpace as AddressSpaceImpl;

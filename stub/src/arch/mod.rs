//! Architecture-specific code.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

pub mod generic;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86_common::paging::{
    X86CommonAddressSpace as ArchAddressSpace, X86CommonAddressSpaceError as ArchAddressSpaceError,
    bits_32::{Bits32AddressSpace, Bits32Error},
    long_mode::{LongModeAddressSpace, LongModeError},
    new_address_space,
    pae::{PaeAddressSpace, PaeError},
};

//! Paging structures that are common to `x86_32` and `x86_64`.

use core::fmt;

use elf::header::Machine;

use crate::{
    arch::{
        ArchAddressSpace, ArchAddressSpaceError, LongModeError,
        generic::address_space::{AddressSpace, MapError, NoMapping, NotFound, ProtectionFlags},
        x86_common::paging::{
            bits_32::Bits32AddressSpace, long_mode::LongModeAddressSpace, pae::PaeAddressSpace,
        },
    },
    platform::{read_u32_at, read_u64_at, write_u32_at, write_u64_at},
};

pub mod bits_32;
pub mod long_mode;
pub mod pae;

/// Constructs a new [`ArchAddressSpace`] that is compatible with the provided [`Machine`].
///
/// # Errors
///
/// - [`ArchAddressSpaceError::NotSupported`]: Returned if the requested address spaces are not
///   supported.
/// - [`ArchAddressSpaceError::OutOfMemory`]: Returned if an error occurred while constructing the
///   new [`ArchAddressSpace`].
pub fn new_address_space(machine: Machine) -> Result<ArchAddressSpace, ArchAddressSpaceError> {
    let address_space = match machine {
        Machine::INTEL_386 => {
            if let Ok(pae) = PaeAddressSpace::new_current(read_u64_at, write_u64_at) {
                ArchAddressSpace::Pae(pae)
            } else if let Ok(bits_32) = Bits32AddressSpace::new_current(read_u32_at, write_u32_at) {
                ArchAddressSpace::Bits32(bits_32)
            } else {
                return Err(ArchAddressSpaceError::NotSupported);
            }
        }
        Machine::X86_64 => ArchAddressSpace::LongMode(
            LongModeAddressSpace::new_current(read_u64_at, write_u64_at).map_err(|error| {
                match error {
                    LongModeError::NotActive => ArchAddressSpaceError::NotSupported,
                    LongModeError::OutOfMemory(_) => ArchAddressSpaceError::OutOfMemory,
                }
            })?,
        ),
        _ => return Err(ArchAddressSpaceError::NotSupported),
    };

    Ok(address_space)
}

/// Various errors that can occur while con
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X86CommonAddressSpaceError {
    /// Error allocating memory for the [`X86CommonAddressSpace`].
    OutOfMemory,
    /// The valid paging modes are invalid.
    NotSupported,
}

impl fmt::Display for X86CommonAddressSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating root page table"),
            Self::NotSupported => write!(f, "requested address space is not available"),
        }
    }
}

/// Implementations of [`AddressSpace`] for `x86_32` and `x86_64`.
pub enum X86CommonAddressSpace {
    /// `32-bit` paging implementation.
    Bits32(Bits32AddressSpace),
    /// `PAE` paging implementation.
    Pae(PaeAddressSpace),
    /// Long mode paging implementations.
    LongMode(LongModeAddressSpace),
}

impl X86CommonAddressSpace {
    /// Returns the value of the `CR3` register required to utilize this [`AddressSpace`].
    pub fn cr3(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.cr3(),
            Self::Pae(pae) => pae.cr3(),
            Self::LongMode(long_mode) => long_mode.cr3(),
        }
    }
}

impl AddressSpace for X86CommonAddressSpace {
    fn page_size(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.page_size(),
            Self::Pae(pae) => pae.page_size(),
            Self::LongMode(long_mode) => long_mode.page_size(),
        }
    }

    fn max_virtual_address(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.max_virtual_address(),
            Self::Pae(pae) => pae.max_virtual_address(),
            Self::LongMode(long_mode) => long_mode.max_virtual_address(),
        }
    }

    fn max_physical_address(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.max_physical_address(),
            Self::Pae(pae) => pae.max_physical_address(),
            Self::LongMode(long_mode) => long_mode.max_physical_address(),
        }
    }

    fn map(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        count: u64,
        protection: ProtectionFlags,
    ) -> Result<(), MapError> {
        match self {
            Self::Bits32(bits_32) => {
                bits_32.map(virtual_address, physical_address, count, protection)
            }
            Self::Pae(pae) => pae.map(virtual_address, physical_address, count, protection),
            Self::LongMode(long_mode) => {
                long_mode.map(virtual_address, physical_address, count, protection)
            }
        }
    }

    unsafe fn unmap(&mut self, virtual_address: u64, count: u64) {
        match self {
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::Bits32(bits_32) => unsafe { bits_32.unmap(virtual_address, count) },
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::Pae(pae) => unsafe { pae.unmap(virtual_address, count) },
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::LongMode(long_mode) => unsafe { long_mode.unmap(virtual_address, count) },
        }
    }

    fn find_region(&self, count: u64) -> Result<u64, NotFound> {
        match self {
            Self::Bits32(bits_32) => bits_32.find_region(count),
            Self::Pae(pae) => pae.find_region(count),
            Self::LongMode(long_mode) => long_mode.find_region(count),
        }
    }

    fn translate_virt(&self, virtual_address: u64) -> Result<(u64, ProtectionFlags), NoMapping> {
        match self {
            Self::Bits32(bits_32) => bits_32.translate_virt(virtual_address),
            Self::Pae(pae) => pae.translate_virt(virtual_address),
            Self::LongMode(long_mode) => long_mode.translate_virt(virtual_address),
        }
    }
}

//! Paging structures that are common to `x86_32` and `x86_64`.

use core::fmt;

use elf::header::Machine;
use memory::{
    address::{Address, AddressChunkRange, AddressSpaceDescriptor},
    phys::PhysicalMemorySpace,
    translation::{MapError, MapFlags, TranslationScheme},
};
use x86_32::paging::{bits_32::Bits32Scheme, pae::PaeScheme};
use x86_64::paging::LongModeScheme;

use crate::{
    arch::{ArchScheme, ArchSchemeError},
    platform::StubPhysicalMemory,
};

/// Constructs a new [`ArchScheme`] that is compatible with the provided [`Machine`].
///
/// # Errors
///
/// - [`ArchSchemeError::NotSupported`]: Returned if the requested address spaces are not
///   supported.
/// - [`ArchSchemeError::OutOfMemory`]: Returned if an error occurred while constructing the
///   new [`ArchScheme`].
pub fn new_address_space(machine: Machine) -> Result<ArchScheme, ArchSchemeError> {
    let address_space = match machine {
        Machine::INTEL_386 => {
            if let Ok(pae) = PaeScheme::new_current(StubPhysicalMemory) {
                ArchScheme::Pae(pae)
            } else if let Ok(bits_32) = Bits32Scheme::new_current(read_u32_at, write_u32_at) {
                ArchScheme::Bits32(bits_32)
            } else {
                return Err(ArchSchemeError::NotSupported);
            }
        }
        Machine::X86_64 => ArchScheme::LongMode(
            LongModeScheme::new_current(read_u64_at, write_u64_at).map_err(
                |error| match error {
                    LongModeError::NotActive => ArchSchemeError::NotSupported,
                    LongModeError::OutOfMemory(_) => ArchSchemeError::OutOfMemory,
                },
            )?,
        ),
        _ => return Err(ArchSchemeError::NotSupported),
    };

    Ok(address_space)
}

/// Various errors that can occur while contructing a [`TranslationScheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X86CommonSchemeError {
    /// Error allocating memory for the [`X86CommonScheme`].
    OutOfMemory,
    /// The valid paging modes are invalid.
    NotSupported,
}

impl fmt::Display for X86CommonSchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating root page table"),
            Self::NotSupported => write!(f, "requested address space is not available"),
        }
    }
}

/// Implementations of [`TranslationScheme`] for `x86_32` and `x86_64`.
pub enum X86CommonScheme<M: PhysicalMemorySpace> {
    /// `32-bit` paging implementation.
    Bits32(Bits32Scheme<M>),
    /// `PAE` paging implementation.
    Pae(PaeScheme<M>),
    /// Long mode paging implementations.
    LongMode(LongModeScheme<M>),
}

impl<M: PhysicalMemorySpace> X86CommonScheme<M> {
    /// Returns the value of the `CR3` register required to utilize this [`TranslationScheme`].
    pub fn cr3(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.cr3(),
            Self::Pae(pae) => pae.cr3(),
            Self::LongMode(long_mode) => long_mode.cr3(),
        }
    }
}

// SAFETY:
//
// The wrapped [`TranslationScheme`] implementations are implemented according to the `x86_64`
// specification.
unsafe impl<M: PhysicalMemorySpace> TranslationScheme for X86CommonScheme<M> {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        match self {
            Self::Bits32(bits_32) => bits_32.input_descriptor(),
            Self::Pae(pae) => pae.input_descriptor(),
            Self::LongMode(long_mode) => long_mode.input_descriptor(),
        }
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        match self {
            Self::Bits32(bits_32) => bits_32.output_descriptor(),
            Self::Pae(pae) => pae.output_descriptor(),
            Self::LongMode(long_mode) => long_mode.output_descriptor(),
        }
    }

    fn chunk_size(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.chunk_size(),
            Self::Pae(pae) => pae.chunk_size(),
            Self::LongMode(long_mode) => long_mode.chunk_size(),
        }
    }

    unsafe fn map(
        &mut self,
        input: AddressChunkRange,
        output: AddressChunkRange,
        flags: MapFlags,
    ) -> Result<(), MapError> {
        match self {
            Self::Bits32(bits_32) => bits_32.map(input, output, flags),
            Self::Pae(pae) => pae.map(input, output, flags),
            Self::LongMode(long_mode) => long_mode.map(input, output, flags),
        }
    }

    unsafe fn unmap(&mut self, input: AddressChunkRange) {
        match self {
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::Bits32(bits_32) => unsafe { bits_32.unmap(input) },
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::Pae(pae) => unsafe { pae.unmap(input) },
            // SAFETY:
            //
            // The invariants of this function ensure that this call is correct since the provided
            // region is completely mapped and must not be accessed while it is unmapped.
            Self::LongMode(long_mode) => unsafe { long_mode.unmap(input) },
        }
    }

    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)> {
        match self {
            Self::Bits32(bits_32) => bits_32.translate_input(input),
            Self::Pae(pae) => pae.translate_input(input),
            Self::LongMode(long_mode) => long_mode.translate_input(input),
        }
    }
}

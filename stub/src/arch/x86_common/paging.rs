//! Paging structures that are common to `x86_32` and `x86_64`.

use core::fmt;

use elf::header::Machine;
use memory::{
    address::{Address, AddressChunkRange, AddressSpaceDescriptor},
    translation::{MapError, MapFlags, TranslationScheme},
};
use x86_32::paging::{bits_32::Bits32Scheme, pae::PaeScheme};
use x86_64::paging::{LongModeError, LongModeScheme};

use crate::{
    arch::paging::ArchScheme,
    platform::StubPhysicalMemory,
    util::{alloc_physical, dealloc_physical},
};

/// Implementations of [`TranslationScheme`] for `x86_32` and `x86_64`.
pub enum X86CommonScheme {
    /// `32-bit` paging implementation.
    Bits32(Bits32Scheme<StubPhysicalMemory>),
    /// `PAE` paging implementation.
    Pae(PaeScheme<StubPhysicalMemory>),
    /// Long mode paging implementations.
    LongMode(LongModeScheme<StubPhysicalMemory>),
}

impl X86CommonScheme {
    /// Constructs a new [`X86CommonScheme`] that is compatible with the provided [`Machine`].
    ///
    /// # Errors
    ///
    /// - [`X86CommonSchemeError::NotSupported`]: Returned if the requested address spaces are not
    ///   supported.
    /// - [`X86CommonSchemeError::OutOfMemory`]: Returned if an error occurred while constructing
    ///   the new [`X86CommonScheme`].
    pub fn max_supported(machine: Machine) -> Result<Self, X86CommonSchemeError> {
        let address_space = match machine {
            Machine::INTEL_386 => {
                if let Ok(pae) =
                    PaeScheme::max_supported(StubPhysicalMemory, alloc_physical, dealloc_physical)
                {
                    Self::Pae(pae)
                } else if let Ok(bits_32) = Bits32Scheme::max_supported(
                    StubPhysicalMemory,
                    alloc_physical,
                    dealloc_physical,
                ) {
                    Self::Bits32(bits_32)
                } else {
                    return Err(X86CommonSchemeError::NotSupported);
                }
            }
            Machine::X86_64 => ArchScheme::LongMode(
                LongModeScheme::max_supported(StubPhysicalMemory, alloc_physical, dealloc_physical)
                    .map_err(|error| match error {
                        LongModeError::NotActive => X86CommonSchemeError::NotSupported,
                        LongModeError::OutOfMemory => X86CommonSchemeError::OutOfMemory,
                    })?,
            ),
            _ => return Err(X86CommonSchemeError::NotSupported),
        };

        Ok(address_space)
    }

    /// Constructs a new [`X86CommonScheme`] by taking over the existing page tables referenced by
    /// `CR3`.
    ///
    /// # Errors
    ///
    /// - [`X86CommonSchemeError::NotSupported`]: Returned if the requested address spaces are not
    ///   supported.
    /// - [`X86CommonSchemeError::OutOfMemory`]: Never returned from this function.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`X86CommonScheme`] must have exclusive
    /// control over the memory making up the page tables.
    pub unsafe fn active_current() -> Result<Self, X86CommonSchemeError> {
        // SAFETY:
        //
        // The invariants of [`Self::active_current()`] ensure that the invariants of
        // [`LongModeScheme::active_current()`] hold.
        let scheme = unsafe {
            LongModeScheme::active_current(StubPhysicalMemory, alloc_physical, dealloc_physical)
        };

        if let Ok(scheme) = scheme {
            return Ok(Self::LongMode(scheme));
        }

        // SAFETY:
        //
        // The invariants of [`Self::active_current()`] ensure that the invariants of
        // [`PaeScheme::active_current()`] hold.
        let scheme = unsafe {
            PaeScheme::active_current(StubPhysicalMemory, alloc_physical, dealloc_physical)
        };

        if let Ok(scheme) = scheme {
            return Ok(Self::Pae(scheme));
        }

        // SAFETY:
        //
        // The invariants of [`Self::active_current()`] ensure that the invariants of
        // [`Bits32Scheme::active_current()`] hold.
        let scheme = unsafe {
            Bits32Scheme::active_current(StubPhysicalMemory, alloc_physical, dealloc_physical)
        };

        if let Ok(scheme) = scheme {
            return Ok(Self::Bits32(scheme));
        }

        Err(X86CommonSchemeError::NotSupported)
    }

    /// Returns the value of the `CR3` register required to utilize this [`TranslationScheme`].
    pub fn cr3(&self) -> u64 {
        match self {
            Self::Bits32(bits_32) => bits_32.cr3(),
            Self::Pae(pae) => pae.cr3(),
            Self::LongMode(long_mode) => long_mode.cr3(),
        }
    }

    /// Returns `true` if this page table configuration requires the `LA57` bit to be set.
    pub fn la57(&self) -> bool {
        match self {
            Self::Bits32(_) => false,
            Self::Pae(_) => false,
            Self::LongMode(long_mode) => long_mode.la57(),
        }
    }

    /// Returns `true` if this page table configuration is a long mode configuration.
    pub fn long_mode(&self) -> bool {
        match self {
            Self::Bits32(_) => false,
            Self::Pae(_) => false,
            Self::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `PAE` bit to be set.
    pub fn pae(&self) -> bool {
        match self {
            Self::Bits32(_) => false,
            Self::Pae(_) => true,
            Self::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `PSE` bit to be set.
    pub fn pse(&self) -> bool {
        match self {
            Self::Bits32(bits_32) => bits_32.pse(),
            Self::Pae(_) => true,
            Self::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `NXE` bit to be set.
    pub fn nxe(&self) -> bool {
        match self {
            Self::Bits32(_) => false,
            Self::Pae(pae) => pae.nxe(),
            Self::LongMode(long_mode) => long_mode.nxe(),
        }
    }
}

// SAFETY:
//
// The wrapped [`TranslationScheme`] implementations are implemented according to the `x86_64`
// specification.
unsafe impl TranslationScheme for X86CommonScheme {
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
            // SAFETY:
            //
            // The invariants of this function ensure that the called function is safe to call.
            Self::Bits32(bits_32) => unsafe { bits_32.map(input, output, flags) },
            // SAFETY:
            //
            // The invariants of this function ensure that the called function is safe to call.
            Self::Pae(pae) => unsafe { pae.map(input, output, flags) },
            // SAFETY:
            //
            // The invariants of this function ensure that the called function is safe to call.
            Self::LongMode(long_mode) => unsafe { long_mode.map(input, output, flags) },
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

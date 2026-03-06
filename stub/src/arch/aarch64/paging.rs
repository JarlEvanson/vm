//! `aarch64` paging structures.

use core::{error, fmt};

use aarch64::{
    common::{Granule, PhysicalAddressSpaceSize},
    paging::vmsa_v8::{VmsaV8Error, VmsaV8Scheme},
};
use elf::header::Machine;
use memory::{
    address::{Address, AddressChunkRange, AddressSpaceDescriptor},
    translation::{MapError, MapFlags, TranslationScheme},
};

use crate::{
    platform::StubPhysicalMemory,
    util::{alloc_physical, dealloc_physical},
};

/// Implementations of [`TranslationScheme`] for `aarch64`.
pub enum Aarch64Scheme {
    /// Implementation of [`TranslationScheme`] for the VMSA V8 format.
    VmsaV8(VmsaV8Scheme<StubPhysicalMemory>),
}

impl Aarch64Scheme {
    /// Constructs a new [`Aarch64Scheme`] that is compatible with the provided [`Machine`].
    ///
    /// # Errors
    ///
    /// - [`Aarch64SchemeError::NotSupported`]: Returned if the requested address spaces are not
    ///   supported.
    pub fn max_supported(machine: Machine) -> Result<Self, Aarch64SchemeError> {
        if machine != Machine::AARCH64 {
            return Err(Aarch64SchemeError::NotSupported);
        }

        Ok(Aarch64Scheme::VmsaV8(VmsaV8Scheme::max_supported(
            StubPhysicalMemory,
            alloc_physical,
            dealloc_physical,
        )?))
    }

    /// Constructs a new [`Aarch64Scheme`] by taking over the existing page tables.
    ///
    /// # Errors
    ///
    /// - [`Aarch64SchemeError::NotSupported`]: Returned if the requested address spaces are not
    ///   supported.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`Aarch64Scheme`] must have exclusive
    /// control over the memory making up the page tables.
    pub unsafe fn active_current() -> Result<Self, Aarch64SchemeError> {
        // SAFETY:
        //
        // The invariants of [`Aarch64Scheme::active_current()`] ensure that the invariants of
        // [`VmsaV8Scheme::active_current()`] are met.
        unsafe {
            Ok(Aarch64Scheme::VmsaV8(VmsaV8Scheme::active_current(
                StubPhysicalMemory,
                alloc_physical,
                dealloc_physical,
            )?))
        }
    }

    /// Returns the size, in bytes, of minimal translation region.
    pub fn granule(&self) -> Granule {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.granule(),
        }
    }

    /// Returns `true` if the `TTBR0` page tables are in use.
    pub fn ttbr0_enabled(&self) -> bool {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.ttbr0_enabled(),
        }
    }

    /// Returns `true` if the `TTBR1` page tables are in use.
    pub fn ttbr1_enabled(&self) -> bool {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.ttbr1_enabled(),
        }
    }

    /// Returns the location of the table referenced by `TTBR0`.
    pub fn ttbr0(&self) -> u64 {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.ttbr0(),
        }
    }

    /// Returns the location of the table referenced by `TTBR1`.
    pub fn ttbr1(&self) -> u64 {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.ttbr1(),
        }
    }

    /// Returns the value of `T0SZ`.
    pub fn t0sz(&self) -> u8 {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.t0sz(),
        }
    }

    /// Returns the value of `T1SZ`.
    pub fn t1sz(&self) -> u8 {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.t1sz(),
        }
    }

    /// Returns the size, in bits, of the output address.
    pub fn ipa(&self) -> PhysicalAddressSpaceSize {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.ipa(),
        }
    }
}

// SAFETY:
//
// The wrapped [`TranslationScheme`] implementation is implemented according to the `aarch64`
// specification.
unsafe impl TranslationScheme for Aarch64Scheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.input_descriptor(),
        }
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.output_descriptor(),
        }
    }

    fn chunk_size(&self) -> u64 {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.chunk_size(),
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
            // The invariants of this function ensure that the invariants of
            // [`VmsaV8Scheme::map()`] are upheld.
            Self::VmsaV8(vmsa_v8) => unsafe { vmsa_v8.map(input, output, flags) },
        }
    }

    unsafe fn unmap(&mut self, input: AddressChunkRange) {
        match self {
            // SAFETY:
            //
            // The invariants of this function ensure that the invariants of
            // [`VmsaV8Scheme::unmap()`] are upheld.
            Self::VmsaV8(vmsa_v8) => unsafe { vmsa_v8.unmap(input) },
        }
    }

    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)> {
        match self {
            Self::VmsaV8(vmsa_v8) => vmsa_v8.translate_input(input),
        }
    }
}

/// Various errors that can occur when creating or taking over an [`Aarch64Scheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aarch64SchemeError {
    /// An error occurred when utilizing [`VmsaV8Scheme`].
    VmsaV8Error(VmsaV8Error),
    /// The operation is not supported.
    NotSupported,
}

impl From<VmsaV8Error> for Aarch64SchemeError {
    fn from(error: VmsaV8Error) -> Self {
        Self::VmsaV8Error(error)
    }
}

impl fmt::Display for Aarch64SchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VmsaV8Error(error) => write!(f, "error interacting with VMSAv8 tables: {error}"),
            Self::NotSupported => f.pad("the requested operation is not available"),
        }
    }
}

impl error::Error for Aarch64SchemeError {}

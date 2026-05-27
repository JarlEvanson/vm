//! Paging implementation for `aarch64`.

mod vmsa_v8;

use aarch64::{Granule, PhysicalAddressSpaceSize};
use elf::header::Machine;
use memory::AddressSpaceDescriptor;

use crate::{
    arch::{
        aarch64::memory::paging::vmsa_v8::VmsaV8TranslationScheme,
        generic::memory::paging::{
            ExternalFrameRange, ExternalPageRange, ExternalPhysicalAddress, ExternalVirtualAddress,
            TranslationScheme,
        },
    },
    platform::{MapError, MappingType, Permissions},
};

/// Implementation of [`TranslationScheme`] for `aarch64` paging.
pub struct Aarch64TranslationScheme(Inner);

impl Aarch64TranslationScheme {
    /// Constructs a new [`Aarch64TranslationScheme`] that is compatible with the provided
    /// [`Machine`].
    pub fn max_supported(machine: Machine) -> Option<Self> {
        if machine != Machine::AARCH64 {
            return None;
        }

        VmsaV8TranslationScheme::max_supported().map(|scheme| Self(Inner::VmsaV8(scheme)))
    }

    /// Constructs a new [`Aarch64TranslationScheme`] by taking over the existing page tables.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`Aarch64TranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        // SAFETY:
        //
        // The invariants of [`Aarch64TranslationScheme::active_current()`] ensure that the
        // invariants of [`VmsaV8Scheme::active_current()`] are met.
        unsafe {
            VmsaV8TranslationScheme::active_current().map(|scheme| Self(Inner::VmsaV8(scheme)))
        }
    }

    /// Returns the size, in bytes, of minimal translation region.
    pub fn granule(&self) -> Granule {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.granule(),
        }
    }

    /// Returns `true` if the `TTBR0` page tables are in use.
    pub fn ttbr0_enabled(&self) -> bool {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.ttbr0_enabled(),
        }
    }

    /// Returns `true` if the `TTBR1` page tables are in use.
    pub fn ttbr1_enabled(&self) -> bool {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.ttbr1_enabled(),
        }
    }

    /// Returns the location of the table referenced by `TTBR0`.
    pub fn ttbr0(&self) -> u64 {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.ttbr0(),
        }
    }

    /// Returns the location of the table referenced by `TTBR1`.
    pub fn ttbr1(&self) -> u64 {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.ttbr1(),
        }
    }

    /// Returns the value of `T0SZ`.
    pub fn t0sz(&self) -> u8 {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.t0sz(),
        }
    }

    /// Returns the value of `T1SZ`.
    pub fn t1sz(&self) -> u8 {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.t1sz(),
        }
    }

    /// Returns the size, in bits, of the output address.
    pub fn ipa(&self) -> PhysicalAddressSpaceSize {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.ipa(),
        }
    }
}

impl TranslationScheme for Aarch64TranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.input_descriptor(),
        }
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.output_descriptor(),
        }
    }

    fn chunk_size(&self) -> u64 {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.chunk_size(),
        }
    }

    fn map_at(
        &mut self,
        input: ExternalPageRange,
        output: ExternalFrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<(), MapError> {
        match &mut self.0 {
            Inner::VmsaV8(scheme) => scheme.map_at(input, output, permissions, mapping_type),
        }
    }

    unsafe fn unmap(&mut self, input: ExternalPageRange) {
        match &mut self.0 {
            // SAFETY:
            //
            // The invariants of the outer function ensure the inner function is safe.
            Inner::VmsaV8(scheme) => unsafe { scheme.unmap(input) },
        }
    }

    fn translate(
        &self,
        address: ExternalVirtualAddress,
    ) -> Option<(Permissions, MappingType, ExternalPhysicalAddress)> {
        match &self.0 {
            Inner::VmsaV8(scheme) => scheme.translate(address),
        }
    }
}

/// Internal type.
enum Inner {
    /// VMSAv8.
    VmsaV8(VmsaV8TranslationScheme),
}

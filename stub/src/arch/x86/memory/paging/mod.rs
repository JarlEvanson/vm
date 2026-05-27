//! Paging implementation for `i686` and `x86_64`.

use elf::header::Machine;
use memory::AddressSpaceDescriptor;
use x86::paging::{PagingMode, current_paging_mode};

use crate::{
    arch::{
        generic::memory::paging::{
            ExternalFrameRange, ExternalPageRange, ExternalPhysicalAddress, ExternalVirtualAddress,
            TranslationScheme,
        },
        x86::memory::paging::{
            bits_32::Bits32TranslationScheme, long_mode::LongModeTranslationScheme,
            pae::PaeTranslationScheme,
        },
    },
    platform::{MapError, MappingType, Permissions},
};

mod bits_32;
mod long_mode;
mod pae;

/// Implementation of [`TranslationScheme`] for `i686` and `x86_64` paging.
pub struct X86TranslationScheme(Inner);

impl X86TranslationScheme {
    /// Constructs a new [`X86TranslationScheme`] that is compatible with the provided [`Machine`].
    pub fn max_supported(machine: Machine) -> Option<Self> {
        let address_space = match machine {
            Machine::INTEL_386 => {
                if let Some(pae) = PaeTranslationScheme::max_supported() {
                    Self(Inner::Pae(pae))
                } else {
                    let bits_32 = Bits32TranslationScheme::max_supported()?;
                    Self(Inner::Bits32(bits_32))
                }
            }
            Machine::X86_64 => {
                let long_mode = LongModeTranslationScheme::max_supported()?;
                Self(Inner::LongMode(long_mode))
            }
            _ => return None,
        };

        Some(address_space)
    }

    /// Constructs a new [`X86TranslationScheme`] by taking over the existing page tables.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`X86TranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        match current_paging_mode() {
            PagingMode::Disabled => unimplemented!(),
            // SAFETY:
            //
            // The invariants of [`X86TranslationScheme::active_current()`] ensure that the
            // invariants of [`Bits32TranslationScheme::active_current()`] are fulfilled.
            PagingMode::Bits32 => unsafe {
                Bits32TranslationScheme::active_current().map(|scheme| Self(Inner::Bits32(scheme)))
            },
            // SAFETY:
            //
            // The invariants of [`X86TranslationScheme::active_current()`] ensure that the
            // invariants of [`PaeTranslationScheme::active_current()`] are fulfilled.
            PagingMode::Pae => unsafe {
                PaeTranslationScheme::active_current().map(|scheme| Self(Inner::Pae(scheme)))
            },
            // SAFETY:
            //
            // The invariants of [`X86TranslationScheme::active_current()`] ensure that the
            // invariants of [`LongModeTranslationScheme::active_current()`] are fulfilled.
            PagingMode::Level4 | PagingMode::Level5 => unsafe {
                LongModeTranslationScheme::active_current()
                    .map(|scheme| Self(Inner::LongMode(scheme)))
            },
        }
    }

    /// Returns the value of the `CR3` register required to utilize this [`TranslationScheme`].
    pub fn cr3(&self) -> u64 {
        match &self.0 {
            Inner::Bits32(bits_32) => bits_32.cr3(),
            Inner::Pae(pae) => pae.cr3(),
            Inner::LongMode(long_mode) => long_mode.cr3(),
        }
    }

    /// Returns `true` if this page table configuration requires the `LA57` bit to be set.
    pub fn la57(&self) -> bool {
        match &self.0 {
            Inner::Bits32(_) => false,
            Inner::Pae(_) => false,
            Inner::LongMode(long_mode) => long_mode.la57(),
        }
    }

    /// Returns `true` if this page table configuration is a long mode configuration.
    pub fn long_mode(&self) -> bool {
        match &self.0 {
            Inner::Bits32(_) => false,
            Inner::Pae(_) => false,
            Inner::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `PAE` bit to be set.
    pub fn pae(&self) -> bool {
        match &self.0 {
            Inner::Bits32(_) => false,
            Inner::Pae(_) => true,
            Inner::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `PSE` bit to be set.
    pub fn pse(&self) -> bool {
        match &self.0 {
            Inner::Bits32(bits_32) => bits_32.pse(),
            Inner::Pae(_) => true,
            Inner::LongMode(_) => true,
        }
    }

    /// Returns `true` if this page table configuration requires the `NXE` bit to be set.
    pub fn nxe(&self) -> bool {
        match &self.0 {
            Inner::Bits32(_) => false,
            Inner::Pae(pae) => pae.nxe(),
            Inner::LongMode(long_mode) => long_mode.nxe(),
        }
    }
}

impl TranslationScheme for X86TranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        match &self.0 {
            Inner::Bits32(scheme) => scheme.input_descriptor(),
            Inner::Pae(scheme) => scheme.input_descriptor(),
            Inner::LongMode(scheme) => scheme.input_descriptor(),
        }
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        match &self.0 {
            Inner::Bits32(scheme) => scheme.output_descriptor(),
            Inner::Pae(scheme) => scheme.output_descriptor(),
            Inner::LongMode(scheme) => scheme.output_descriptor(),
        }
    }

    fn chunk_size(&self) -> u64 {
        match &self.0 {
            Inner::Bits32(scheme) => scheme.chunk_size(),
            Inner::Pae(scheme) => scheme.chunk_size(),
            Inner::LongMode(scheme) => scheme.chunk_size(),
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
            Inner::Bits32(scheme) => scheme.map_at(input, output, permissions, mapping_type),
            Inner::Pae(scheme) => scheme.map_at(input, output, permissions, mapping_type),
            Inner::LongMode(scheme) => scheme.map_at(input, output, permissions, mapping_type),
        }
    }

    unsafe fn unmap(&mut self, input: ExternalPageRange) {
        match &mut self.0 {
            // SAFETY:
            //
            // The invariants of the outer function ensure the inner function is safe.
            Inner::Bits32(scheme) => unsafe { scheme.unmap(input) },
            // SAFETY:
            //
            // The invariants of the outer function ensure the inner function is safe.
            Inner::Pae(scheme) => unsafe { scheme.unmap(input) },
            // SAFETY:
            //
            // The invariants of the outer function ensure the inner function is safe.
            Inner::LongMode(scheme) => unsafe { scheme.unmap(input) },
        }
    }

    fn translate(
        &self,
        address: ExternalVirtualAddress,
    ) -> Option<(Permissions, MappingType, ExternalPhysicalAddress)> {
        match &self.0 {
            Inner::Bits32(scheme) => scheme.translate(address),
            Inner::Pae(scheme) => scheme.translate(address),
            Inner::LongMode(scheme) => scheme.translate(address),
        }
    }
}

/// Internal type.
enum Inner {
    /// 32-bit paging.
    Bits32(Bits32TranslationScheme),
    /// PAE (Physical Address Extension) paging.
    Pae(PaeTranslationScheme),
    /// 64-bit paging.
    LongMode(LongModeTranslationScheme),
}

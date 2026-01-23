//! The `elf` crate provides an interface for reading ELF files.
//!
//! # Capabilities
//!
//! ## Works in `no_std` environments
//!
//! This crate provides an ELF file parsing interface which does not allocate or use any `std`
//! features, so it can be used in `no_std` contexts such as bootloaders, kernels, or hypervisors.
//!
//! ## Endian Awareness
//!
//! This crate handles differences between host and file endianness when parsing the ELF file
//! structures and provides generic implementations intended to support various use cases.
//!
//! ## Class Awareness
//!
//! This crate handles differences between host and file class sizes when parsing the ELF file
//! structures and provides generic implementations intended to support various use cases.
//!
//! ## Zero-Alloc Parsing
//!
//! This crate implements parsing in such a manner that avoids heap allocations. ELF structures are
//! lazily parsed with iterators or tables that only parse the requested structure when required.
//!
//! ## Uses no unsafe code
//!
//! This crate contains zero unsafe blocks of code.
#![no_std]

use core::fmt;

use crate::{
    class::Class,
    encoding::Encoding,
    header::{ElfHeader, ElfHeaderError},
    medium::{Medium, MediumError},
    program_header::ProgramHeaderTable,
    section_header::SectionHeaderTable,
};

pub mod class;
pub mod dynamic;
pub mod encoding;
pub mod header;
pub mod ident;
pub mod medium;
pub mod program_header;
pub mod raw;
pub mod relocation;
pub mod section_header;
pub mod symbol;
pub mod table;

/// An ELF file.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Elf<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of this [`Elf`].
    medium: &'slice M,
    /// The [`Class`] used to decode this [`Elf`].
    class: C,
    /// The [`Encoding`] used to decode this [`Elf`].
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
#[expect(clippy::missing_panics_doc)]
#[expect(clippy::type_complexity)]
impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> Elf<'slice, M, C, E> {
    /// Creates a new [`Elf`] from the given `slice`.
    ///
    /// # Errors
    ///
    /// Returns various errors if the [`Class`][c] or [`Encoding`][e] are not supported or if the parsing
    /// would have gone out of bounds.
    ///
    /// [c]: crate::ident::Class
    /// [e]: crate::ident::Encoding
    pub fn new(medium: &'slice M) -> Result<Self, ElfHeaderError<M::Error>> {
        let header = ElfHeader::new(medium)?;

        Ok(Self {
            medium,
            class: header.class,
            encoding: header.encoding,
        })
    }

    /// Returns the [`ElfHeader`] of this [`Elf`].
    pub fn header(&self) -> ElfHeader<'slice, M, C, E> {
        ElfHeader {
            medium: self.medium,
            class: self.class,
            encoding: self.encoding,
        }
    }

    /// Returns the [`SectionHeaderTable`] of this [`Elf`].
    pub fn section_header_table(
        &self,
    ) -> Result<Option<SectionHeaderTable<'slice, M, C, E>>, MediumError<M::Error>> {
        let offset = self.header().section_header_offset()?.into();
        if offset == 0u64 {
            return Ok(None);
        }

        let size = u64::from(self.header().section_header_size()?);
        let count = if u64::from(self.header().section_header_count()?) != 0 {
            u64::from(self.header().section_header_count()?)
        } else {
            let table =
                SectionHeaderTable::new(self.class, self.encoding, self.medium, offset, 1, size)
                    .ok_or(MediumError::BoundsError {
                        offset,
                        length: size,
                        size: self.medium.size(),
                    })?;
            table.get(0).unwrap().size()?.into()
        };

        Ok(SectionHeaderTable::new(
            self.class,
            self.encoding,
            self.medium,
            offset,
            count,
            size,
        ))
    }

    /// Returns the [`ProgramHeaderTable`] of this [`Elf`].
    pub fn program_header_table(
        &self,
    ) -> Result<Option<ProgramHeaderTable<'slice, M, C, E>>, MediumError<M::Error>> {
        let offset = self.header().program_header_offset()?.into();
        if offset == 0u64 {
            return Ok(None);
        }

        let count = u64::from(self.header().program_header_count()?);
        let size = u64::from(self.header().program_header_size()?);
        Ok(ProgramHeaderTable::new(
            self.class,
            self.encoding,
            self.medium,
            offset,
            count,
            size,
        ))
    }
}

/// Safely converts `value` to a `u64` relying on compile time code checking.
#[expect(clippy::as_conversions, reason = "implementation of type-safe as cast")]
fn usize_to_u64(value: usize) -> u64 {
    #[cfg(not(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    )))]
    compile_error!("library supports only 16-bit, 32-bit, and 64-bit usize");
    value as u64
}

/// Safely converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions, reason = "implementation of type-safe as cast")]
fn u64_to_usize(value: u64) -> usize {
    #[cfg(not(any(target_pointer_width = "64")))]
    compile_error!("library supports only 16-bit, 32-bit, and 64-bit usize");
    value as usize
}

/// Safely extracts the target type or its error type.
fn extract_format<T: fmt::Debug, E: fmt::Debug>(result: &Result<T, E>) -> &dyn fmt::Debug {
    match result {
        Ok(value) => value,
        Err(error) => error,
    }
}

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

mod class;
mod encoding;
pub mod file_header;
pub mod ident;
pub mod program_header;
pub mod raw;
pub mod relocation;
pub mod section_header;
pub mod symbol;
pub mod table;

use core::fmt;

pub use encoding::{
    AnyEndian, BackedMedium, BigEndian, Encoding, LittleEndian, Medium, Merge as EncodingMerge,
    UnsupportedEncodingError,
};

pub use class::{
    AnyClass, Class, Class32, Class64, ClassBase, Merge as ClassMerge, UnsupportedClassError,
};

use crate::{
    file_header::{FileHeader, ParseFileHeaderError},
    program_header::ProgramHeaderTable,
    section_header::SectionHeaderTable,
};

/// An ELF file.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct File<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of this [`File`].
    medium: &'slice M,
    /// The [`Class`] used to decode this [`File`].
    class: C,
    /// The [`Encoding`] used to decode this [`File`].
    encoding: E,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> File<'slice, M, C, E> {
    /// Creates a new [`File`] from the given `slice`.
    ///
    /// # Errors
    ///
    /// Returns various errors if the `class` or `encoding` are not supported or if the parsing
    /// would have gone out of bounds.
    pub fn new(medium: &'slice M) -> Result<Self, ParseFileError> {
        let header = FileHeader::new(medium)?;

        let file = Self {
            medium,
            class: header.class(),
            encoding: header.encoding(),
        };

        Ok(file)
    }

    /// Returns the [`FileHeader`] of this [`File`].
    pub fn header(&self) -> FileHeader<'slice, M, C, E> {
        FileHeader {
            medium: self.medium,
            class: self.class,
            encoding: self.encoding,
        }
    }

    /// Returns the [`SectionHeaderTable`] of this [`File`].
    pub fn section_header_table(&self) -> Option<SectionHeaderTable<'slice, M, C, E>> {
        let offset = self.header().section_header_offset().into();
        if offset == 0u64 {
            return None;
        }

        let size = u64::from(self.header().section_header_size());
        let count = if u64::from(self.header().section_header_count()) != 0 {
            u64::from(self.header().section_header_count())
        } else {
            let table =
                SectionHeaderTable::new(self.class, self.encoding, self.medium, offset, 1, size)?;
            table.get(0)?.size().into()
        };

        SectionHeaderTable::new(self.class, self.encoding, self.medium, offset, count, size)
    }

    /// Returns the [`ProgramHeaderTable`] of this [`File`].
    pub fn program_header_table(&self) -> Option<ProgramHeaderTable<'slice, M, C, E>> {
        let offset = self.header().program_header_offset().into();
        if offset == 0u64 {
            return None;
        }

        let count = u64::from(self.header().program_header_count());
        let size = u64::from(self.header().program_header_size());
        ProgramHeaderTable::new(self.class, self.encoding, self.medium, offset, count, size)
    }
}

/// Various errors that can occur while parsing an [`File`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ParseFileError {
    /// An error cocured while parsing the [`FileHeader`].
    ParseFileHeaderError(ParseFileHeaderError),
}

impl From<ParseFileHeaderError> for ParseFileError {
    fn from(value: ParseFileHeaderError) -> Self {
        Self::ParseFileHeaderError(value)
    }
}

impl fmt::Display for ParseFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseFileHeaderError(error) => write!(f, "error parsing file header: {error}"),
        }
    }
}

impl core::error::Error for ParseFileError {}

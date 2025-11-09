//! Ergonoic wrapper over ELF file headers.

use core::{error, fmt};

use crate::{
    Class, ClassBase, Encoding, Medium, UnsupportedClassError, UnsupportedEncodingError,
    ident::{FileIdent, ValidateFileIdentSpecError},
};

/// Contains basic information about how an ELF file is arranged.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct FileHeader<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    pub(crate) medium: &'slice M,
    /// The [`Class`] used to decode the ELF file.
    pub(crate) class: C,
    /// The [`Encoding`] used to decode the ELF file.
    pub(crate) encoding: E,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> FileHeader<'slice, M, C, E> {
    /// Creates a new [`FileHeader`] from the given [`Medium`].
    ///
    /// # Errors
    /// - [`ParseFileHeaderError::TooSmall`]: Returned if `medium.size()` is too small to contain
    ///   an ELF header.
    /// - [`ParseFileHeaderError::UnsupportedClass`]: Returned if [`ident::Class`][c] of
    ///   [`FileIdent`] is not supported.
    /// - [`ParseFileHeaderError::UnsupportedEncoding`]: Returned if [`ident::Encoding`][e] of
    ///   [`FileIdent`] is not supported.
    ///
    /// [c]: crate::ident::Class
    /// [e]: crate::ident::Encoding
    pub fn new(medium: &'slice M) -> Result<Self, ParseFileHeaderError> {
        let ident = FileIdent::new(medium).ok_or(ParseFileHeaderError::TooSmall)?;
        let class = C::from_elf_class(ident.class())?;
        let encoding = E::from_elf_encoding(ident.encoding())?;

        if medium.size() < class.expected_elf_header_size() {
            return Err(ParseFileHeaderError::TooSmall);
        }

        let header = Self {
            medium,
            class,
            encoding,
        };

        Ok(header)
    }

    /// Validates that this [`FileIdent`] matches the ELF specification and is supported by this
    /// crate.
    ///
    /// # Errors
    /// - [`ValidateFileHeaderSpecError::IdentError`]: Returned when the [`FileIdent`] contained in
    ///   this [`FileHeader`] fails validaton.
    /// - [`ValidateFileHeaderSpecError::InvalidFileHeaderSize`]: Returned when the header size
    ///   provided by the ELF file less than the minimum for the [`Class`].
    pub fn validate_spec(&self) -> Result<(), ValidateFileHeaderSpecError> {
        self.ident().validate_spec()?;
        if u64::from(self.header_size()) < self.class.expected_elf_header_size() {
            return Err(ValidateFileHeaderSpecError::InvalidFileHeaderSize);
        }

        Ok(())
    }

    /// Returns the [`Class`] implementation of this [`FileHeader`].
    pub fn class(&self) -> C {
        self.class
    }

    /// Returns the [`Encoding`] implementation of this [`FileHeader`].
    pub fn encoding(&self) -> E {
        self.encoding
    }

    /// Returns the [`FileIdent`] associated with this [`FileHeader`].
    pub fn ident(&self) -> FileIdent<'slice, M> {
        FileIdent(self.medium)
    }

    /// Returns the [`FileKind`] associated with this [`FileHeader`].
    pub fn file_kind(&self) -> FileKind {
        FileKind(
            self.encoding
                .parse_u16(self.class.elf_kind_offset(), self.medium),
        )
    }

    /// Returns the architecture for which this ELF file is targeted.
    pub fn machine(&self) -> Machine {
        Machine(
            self.encoding
                .parse_u16(self.class.machine_offset(), self.medium),
        )
    }

    /// Returns the version of this ELF file.
    pub fn version(&self) -> u32 {
        self.encoding
            .parse_u32(self.class.version_offset(), self.medium)
    }

    /// Returns the processor specific flags associated with the ELF file.
    pub fn flags(&self) -> u32 {
        self.encoding
            .parse_u32(ClassFileHeader::flags_offset(self.class), self.medium)
    }

    /// Returns the size of the ELF file header in bytes.
    pub fn header_size(&self) -> u16 {
        self.encoding
            .parse_u16(self.class.header_size_offset(), self.medium)
    }

    /// Returns the virtual address of the entry point of this ELF file.
    pub fn entry(&self) -> C::ClassUsize {
        self.class
            .parse_class_usize(self.encoding, self.class.entry_offset(), self.medium)
    }

    /// Returns the program header table's file offset in bytes.
    pub fn program_header_offset(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.class.program_header_offset_offset(),
            self.medium,
        )
    }

    /// Returns the number of program headers in the program header table.
    pub fn program_header_count(&self) -> u16 {
        self.encoding
            .parse_u16(self.class.program_header_count_offset(), self.medium)
    }

    /// Return the size of each program header in the program header table.
    pub fn program_header_size(&self) -> u16 {
        self.encoding
            .parse_u16(self.class.program_header_size_offset(), self.medium)
    }

    /// Returns the section header table's file offset in bytes.
    pub fn section_header_offset(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.class.section_header_offset_offset(),
            self.medium,
        )
    }

    /// Returns the number of section headers in the section header table.
    pub fn section_header_count(&self) -> u16 {
        self.encoding
            .parse_u16(self.class.section_header_count_offset(), self.medium)
    }

    /// Return the size of each section header in the section header table.
    pub fn section_header_size(&self) -> u16 {
        self.encoding
            .parse_u16(self.class.section_header_size_offset(), self.medium)
    }

    /// Returns the index into the section header table to obtain the section name string table.
    pub fn section_header_string_table_index(&self) -> u16 {
        self.encoding.parse_u16(
            self.class.section_header_string_table_index_offset(),
            self.medium,
        )
    }
}

impl<M: Medium + ?Sized, C: Class, E: Encoding> fmt::Debug for FileHeader<'_, M, C, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("FileHeader");

        debug_struct.field("ident", &self.ident());
        debug_struct.field("machine", &self.machine());
        debug_struct.field("version", &self.version());
        debug_struct.field("flags", &self.flags());
        debug_struct.field("header_size", &self.header_size());
        debug_struct.field("entry", &self.entry());

        debug_struct.field("program_header_offset", &self.program_header_offset());
        debug_struct.field("program_header_count", &self.program_header_count());
        debug_struct.field("program_header_size", &self.program_header_size());

        debug_struct.field("section_header_offset", &self.section_header_offset());
        debug_struct.field("section_header_count", &self.section_header_count());
        debug_struct.field("section_header_size", &self.section_header_size());

        debug_struct.field(
            "section_header_string_table_index",
            &self.section_header_string_table_index(),
        );

        debug_struct.finish()
    }
}

/// Various errors that can occur while creating an [`FileHeader`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseFileHeaderError {
    /// The given slice is too small.
    TooSmall,
    /// The [`Class`][c] is unsupported.
    ///
    /// [c]: crate::ident::Class
    UnsupportedClass(UnsupportedClassError),
    /// The [`Encoding`][e] is unsupported.
    ///
    /// [e]: crate::ident::Encoding
    UnsupportedEncoding(UnsupportedEncodingError),
}

impl From<UnsupportedClassError> for ParseFileHeaderError {
    fn from(value: UnsupportedClassError) -> Self {
        Self::UnsupportedClass(value)
    }
}

impl From<UnsupportedEncodingError> for ParseFileHeaderError {
    fn from(value: UnsupportedEncodingError) -> Self {
        Self::UnsupportedEncoding(value)
    }
}

impl fmt::Display for ParseFileHeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall => f.pad("slice too small"),
            Self::UnsupportedClass(error) => fmt::Display::fmt(error, f),
            Self::UnsupportedEncoding(error) => fmt::Display::fmt(error, f),
        }
    }
}

/// Various errors that can occur when validating an [`FileHeader`] follows the ELF specification
/// and is supported by this crate.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidateFileHeaderSpecError {
    /// An error occured while validating the contained [`FileIdent`].
    IdentError(ValidateFileIdentSpecError),
    /// The size of the [`FileHeader`] is given as smaller than expected.
    InvalidFileHeaderSize,
}

impl From<ValidateFileIdentSpecError> for ValidateFileHeaderSpecError {
    fn from(value: ValidateFileIdentSpecError) -> Self {
        Self::IdentError(value)
    }
}

impl fmt::Display for ValidateFileHeaderSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentError(error) => write!(f, "error while parsing ELF identifier: {error}"),
            Self::InvalidFileHeaderSize => {
                write!(f, "given ELF header size is smaller than expected")
            }
        }
    }
}

impl error::Error for ValidateFileHeaderSpecError {}

/// The kind of the ELF file.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct FileKind(pub u16);

impl FileKind {
    /// No kind.
    pub const NONE: Self = Self(0);
    /// Relocatable ELF file.
    pub const RELOCATABLE: Self = Self(1);
    /// Executable ELF file.
    pub const EXECUTABLE: Self = Self(2);
    /// Shared object ELF file.
    pub const SHARED: Self = Self(3);
    /// Core ELF file.
    pub const CORE: Self = Self(4);
}

impl error::Error for ParseFileHeaderError {}

impl fmt::Debug for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("None"),
            Self::RELOCATABLE => f.pad("Relocatable"),
            Self::EXECUTABLE => f.pad("Executable"),
            Self::SHARED => f.pad("SharedObject"),
            Self::CORE => f.pad("Core"),
            elf_kind => f.debug_tuple("FileKind").field(&elf_kind.0).finish(),
        }
    }
}

/// The architecture of the ELF file.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Machine(pub u16);

impl Machine {
    /// No required machine.
    pub const NONE: Self = Self(0);
    /// ELF file requires the Intel 80386 architecture.
    pub const INTEL_386: Self = Self(3);
    /// ELF file requires the AArch32 architecture.
    pub const ARM: Self = Self(40);
    /// ELF file requires the AMD x86_64 architecture.
    pub const X86_64: Self = Self(62);
    /// ELF file requires the AArch64 architecture.
    pub const AARCH64: Self = Self(183);
}

impl fmt::Debug for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("None"),
            Self::INTEL_386 => f.pad("Intel386"),
            Self::ARM => f.pad("Aarch32"),
            Self::X86_64 => f.pad("x86_64"),
            Self::AARCH64 => f.pad("Aarch64"),
            machine => f.debug_tuple("Machine").field(&machine.0).finish(),
        }
    }
}

/// The definitions required to implement class aware parsing of ELF file headers.
pub trait ClassFileHeader: ClassBase {
    /// The offset of the kind field.
    fn elf_kind_offset(self) -> u64;
    /// The offset of the machine field.
    fn machine_offset(self) -> u64;
    /// The offset of the version field.
    fn version_offset(self) -> u64;
    /// The offset of the entry field.
    fn entry_offset(self) -> u64;
    /// The offset of the flags field.
    fn flags_offset(self) -> u64;
    /// The offset of the header size field.
    fn header_size_offset(self) -> u64;
    /// The offset of the program header offset field.
    fn program_header_offset_offset(self) -> u64;
    /// The offset of the program header count field.
    fn program_header_count_offset(self) -> u64;
    /// The offset of the program header size field.
    fn program_header_size_offset(self) -> u64;
    /// The offset of the section header offset field.
    fn section_header_offset_offset(self) -> u64;
    /// The offset of the section header count field.
    fn section_header_count_offset(self) -> u64;
    /// The offset of the section header size field.
    fn section_header_size_offset(self) -> u64;
    /// The offset of the section header string table index field.
    fn section_header_string_table_index_offset(self) -> u64;

    /// The expected size of an ELF file header.
    fn expected_elf_header_size(self) -> u64;
}

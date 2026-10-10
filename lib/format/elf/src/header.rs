//! Ergonomic wrapper over ELF headers.

use core::{error, fmt};

use crate::{
    class::{ClassBase, UnsupportedClassError},
    encoding::{Encoding, UnsupportedEncodingError},
    extract_format,
    ident::{ElfIdent, ElfIdentValidationError},
    medium::{Medium, MediumError},
};

/// Contains basic information about how an ELF file is arranged.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ElfHeader<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    pub(crate) medium: &'slice M,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    pub(crate) class: C,
    /// The [`Encoding`] used to decode the ELF file.
    pub(crate) encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassElfHeader, E: Encoding> ElfHeader<'slice, M, C, E> {
    /// The current version of the ELF file header.
    pub const CURRENT_FILE_VERSION: u32 = 1;

    /// Creates a new [`ElfHeader`] from the given [`Medium`].
    ///
    /// # Errors
    ///
    /// - [`ElfHeaderError::UnsupportedClass`]: Returned when the [`Class`][c] of the ELF file is
    ///   not supported.
    /// - [`ElfHeaderError::UnsupportedEncoding`]: Returned when the [`Encoding`][e] of the ELF
    ///   file is not supported.
    /// - [`ElfHeaderError::MediumError`]: Returned when an error occurs while interacting with the
    ///   underlying [`Medium`] or if the [`Medium`] is too small to contain an [`ElfHeader`].
    ///
    /// [c]: crate::ident::Class
    /// [e]: crate::ident::Encoding
    pub fn new(medium: &'slice M) -> Result<Self, ElfHeaderError<M::Error>> {
        let ident = ElfIdent::new(medium)?;
        let class = C::from_elf_class(ident.class()?)?;
        let encoding = E::from_elf_encoding(ident.encoding()?)?;

        if medium.size() < class.expected_elf_header_size() {
            // If the [`Medium`] isn't large enough to contain an [`ElfHeader`], then return a
            // [`MediumError::BoundsError`].
            return Err(ElfHeaderError::MediumError(MediumError::BoundsError {
                offset: 0,
                length: class.expected_elf_header_size(),
                size: medium.size(),
            }));
        }

        let header = Self {
            medium,
            class,
            encoding,
        };
        Ok(header)
    }

    /// Validates that the [`ElfHeader`] matches the ELF specification.
    ///
    /// # Errors
    ///
    /// - [`ElfHeaderValidationError::ElfIdentValidationError`]: Returned if
    ///   [`ElfIdent::validate()`] returns an error.
    /// - [`ElfHeaderValidationError::UnsupportedElfFileVersion`]: Returned if the version of the
    ///   ELF file is not supported.
    /// - [`ElfHeaderValidationError::InvalidElfHeaderSize`]: Returned if the specified size of the
    ///   [`ElfHeader`] is too small.
    pub fn validate(&self) -> Result<(), ElfHeaderValidationError<M::Error>> {
        self.ident()?.validate()?;

        let version = self.version()?;
        if version != Self::CURRENT_FILE_VERSION {
            return Err(ElfHeaderValidationError::UnsupportedElfFileVersion(version));
        }

        let header_size = self.header_size()?;
        if u64::from(header_size) < self.class.expected_elf_header_size() {
            return Err(ElfHeaderValidationError::InvalidElfHeaderSize(header_size));
        }

        Ok(())
    }

    /// Returns the [`ElfIdent`] associated with this [`ElfHeader`].
    pub fn ident(&self) -> Result<ElfIdent<'slice, M>, MediumError<M::Error>> {
        ElfIdent::new(self.medium)
    }

    /// Returns the [`ElfType`] associated with this [`ElfHeader`].
    pub fn elf_type(&self) -> Result<ElfType, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.elf_type_offset(), self.medium)
            .map(ElfType)
    }

    /// Returns the architecture for which this ELF file is targeted.
    pub fn machine(&self) -> Result<Machine, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.machine_offset(), self.medium)
            .map(Machine)
    }

    /// Returns the version of this ELF file.
    pub fn version(&self) -> Result<u32, MediumError<M::Error>> {
        self.encoding
            .read_u32(self.class.version_offset(), self.medium)
    }

    /// Returns the processor specific flags associated with the ELF file.
    pub fn flags(&self) -> Result<u32, MediumError<M::Error>> {
        self.encoding
            .read_u32(self.class.flags_offset(), self.medium)
    }

    /// Returns the size of the ELF file header in bytes.
    pub fn header_size(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.header_size_offset(), self.medium)
    }

    /// Returns the virtual address of the entry point of this ELF file.
    pub fn entry(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class
            .read_class_usize(self.encoding, self.class.entry_offset(), self.medium)
    }

    /// Returns the program header table's file offset in bytes.
    pub fn program_header_offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.class.program_header_offset_offset(),
            self.medium,
        )
    }

    /// Returns the number of program headers in the program header table.
    pub fn program_header_count(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.program_header_count_offset(), self.medium)
    }

    /// Returns the size of each program header in the program header table.
    pub fn program_header_size(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.program_header_size_offset(), self.medium)
    }

    /// Returns the section header table's file offset in bytes.
    pub fn section_header_offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.class.section_header_offset_offset(),
            self.medium,
        )
    }

    /// Returns the number of section headers in the section header table.
    pub fn section_header_count(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.section_header_count_offset(), self.medium)
    }

    /// Returns the size of each section header in the section header table.
    pub fn section_header_size(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding
            .read_u16(self.class.section_header_size_offset(), self.medium)
    }

    /// Returns the index into the section header table to obtain the section name string table.
    pub fn section_header_string_table_index(&self) -> Result<u16, MediumError<M::Error>> {
        self.encoding.read_u16(
            self.class.section_header_string_table_index_offset(),
            self.medium,
        )
    }

    /// Returns the underlying [`Medium`].
    pub fn medium(&self) -> &M {
        self.medium
    }

    /// Returns the [`Class`][crate::class::Class] implementation of this [`ElfHeader`].
    pub fn class(&self) -> C {
        self.class
    }

    /// Returns the [`Encoding`] implementation of this [`ElfHeader`].
    pub fn encoding(&self) -> E {
        self.encoding
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassElfHeader, E: Encoding> fmt::Debug
    for ElfHeader<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ident = self.ident();
        let elf_type = self.elf_type();
        let machine = self.machine();
        let version = self.version();
        let flags = self.flags();
        let header_size = self.header_size();
        let entry = self.entry();

        let program_header_offset = self.program_header_offset();
        let program_header_count = self.program_header_count();
        let program_header_size = self.program_header_size();

        let section_header_offset = self.section_header_offset();
        let section_header_count = self.section_header_count();
        let section_header_size = self.section_header_size();

        let section_header_string_table_index = self.section_header_string_table_index();

        f.debug_struct("ElfHeader")
            .field("ident", extract_format(&ident))
            .field("type", extract_format(&elf_type))
            .field("machine", extract_format(&machine))
            .field("version", extract_format(&version))
            .field("flags", extract_format(&flags))
            .field("header_size", extract_format(&header_size))
            .field("entry", extract_format(&entry))
            .field(
                "program_header_offset",
                extract_format(&program_header_offset),
            )
            .field(
                "program_header_count",
                extract_format(&program_header_count),
            )
            .field("program_header_size", extract_format(&program_header_size))
            .field(
                "section_header_offset",
                extract_format(&section_header_offset),
            )
            .field(
                "section_header_count",
                extract_format(&section_header_count),
            )
            .field("section_header_size", extract_format(&section_header_size))
            .field(
                "section_header_string_table_index",
                extract_format(&section_header_string_table_index),
            )
            .finish()
    }
}

/// Various errors that can occur when creating a new [`ElfHeader`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfHeaderError<E> {
    /// The [`Class`][crate::ident::Class] of the ELF file is not supported.
    UnsupportedClass(UnsupportedClassError),
    /// The [`Encoding`][crate::ident::Encoding] of the ELF file is not supported.
    UnsupportedEncoding(UnsupportedEncodingError),
    /// An error occurred when interacting with the underlying [`Medium`].
    MediumError(MediumError<E>),
}

impl<E> From<UnsupportedClassError> for ElfHeaderError<E> {
    fn from(value: UnsupportedClassError) -> Self {
        Self::UnsupportedClass(value)
    }
}

impl<E> From<UnsupportedEncodingError> for ElfHeaderError<E> {
    fn from(value: UnsupportedEncodingError) -> Self {
        Self::UnsupportedEncoding(value)
    }
}

impl<E> From<MediumError<E>> for ElfHeaderError<E> {
    fn from(value: MediumError<E>) -> Self {
        Self::MediumError(value)
    }
}

impl<E: fmt::Display> fmt::Display for ElfHeaderError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedClass(error) => fmt::Display::fmt(error, f),
            Self::UnsupportedEncoding(error) => fmt::Display::fmt(error, f),
            Self::MediumError(error) => write!(f, "error accessing ELF header bytes: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for ElfHeaderError<E> {}

/// Various errors that can occur when validating that an [`ElfHeader`] follows the ELF specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfHeaderValidationError<E> {
    /// Various errors that can occur when validating that an [`ElfIdent`] follows the ELF
    /// specification.
    ElfIdentValidationError(ElfIdentValidationError<E>),
    /// The ELF file version is not supported.
    UnsupportedElfFileVersion(u32),
    /// The specified size of the [`ElfHeader`] is smaller than the specification allows.
    InvalidElfHeaderSize(u16),
    /// Various errors that can occur when accessing the underlying [`Medium`].
    MediumError(MediumError<E>),
}

impl<E> From<MediumError<E>> for ElfHeaderValidationError<E> {
    fn from(value: MediumError<E>) -> Self {
        Self::MediumError(value)
    }
}

impl<E> From<ElfIdentValidationError<E>> for ElfHeaderValidationError<E> {
    fn from(value: ElfIdentValidationError<E>) -> Self {
        ElfHeaderValidationError::ElfIdentValidationError(value)
    }
}

impl<E: fmt::Display> fmt::Display for ElfHeaderValidationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElfIdentValidationError(error) => {
                write!(f, "error validating ELF ident: {error}")
            }
            Self::UnsupportedElfFileVersion(version) => {
                write!(f, "unsupported ELF file version: {version}")
            }
            Self::InvalidElfHeaderSize(size) => {
                write!(f, "specified size of ELF header is too small: {size}")
            }
            Self::MediumError(error) => write!(f, "error accessing ELF header bytes: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for ElfHeaderValidationError<E> {}

/// The type of the ELF file.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ElfType(pub u16);

impl ElfType {
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

impl fmt::Debug for ElfType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("None"),
            Self::RELOCATABLE => f.pad("Relocatable"),
            Self::EXECUTABLE => f.pad("Executable"),
            Self::SHARED => f.pad("SharedObject"),
            Self::CORE => f.pad("Core"),
            elf_kind => f.debug_tuple("ElfType").field(&elf_kind.0).finish(),
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
pub trait ClassElfHeader: ClassBase {
    /// The offset of the type field.
    fn elf_type_offset(self) -> u64;
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

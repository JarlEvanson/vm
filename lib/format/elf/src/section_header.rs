//! Ergonomic wrapper over ELF section headers.

use core::fmt;

use crate::{
    class::ClassBase,
    encoding::Encoding,
    extract_format,
    medium::{BackedMedium, Medium, MediumError},
    table::{Table, TableItem},
};

/// A [`Table`] of [`SectionHeader`]s.
pub type SectionHeaderTable<'slice, M, C, E> =
    Table<'slice, M, C, E, SectionHeader<'slice, M, C, E>>;

/// Contains basic information about a section in an ELF file.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SectionHeader<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`SectionHeader`].
    offset: u64,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassSectionHeader, E: Encoding>
    SectionHeader<'slice, M, C, E>
{
    /// Creates a new [`SectionHeader`] from the given [`Medium`].
    ///
    /// Returns [`None`] if the provided bounds are too small to contain a [`SectionHeader`].
    pub fn new(class: C, encoding: E, offset: u64, medium: &'slice M) -> Option<Self> {
        let max_offset = offset.checked_add(class.expected_section_header_size())?;
        if max_offset > medium.size() {
            return None;
        }

        let header = Self {
            medium,
            offset,
            class,
            encoding,
        };

        Some(header)
    }

    /// Returns the offset into the section header string table section that describes the name
    /// associated with this [`SectionHeader`].
    pub fn name_offset(&self) -> Result<u32, MediumError<M::Error>> {
        self.encoding.read_u32(
            self.offset + ClassSectionHeader::name_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the [`SectionType`] associated with this section.
    pub fn section_type(&self) -> Result<SectionType, MediumError<M::Error>> {
        self.encoding
            .read_u32(
                self.offset + ClassSectionHeader::section_type_offset(self.class),
                self.medium,
            )
            .map(SectionType)
    }

    /// Returns various flags that affect the interpretation and manipulation of this section.
    pub fn flags(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassSectionHeader::flags_offset(self.class),
            self.medium,
        )
    }

    /// Returns the address at which the section's first byte should reside in memory.
    pub fn address(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + self.class.address_offset(),
            self.medium,
        )
    }

    /// Returns the offset at which the section's first byte resides in the [`Medium`].
    pub fn offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassSectionHeader::offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the size of the section.
    pub fn size(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassSectionHeader::size_offset(self.class),
            self.medium,
        )
    }

    /// Returns the [`SectionHeaderTable`] index link (interpretation depends on the
    /// [`SectionType`]).
    pub fn link(&self) -> Result<u32, MediumError<M::Error>> {
        self.encoding
            .read_u32(self.offset + self.class.link_offset(), self.medium)
    }

    /// Returns extra information (interpretation depends on the [`SectionType`]).
    pub fn info(&self) -> Result<u32, MediumError<M::Error>> {
        self.encoding.read_u32(
            self.offset + ClassSectionHeader::info_offset(self.class),
            self.medium,
        )
    }

    /// Returns the required alignment of the section.
    pub fn address_alignment(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + self.class.address_align_offset(),
            self.medium,
        )
    }

    /// Returns the size of fixed-size entries in a section.
    pub fn entry_size(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + self.class.entry_size_offset(),
            self.medium,
        )
    }

    /// Returns the underlying [`Medium`].
    pub fn medium(&self) -> &M {
        self.medium
    }

    /// Returns the [`Class`][crate::class::Class] implementation of this [`SectionHeader`].
    pub fn class(&self) -> C {
        self.class
    }

    /// Returns the [`Encoding`] implementation of this [`SectionHeader`].
    pub fn encoding(&self) -> E {
        self.encoding
    }
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: BackedMedium + ?Sized, C: ClassSectionHeader, E: Encoding>
    SectionHeader<'slice, M, C, E>
{
    /// Returns the bytes contained in the section described by [`SectionHeader`].
    ///
    /// Returns [`None`] if [`SectionType::NOBITS`].
    pub fn section(&self) -> Result<&'slice [u8], MediumError<M::Error>> {
        if self.section_type()? == SectionType::NOBITS {
            return Ok(&[]);
        }

        let offset: u64 = self.offset()?.into();
        let size: u64 = self.size()?.into();
        self.medium.access_slice(offset, size)
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassSectionHeader, E: Encoding> fmt::Debug
    for SectionHeader<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name_offset = self.name_offset();
        let section_type = self.section_type();
        let flags = self.flags();
        let address = self.address();
        let offset = self.offset();
        let size = self.size();
        let link = self.link();
        let info = self.info();
        let address_alignment = self.address_alignment();
        let entry_size = self.entry_size();

        f.debug_struct("SectionHeader")
            .field("name_offset", extract_format(&name_offset))
            .field("section_type", extract_format(&section_type))
            .field("flags", extract_format(&flags))
            .field("address", extract_format(&address))
            .field("offset", extract_format(&offset))
            .field("size", extract_format(&size))
            .field("link", extract_format(&link))
            .field("info", extract_format(&info))
            .field("address_alignment", extract_format(&address_alignment))
            .field("entry_size", extract_format(&entry_size))
            .finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassSectionHeader, E: Encoding> TableItem<'slice, M, C, E>
    for SectionHeader<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_section_header_size())
            .expect("overflow when calculating max offset");
        assert!(max_offset <= medium.size(), "out of bounds structure");

        Self {
            medium,
            offset,
            class,
            encoding,
        }
    }

    fn expected_size(c: C) -> u64 {
        c.expected_section_header_size()
    }
}

/// The type of the section.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectionType(pub u32);

impl SectionType {
    /// The [`SectionHeader`] does not have an associated section.
    pub const NULL: Self = Self(0);
    /// The section holds information defined by the program.
    pub const PROGBITS: Self = Self(1);
    /// The section holds a symbol table.
    pub const SYMTAB: Self = Self(2);
    /// The section holds a string table.
    pub const STRTAB: Self = Self(3);
    /// The section holds [`Rela`][r] entries.
    ///
    /// [r]: crate::relocation::Rela
    pub const RELA: Self = Self(4);
    /// The section holds a symbol hash table.
    pub const HASH: Self = Self(5);
    /// The section holds information for dynamic linking.
    pub const DYNAMIC: Self = Self(6);
    /// The section holds information used for marking the file in some way.
    pub const NOTE: Self = Self(7);
    /// The section of this type occupies no space in the file, but otherwise resembles
    /// [`SectionType::PROGBITS`].
    pub const NOBITS: Self = Self(8);
    /// The section holds [`Rel`][r] entries.
    ///
    /// [r]: crate::relocation::Rel
    pub const REL: Self = Self(9);
    /// This [`SectionType`] is reserved and has unspecified semantics.
    pub const SHLIB: Self = Self(10);
    /// The section holds a dynamic symbol table.
    pub const DYNSYM: Self = Self(11);
    /// The section holds an array of pointers to initialization functions.
    pub const INIT_ARRAY: Self = Self(12);
    /// The section holds an array of pointers to termination functions.
    pub const FINI_ARRAY: Self = Self(13);
    /// The section holds an array of pointers to function invoked before all other initialization
    /// functions.
    pub const PREINIT_ARRAY: Self = Self(14);
    /// The section defines a section group.
    pub const GROUP: Self = Self(15);
    /// The section is associated with a symbol table section.
    pub const SYMTAB_SHNDX: Self = Self(16);
}

impl fmt::Debug for SectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NULL => f.pad("Null"),
            Self::PROGBITS => f.pad("ProgBits"),
            Self::SYMTAB => f.pad("SymTab"),
            Self::STRTAB => f.pad("StrTab"),
            Self::RELA => f.pad("Rela"),
            Self::HASH => f.pad("Hash"),
            Self::DYNAMIC => f.pad("Dynamic"),
            Self::NOTE => f.pad("Note"),
            Self::NOBITS => f.pad("NoBits"),
            Self::REL => f.pad("Rel"),
            Self::SHLIB => f.pad("ShLib"),
            Self::DYNSYM => f.pad("DynSym"),
            Self::INIT_ARRAY => f.pad("InitArray"),
            Self::FINI_ARRAY => f.pad("FiniArray"),
            Self::PREINIT_ARRAY => f.pad("PreInitArray"),
            Self::GROUP => f.pad("Group"),
            Self::SYMTAB_SHNDX => f.pad("SymTabShIndex"),
            section_kind => f.debug_tuple("SectionKind").field(&section_kind.0).finish(),
        }
    }
}

/// The definitions required to implement class aware parsing of ELF section headers.
pub trait ClassSectionHeader: ClassBase {
    /// The offset of the name field.
    fn name_offset_offset(self) -> u64;
    /// The offset of the section type field.
    fn section_type_offset(self) -> u64;
    /// The offset of the flags field.
    fn flags_offset(self) -> u64;
    /// The offset of the address field.
    fn address_offset(self) -> u64;
    /// The offset of the offset field.
    fn offset_offset(self) -> u64;
    /// The offset of the size field.
    fn size_offset(self) -> u64;
    /// The offset of the link field.
    fn link_offset(self) -> u64;
    /// The offset of the info field.
    fn info_offset(self) -> u64;
    /// The offset of the address alignment field.
    fn address_align_offset(self) -> u64;
    /// The offset of the entry size field.
    fn entry_size_offset(self) -> u64;

    /// The expected size of an ELF section header.
    fn expected_section_header_size(self) -> u64;
}

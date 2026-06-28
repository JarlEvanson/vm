//! Ergonomic wrapper over ELF program headers.

use core::fmt;

use crate::{
    class::ClassBase,
    encoding::Encoding,
    extract_format,
    medium::{BackedMedium, Medium, MediumError},
    table::{Table, TableItem},
};

/// A [`Table`] of [`ProgramHeader`]s.
pub type ProgramHeaderTable<'slice, M, C, E> =
    Table<'slice, M, C, E, ProgramHeader<'slice, M, C, E>>;

/// Contains basic information about a segment in an ELF file.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ProgramHeader<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`ProgramHeader`].
    offset: u64,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassProgramHeader, E: Encoding>
    ProgramHeader<'slice, M, C, E>
{
    /// Creates a new [`ProgramHeader`] from the given [`Medium`].
    ///
    /// Returns [`None`] if the provided bounds are too small to contain a [`ProgramHeader`].
    pub fn new(class: C, encoding: E, offset: u64, medium: &'slice M) -> Option<Self> {
        let max_offset = offset.checked_add(class.expected_program_header_size())?;
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

    /// Returns the [`SegmentType`] associated with this segment.
    pub fn segment_type(&self) -> Result<SegmentType, MediumError<M::Error>> {
        self.encoding
            .read_u32(
                self.offset + ClassProgramHeader::segment_type_offset(self.class),
                self.medium,
            )
            .map(SegmentType)
    }

    /// Returns the offset at which the segment's first byte resides in the [`Medium`].
    pub fn offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the virtual address at which the first byte of the segment resides in memory.
    pub fn virtual_address(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::virtual_address_offset(self.class),
            self.medium,
        )
    }

    /// Returns the physical address at which the first byte of the segment resides in memory.
    ///
    /// If physical addressing is not relevant, then this field has unspecified contents.
    pub fn physical_address(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::physical_address_offset(self.class),
            self.medium,
        )
    }

    /// Returns the number of bytes in the file image of the segment.
    ///
    /// This may be zero.
    pub fn file_size(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::file_size_offset(self.class),
            self.medium,
        )
    }

    /// Returns the number of bytes in the memory image of the segment.
    ///
    /// This may be zero.
    pub fn memory_size(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::memory_size_offset(self.class),
            self.medium,
        )
    }

    /// Returns various flags that affect the interpretation and manipulation of this segment.
    pub fn flags(&self) -> Result<SegmentFlags, MediumError<M::Error>> {
        self.encoding
            .read_u32(
                self.offset + ClassProgramHeader::flags_offset(self.class),
                self.medium,
            )
            .map(SegmentFlags)
    }

    /// Returns the required alignment of the segment.
    ///
    /// [`ProgramHeader::offset()`] must equal [`ProgramHeader::virtual_address()`], modulo
    /// [`ProgramHeader::alignment()`].
    pub fn alignment(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::align_offset(self.class),
            self.medium,
        )
    }

    /// Returns the underlying [`Medium`].
    pub fn medium(&self) -> &M {
        self.medium
    }

    /// Returns the [`Class`][crate::class::Class] implementation of this [`ProgramHeader`].
    pub fn class(&self) -> C {
        self.class
    }

    /// Returns the [`Encoding`] implementation of this [`ProgramHeader`].
    pub fn encoding(&self) -> E {
        self.encoding
    }
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: BackedMedium + ?Sized, C: ClassProgramHeader, E: Encoding>
    ProgramHeader<'slice, M, C, E>
{
    /// Returns the bytes contained in the file image of the segment as described by
    /// [`ProgramHeader`].
    ///
    /// The returned slice will be [`Self::file_size()`] bytes, not [`Self::memory_size()`] bytes.
    pub fn segment(&self) -> Result<&'slice [u8], MediumError<M::Error>> {
        let offset: u64 = self.offset()?.into();
        let size: u64 = self.file_size()?.into();
        self.medium.access_slice(offset, size)
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassProgramHeader, E: Encoding> fmt::Debug
    for ProgramHeader<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segment_type = self.segment_type();
        let offset = self.offset();
        let virtual_address = self.virtual_address();
        let file_size = self.file_size();
        let memory_size = self.memory_size();
        let flags = self.flags();
        let alignment = self.alignment();

        f.debug_struct("ProgramHeader")
            .field("segment_type", extract_format(&segment_type))
            .field("offset", extract_format(&offset))
            .field("virtual_address", extract_format(&virtual_address))
            .field("file_size", extract_format(&file_size))
            .field("memory_size", extract_format(&memory_size))
            .field("flags", extract_format(&flags))
            .field("alignment", extract_format(&alignment))
            .finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassProgramHeader, E: Encoding> TableItem<'slice, M, C, E>
    for ProgramHeader<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        Self::new(class, encoding, offset, medium).expect("out of bounds structure")
    }

    fn expected_size(c: C) -> u64 {
        c.expected_program_header_size()
    }
}

/// The type of the segment.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentType(pub u32);

impl SegmentType {
    /// The [`ProgramHeader`] is unused.
    pub const NULL: Self = Self(0);
    /// The segment is loadable, as described by [`ProgramHeader::file_size()`] and
    /// [`ProgramHeader::memory_size()`].
    pub const LOAD: Self = Self(1);
    /// The segment contains dynamic linking information.
    pub const DYNAMIC: Self = Self(2);
    /// The segment specifies the location and size of a null-terminated path name to invoke as an
    /// interpreter.
    pub const INTERP: Self = Self(3);
    /// The segment specifies the location and size of auxiliary information.
    pub const NOTE: Self = Self(4);
    /// This [`SegmentType`] type is reserved and segments with it have unspecified semantics.
    pub const SHLIB: Self = Self(5);
    /// The segment specifies the location and size of the [`ProgramHeaderTable`] itself, both in
    /// the file and the memory image of the program.
    pub const PHDR: Self = Self(6);
    /// The segment specifies the thread-local storage template.
    pub const TLS: Self = Self(7);
}

impl fmt::Debug for SegmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NULL => f.pad("Null"),
            Self::LOAD => f.pad("Load"),
            Self::DYNAMIC => f.pad("Dynamic"),
            Self::INTERP => f.pad("Interpreter"),
            Self::NOTE => f.pad("Note"),
            Self::SHLIB => f.pad("ShLib"),
            Self::PHDR => f.pad("Phdr"),
            Self::TLS => f.pad("Tls"),
            segment_type => f.debug_tuple("SegmentType").field(&segment_type.0).finish(),
        }
    }
}

/// The flags relevant to the segment.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentFlags(pub u32);

impl SegmentFlags {
    /// The segment must be executable.
    pub const EXECUTE: Self = Self(0x1);
    /// The segment must be writable.
    pub const WRITE: Self = Self(0x2);
    /// The segment must be readable.
    pub const READ: Self = Self(0x4);

    /// Returns `true` if `self` contains the flags that `rhs` has set.
    pub const fn contains(self, rhs: Self) -> bool {
        (self.0 & rhs.0) == rhs.0
    }
}

/// The definitions required to implement class aware parsing of ELF program headers.
pub trait ClassProgramHeader: ClassBase {
    /// The offset of the segment type field.
    fn segment_type_offset(self) -> u64;
    /// The offset of the offset field.
    fn offset_offset(self) -> u64;
    /// The offset of the virtual address field.
    fn virtual_address_offset(self) -> u64;
    /// The offset of the physical address field.
    fn physical_address_offset(self) -> u64;
    /// The offset of the file size field.
    fn file_size_offset(self) -> u64;
    /// The offset of the memory size field.
    fn memory_size_offset(self) -> u64;
    /// The offset of the flags field.
    fn flags_offset(self) -> u64;
    /// The offset of the alignment field.
    fn align_offset(self) -> u64;

    /// The expected size of an ELF program header.
    fn expected_program_header_size(self) -> u64;
}

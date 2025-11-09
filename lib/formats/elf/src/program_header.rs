//! Ergonomic wrapper over ELF program headers.

use core::fmt;

use crate::{
    BackedMedium, Class, ClassBase, Encoding, Medium,
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
    /// The [`Class`] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> ProgramHeader<'slice, M, C, E> {
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

    /// Returns the [`SegmentKind`] associated with this segment.
    pub fn kind(&self) -> SegmentKind {
        SegmentKind(self.encoding.parse_u32(
            self.offset + ClassProgramHeader::kind_offset(self.class),
            self.medium,
        ))
    }

    /// Returns the offset at which the segment's first byte resides in the [`Medium`].
    pub fn offset(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the virtual address at which the first byte of the segment resides in memory.
    pub fn virtual_address(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::virtual_address_offset(self.class),
            self.medium,
        )
    }

    /// Returns the physical address at which the first byte of the segment resides in memory.
    ///
    /// If physical addressing is not relevant, then this field has unspecified contents.
    pub fn physical_address(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::physical_address_offset(self.class),
            self.medium,
        )
    }

    /// Returns the number of bytes in the file image of the segment.
    ///
    /// This may be zero.
    pub fn file_size(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::file_size_offset(self.class),
            self.medium,
        )
    }

    /// Returns the number of bytes in the memory image of the segment.
    ///
    /// This may be zero.
    pub fn memory_size(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::memory_size_offset(self.class),
            self.medium,
        )
    }

    /// Returns various flags that affect the interpretation and manipulation of this segment.
    pub fn flags(&self) -> SegmentFlags {
        SegmentFlags(self.encoding.parse_u32(
            self.offset + ClassProgramHeader::flags_offset(self.class),
            self.medium,
        ))
    }

    /// Returns the required alignment of the segment.
    ///
    /// [`ProgramHeader::offset()`] must equal [`ProgramHeader::virtual_address()`], modulo
    /// [`ProgramHeader::alignment()`].
    pub fn alignment(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassProgramHeader::align_offset(self.class),
            self.medium,
        )
    }
}

impl<'slice, M: BackedMedium + ?Sized, C: Class, E: Encoding> ProgramHeader<'slice, M, C, E> {
    /// Returns the bytes contained in the file image of the segment as described by
    /// [`ProgramHeader`].
    pub fn segment(&self) -> Option<&[u8]> {
        let offset: u64 = self.offset().into();
        let size: u64 = self.file_size().into();
        self.medium.access_slice(offset, size)
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> TableItem<'slice, M, C, E>
    for ProgramHeader<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_program_header_size())
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
        c.expected_program_header_size()
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> fmt::Debug
    for ProgramHeader<'slice, M, C, E>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("ProgramHeader");

        debug_struct.field("kind", &self.kind());
        debug_struct.field("offset", &self.offset());
        debug_struct.field("virtual_address", &self.virtual_address());
        debug_struct.field("physical_address", &self.physical_address());
        debug_struct.field("file_size", &self.file_size());
        debug_struct.field("memory_size", &self.memory_size());
        debug_struct.field("flags", &self.flags());
        debug_struct.field("alignment", &self.alignment());

        debug_struct.finish()
    }
}

/// The kind of the segment.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SegmentKind(pub u32);

impl SegmentKind {
    /// The [`ProgramHeader`] is unused.
    pub const NULL: Self = Self(0);
    /// The segment is loadable, as descibed by [`ProgramHeader::file_size()`] and
    /// [`ProgramHeader::memory_size()`].
    pub const LOAD: Self = Self(1);
    /// The segment contains dynamic linking information.
    pub const DYNAMIC: Self = Self(2);
    /// The segment specifies the location and size of a null-terminated path name to invoke as an
    /// interpreter.
    pub const INTERP: Self = Self(3);
    /// The segment specifies the location and size of auxilary information.
    pub const NOTE: Self = Self(4);
    /// This [`SegmentKind`] kind is reserved and segments with it have unspecified semantics.
    pub const SHLIB: Self = Self(5);
    /// The segment specifies the location and size of the [`ProgramHeaderTable`] itself, both in
    /// the file and the memory image of the program.
    pub const PHDR: Self = Self(6);
    /// The segment specifies the thread-local storage template.
    pub const TLS: Self = Self(7);
}

impl fmt::Debug for SegmentKind {
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
            segment_kind => f.debug_tuple("SegmentKind").field(&segment_kind.0).finish(),
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

    /// Returns `true` if `self` contains the flags that `lhs` has set.
    pub const fn contains(self, lhs: Self) -> bool {
        self.0 & lhs.0 == lhs.0
    }
}

/// The definitions required to implement class aware parsing of ELF section headers.
pub trait ClassProgramHeader: ClassBase {
    /// The offset of the kind field.
    fn kind_offset(self) -> u64;
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

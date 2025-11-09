//! Ergonomic wrapper over ELF symbols.

use core::fmt;

use crate::{
    Class, ClassBase, Encoding, Medium,
    table::{Table, TableItem},
};

/// A [`Table`] of [`Symbol`]s.
pub type SymbolTable<'slice, M, C, E> = Table<'slice, M, C, E, Symbol<'slice, M, C, E>>;

/// Contains basic information about a symbol in an ELF file.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Symbol<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the start of the [`Symbol`].
    offset: u64,
    /// The [`Class`] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> Symbol<'slice, M, C, E> {
    /// Returns the offset into the symbol string table that describes the name of the [`Symbol`].
    pub fn name_offset(&self) -> u32 {
        self.encoding.parse_u32(
            self.offset + ClassSymbol::name_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the value of the [`Symbol`].
    ///
    /// This may be an absolute value, an address, or other type.
    pub fn value(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassSymbol::value_offset(self.class),
            self.medium,
        )
    }

    /// Returns the size associated with the [`Symbol`].
    pub fn size(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassSymbol::size_offset(self.class),
            self.medium,
        )
    }

    /// Returns information about the [`Symbol`]'s type and binding attributes.
    pub fn info(&self) -> u8 {
        self.encoding.parse_u8(
            self.offset + ClassSymbol::info_offset(self.class),
            self.medium,
        )
    }

    /// Returns information about the [`Symbol`]'s visibility.
    pub fn other(&self) -> u8 {
        self.encoding.parse_u8(
            self.offset + ClassSymbol::other_offset(self.class),
            self.medium,
        )
    }

    /// Returns index of the [`SectionHeader`][sh] relative to which this [`Symbol`] is defined.
    ///
    /// [sh]: crate::section_header::SectionHeader
    pub fn section_header_index(&self) -> u16 {
        self.encoding.parse_u16(
            self.offset + ClassSymbol::section_header_index_offset(self.class),
            self.medium,
        )
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> TableItem<'slice, M, C, E>
    for Symbol<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_symbol_size())
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
        c.expected_symbol_size()
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> fmt::Debug for Symbol<'slice, M, C, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Symbol");

        debug_struct.field("name_offset", &self.name_offset());
        debug_struct.field("value", &self.value());
        debug_struct.field("size", &self.size());
        debug_struct.field("info", &self.info());
        debug_struct.field("other", &self.other());
        debug_struct.field("section_header_index", &self.section_header_index());

        debug_struct.finish()
    }
}

/// The definitions required to implement class aware parsing of ELF symbols.
pub trait ClassSymbol: ClassBase {
    /// The offset of the name field.
    fn name_offset_offset(self) -> u64;
    /// The offset of the value field.
    fn value_offset(self) -> u64;
    /// The offset of the size field.
    fn size_offset(self) -> u64;
    /// The offset of the info field.
    fn info_offset(self) -> u64;
    /// The offset of the other field.
    fn other_offset(self) -> u64;
    /// The offset of the section header index field.
    fn section_header_index_offset(self) -> u64;

    /// The expected size of an ELF symbol.
    fn expected_symbol_size(self) -> u64;
}

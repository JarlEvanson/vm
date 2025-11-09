//! Ergonomic wrapper over ELF relocations.

use core::fmt;

use crate::{
    Class, ClassBase, Encoding, Medium,
    table::{Table, TableItem},
};

/// A [`Table`] of [`Rel`]s.
pub type RelTable<'slice, M, C, E> = Table<'slice, M, C, E, Rel<'slice, M, C, E>>;
/// A [`Table`] of [`Rela`]s.
pub type RelaTable<'slice, M, C, E> = Table<'slice, M, C, E, Rela<'slice, M, C, E>>;

/// An ELF relocation without an explicit addend.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rel<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`Rel`].
    offset: u64,
    /// The [`Class`] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}
impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> Rel<'slice, M, C, E> {
    /// Returns the offset at which to apply the relocation.
    pub fn offset(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rel_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the info field of this [`Rel`], which contains the symbol table index associated
    /// with this relocation and the type of relocation to apply.
    pub fn info(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rel_info_offset(self.class),
            self.medium,
        )
    }

    /// Returns the symbol table index associated with this relocation.
    pub fn symbol_index(&self) -> C::SymbolIndex {
        self.class.symbol_index_raw(self.info())
    }

    /// Returns the type of relocation to apply.
    pub fn relocation_kind(&self) -> C::RelocationKind {
        self.class.relocation_kind_raw(self.info())
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> TableItem<'slice, M, C, E>
    for Rel<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_rel_size())
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
        c.expected_rel_size()
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> fmt::Debug for Rel<'slice, M, C, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Rel");

        debug_struct.field("offset", &self.offset());
        debug_struct.field("symbol_index", &self.symbol_index());
        debug_struct.field("relocation_kind", &self.relocation_kind());

        debug_struct.finish()
    }
}

/// An ELF relocation with an explicit addend.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rela<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`Rela`].
    offset: u64,
    /// The [`Class`] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}
impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> Rela<'slice, M, C, E> {
    /// Returns the offset at which to apply the relocation.
    pub fn offset(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rela_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the info field of this [`Rela`], which contains the symbol table index associated
    /// with this relocation and the type of relocation to apply.
    pub fn info(&self) -> C::ClassUsize {
        self.class.parse_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rela_info_offset(self.class),
            self.medium,
        )
    }

    /// Returns the symbol table index associated with this relocation.
    pub fn symbol_index(&self) -> C::SymbolIndex {
        self.class.symbol_index_raw(self.info())
    }

    /// Returns the type of relocation to apply.
    pub fn relocation_kind(&self) -> C::RelocationKind {
        self.class.relocation_kind_raw(self.info())
    }

    /// Returns the constant addend used to compute the relocation.
    pub fn addend(&self) -> C::ClassIsize {
        self.class.parse_class_isize(
            self.encoding,
            self.offset + ClassRelocation::rela_addend_offset(self.class),
            self.medium,
        )
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> TableItem<'slice, M, C, E>
    for Rela<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_rela_size())
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
        c.expected_rel_size()
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding> fmt::Debug for Rela<'slice, M, C, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Rela");

        debug_struct.field("offset", &self.offset());
        debug_struct.field("symbol_index", &self.symbol_index());
        debug_struct.field("relocation_kind", &self.relocation_kind());
        debug_struct.field("addend", &self.addend());

        debug_struct.finish()
    }
}

/// The definitions required to implement class aware parsing of ELF relocations.
pub trait ClassRelocation: ClassBase {
    /// The type representing the symbol index of the relocation.
    type SymbolIndex: fmt::Debug;
    /// The type representing the kind of the relocation.
    type RelocationKind: fmt::Debug;

    /// The symbol table index extracted from the info field.
    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex;
    /// The raw relocation kind extracted from the info field.
    fn relocation_kind_raw(self, info: Self::ClassUsize) -> Self::RelocationKind;

    /// The offset of the offset field in [`Rel`].
    fn rel_offset_offset(self) -> u64;
    /// The offset of the info field in [`Rel`].
    fn rel_info_offset(self) -> u64;

    /// The offset of the offset field in [`Rela`].
    fn rela_offset_offset(self) -> u64;
    /// The offset of the offset field in [`Rela`].
    fn rela_info_offset(self) -> u64;
    /// The offset of the offset field in [`Rela`].
    fn rela_addend_offset(self) -> u64;

    /// The expected size of a [`Rel`].
    fn expected_rel_size(self) -> u64;
    /// The expected size of a [`Rela`].
    fn expected_rela_size(self) -> u64;
}

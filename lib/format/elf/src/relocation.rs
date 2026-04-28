//! Ergonomic wrapper over ELF relocations.

use core::fmt;

use crate::{
    class::ClassBase,
    encoding::Encoding,
    extract_format,
    medium::{Medium, MediumError},
    table::TableItem,
};

/// An ELF relocation without an explicit addend.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rel<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`Rel`].
    offset: u64,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> Rel<'slice, M, C, E> {
    /// Returns the offset at which to apply the relocation.
    pub fn offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rel_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the info field of this [`Rel`], which contains the symbol table index associated
    /// with this relocation and the type of relocation to apply.
    pub fn info(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rel_info_offset(self.class),
            self.medium,
        )
    }

    /// Returns the symbol table index associated with this relocation.
    pub fn symbol_index(&self) -> Result<C::SymbolIndex, MediumError<M::Error>> {
        self.info().map(|value| self.class.symbol_index_raw(value))
    }

    /// Returns the type of relocation to apply.
    pub fn relocation_type(&self) -> Result<C::RelocationType, MediumError<M::Error>> {
        self.info()
            .map(|value| self.class.relocation_type_raw(value))
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> fmt::Debug
    for Rel<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset = self.offset();
        let symbol_index = self.symbol_index();
        let relocation_type = self.relocation_type();

        f.debug_struct("Rela")
            .field("offset", extract_format(&offset))
            .field("symbol_index", extract_format(&symbol_index))
            .field("relocation_type", extract_format(&relocation_type))
            .finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> TableItem<'slice, M, C, E>
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

/// An ELF relocation with an explicit addend.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Rela<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`Rela`].
    offset: u64,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> Rela<'slice, M, C, E> {
    /// Returns the offset at which to apply the relocation.
    pub fn offset(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rela_offset_offset(self.class),
            self.medium,
        )
    }

    /// Returns the info field of this [`Rela`], which contains the symbol table index associated
    /// with this relocation and the type of relocation to apply.
    pub fn info(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassRelocation::rela_info_offset(self.class),
            self.medium,
        )
    }

    /// Returns the symbol table index associated with this relocation.
    pub fn symbol_index(&self) -> Result<C::SymbolIndex, MediumError<M::Error>> {
        self.info().map(|value| self.class.symbol_index_raw(value))
    }

    /// Returns the type of relocation to apply.
    pub fn relocation_type(&self) -> Result<C::RelocationType, MediumError<M::Error>> {
        self.info()
            .map(|value| self.class.relocation_type_raw(value))
    }

    /// Returns the constant addend used to compute the relocation.
    pub fn addend(&self) -> Result<C::ClassIsize, MediumError<M::Error>> {
        self.class.read_class_isize(
            self.encoding,
            self.offset + ClassRelocation::rela_addend_offset(self.class),
            self.medium,
        )
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> fmt::Debug
    for Rela<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset = self.offset();
        let symbol_index = self.symbol_index();
        let relocation_type = self.relocation_type();
        let addend = self.addend();

        f.debug_struct("Rela")
            .field("offset", extract_format(&offset))
            .field("symbol_index", extract_format(&symbol_index))
            .field("relocation_type", extract_format(&relocation_type))
            .field("addend", extract_format(&addend))
            .finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassRelocation, E: Encoding> TableItem<'slice, M, C, E>
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
        c.expected_rela_size()
    }
}

/// The definitions required to implement class aware parsing of ELF relocations.
pub trait ClassRelocation: ClassBase {
    /// The type representing the symbol index of the relocation.
    type SymbolIndex: fmt::Debug;
    /// The type representing the type of the relocation.
    type RelocationType: fmt::Debug;

    /// The symbol table index extracted from the info field.
    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex;
    /// The raw relocation type extracted from the info field.
    fn relocation_type_raw(self, info: Self::ClassUsize) -> Self::RelocationType;

    /// The offset of the offset field in [`Rel`].
    fn rel_offset_offset(self) -> u64;
    /// The offset of the info field in [`Rel`].
    fn rel_info_offset(self) -> u64;

    /// The offset of the offset field in [`Rela`].
    fn rela_offset_offset(self) -> u64;
    /// The offset of the info field in [`Rela`].
    fn rela_info_offset(self) -> u64;
    /// The offset of the offset field in [`Rela`].
    fn rela_addend_offset(self) -> u64;

    /// The expected size of a [`Rel`].
    fn expected_rel_size(self) -> u64;
    /// The expected size of a [`Rela`].
    fn expected_rela_size(self) -> u64;
}

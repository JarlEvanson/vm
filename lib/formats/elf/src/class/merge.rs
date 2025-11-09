//! Merged ELF file parsing.

use crate::{
    Encoding, Medium,
    class::{ClassBase, UnsupportedClassError},
    file_header::ClassFileHeader,
    ident,
    program_header::ClassProgramHeader,
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
};

/// A zero-sized object offering methods for safe parsing of 32-bit ELF files.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Merge<A: ClassBase, B: ClassBase> {
    /// The first [`ClassBase`] implementation.
    A(A),
    /// The second [`ClassBase`] implementation.
    B(B),
}

impl<A: ClassBase, B: ClassBase> ClassBase for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
{
    type ClassUsize = B::ClassUsize;
    type ClassIsize = B::ClassIsize;

    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError> {
        if let Ok(a) = A::from_elf_class(class) {
            return Ok(Self::A(a));
        }

        B::from_elf_class(class).map(Self::B)
    }

    fn parse_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassUsize {
        match self {
            Self::A(a) => B::ClassUsize::from(a.parse_class_usize(encoding, offset, medium)),
            Self::B(b) => b.parse_class_usize(encoding, offset, medium),
        }
    }

    fn parse_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassIsize {
        match self {
            Self::A(a) => B::ClassIsize::from(a.parse_class_isize(encoding, offset, medium)),
            Self::B(b) => b.parse_class_isize(encoding, offset, medium),
        }
    }
}

impl<A: ClassFileHeader, B: ClassFileHeader> ClassFileHeader for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
{
    fn elf_kind_offset(self) -> u64 {
        match self {
            Self::A(a) => A::elf_kind_offset(a),
            Self::B(b) => B::elf_kind_offset(b),
        }
    }

    fn machine_offset(self) -> u64 {
        match self {
            Self::A(a) => A::machine_offset(a),
            Self::B(b) => B::machine_offset(b),
        }
    }

    fn version_offset(self) -> u64 {
        match self {
            Self::A(a) => A::version_offset(a),
            Self::B(b) => B::version_offset(b),
        }
    }

    fn entry_offset(self) -> u64 {
        match self {
            Self::A(a) => A::entry_offset(a),
            Self::B(b) => B::entry_offset(b),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::A(a) => A::flags_offset(a),
            Self::B(b) => B::flags_offset(b),
        }
    }

    fn header_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::header_size_offset(a),
            Self::B(b) => B::header_size_offset(b),
        }
    }

    fn program_header_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::program_header_offset_offset(a),
            Self::B(b) => B::program_header_offset_offset(b),
        }
    }

    fn program_header_count_offset(self) -> u64 {
        match self {
            Self::A(a) => A::program_header_count_offset(a),
            Self::B(b) => B::program_header_count_offset(b),
        }
    }

    fn program_header_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::program_header_size_offset(a),
            Self::B(b) => B::program_header_size_offset(b),
        }
    }

    fn section_header_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::section_header_offset_offset(a),
            Self::B(b) => B::section_header_offset_offset(b),
        }
    }

    fn section_header_count_offset(self) -> u64 {
        match self {
            Self::A(a) => A::section_header_count_offset(a),
            Self::B(b) => B::section_header_count_offset(b),
        }
    }

    fn section_header_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::section_header_size_offset(a),
            Self::B(b) => B::section_header_size_offset(b),
        }
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        match self {
            Self::A(a) => A::section_header_string_table_index_offset(a),
            Self::B(b) => B::section_header_string_table_index_offset(b),
        }
    }

    fn expected_elf_header_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_elf_header_size(a),
            Self::B(b) => B::expected_elf_header_size(b),
        }
    }
}

impl<A: ClassSectionHeader, B: ClassSectionHeader> ClassSectionHeader for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
{
    fn name_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::name_offset_offset(a),
            Self::B(b) => B::name_offset_offset(b),
        }
    }

    fn kind_offset(self) -> u64 {
        match self {
            Self::A(a) => A::kind_offset(a),
            Self::B(b) => B::kind_offset(b),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::A(a) => A::flags_offset(a),
            Self::B(b) => B::flags_offset(b),
        }
    }

    fn address_offset(self) -> u64 {
        match self {
            Self::A(a) => A::address_offset(a),
            Self::B(b) => B::address_offset(b),
        }
    }

    fn offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::offset_offset(a),
            Self::B(b) => B::offset_offset(b),
        }
    }

    fn size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::size_offset(a),
            Self::B(b) => B::size_offset(b),
        }
    }

    fn link_offset(self) -> u64 {
        match self {
            Self::A(a) => A::link_offset(a),
            Self::B(b) => B::link_offset(b),
        }
    }

    fn info_offset(self) -> u64 {
        match self {
            Self::A(a) => A::info_offset(a),
            Self::B(b) => B::info_offset(b),
        }
    }

    fn address_align_offset(self) -> u64 {
        match self {
            Self::A(a) => A::address_align_offset(a),
            Self::B(b) => B::address_align_offset(b),
        }
    }

    fn entry_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::entry_size_offset(a),
            Self::B(b) => B::entry_size_offset(b),
        }
    }

    fn expected_section_header_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_section_header_size(a),
            Self::B(b) => B::expected_section_header_size(b),
        }
    }
}

impl<A: ClassSymbol, B: ClassSymbol> ClassSymbol for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
{
    fn name_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::name_offset_offset(a),
            Self::B(b) => B::name_offset_offset(b),
        }
    }

    fn value_offset(self) -> u64 {
        match self {
            Self::A(a) => A::value_offset(a),
            Self::B(b) => B::value_offset(b),
        }
    }

    fn size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::size_offset(a),
            Self::B(b) => B::size_offset(b),
        }
    }

    fn info_offset(self) -> u64 {
        match self {
            Self::A(a) => A::info_offset(a),
            Self::B(b) => B::info_offset(b),
        }
    }

    fn other_offset(self) -> u64 {
        match self {
            Self::A(a) => A::other_offset(a),
            Self::B(b) => B::other_offset(b),
        }
    }

    fn section_header_index_offset(self) -> u64 {
        match self {
            Self::A(a) => A::section_header_index_offset(a),
            Self::B(b) => B::section_header_index_offset(b),
        }
    }

    fn expected_symbol_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_symbol_size(a),
            Self::B(b) => B::expected_symbol_size(b),
        }
    }
}

impl<A: ClassRelocation, B: ClassRelocation> ClassRelocation for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
    B::SymbolIndex: From<A::SymbolIndex>,
    A::SymbolIndex: TryFrom<B::SymbolIndex>,
    B::RelocationKind: From<A::RelocationKind>,
    A::RelocationKind: TryFrom<B::RelocationKind>,
{
    type SymbolIndex = B::SymbolIndex;
    type RelocationKind = B::RelocationKind;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        match self {
            Self::A(a) => {
                let info = A::ClassUsize::try_from(info)
                    .ok()
                    .expect("misused `symbol_index_raw()`");
                B::SymbolIndex::from(a.symbol_index_raw(info))
            }
            Self::B(b) => b.symbol_index_raw(info),
        }
    }

    fn relocation_kind_raw(self, info: Self::ClassUsize) -> Self::RelocationKind {
        match self {
            Self::A(a) => {
                let info = A::ClassUsize::try_from(info)
                    .ok()
                    .expect("misused `symbol_index_raw()`");
                B::RelocationKind::from(a.relocation_kind_raw(info))
            }
            Self::B(b) => b.relocation_kind_raw(info),
        }
    }

    fn rel_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::rel_offset_offset(a),
            Self::B(b) => B::rel_offset_offset(b),
        }
    }

    fn rel_info_offset(self) -> u64 {
        match self {
            Self::A(a) => A::rel_info_offset(a),
            Self::B(b) => B::rel_info_offset(b),
        }
    }

    fn rela_offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::rela_offset_offset(a),
            Self::B(b) => B::rela_offset_offset(b),
        }
    }

    fn rela_info_offset(self) -> u64 {
        match self {
            Self::A(a) => A::rela_info_offset(a),
            Self::B(b) => B::rela_info_offset(b),
        }
    }

    fn rela_addend_offset(self) -> u64 {
        match self {
            Self::A(a) => A::rela_addend_offset(a),
            Self::B(b) => B::rela_addend_offset(b),
        }
    }

    fn expected_rel_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_rel_size(a),
            Self::B(b) => B::expected_rel_size(b),
        }
    }

    fn expected_rela_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_rela_size(a),
            Self::B(b) => B::expected_rela_size(b),
        }
    }
}

impl<A: ClassProgramHeader, B: ClassProgramHeader> ClassProgramHeader for Merge<A, B>
where
    B::ClassUsize: From<A::ClassUsize>,
    A::ClassUsize: TryFrom<B::ClassUsize>,
    B::ClassIsize: From<A::ClassIsize>,
    A::ClassIsize: TryFrom<B::ClassIsize>,
{
    fn kind_offset(self) -> u64 {
        match self {
            Self::A(a) => A::kind_offset(a),
            Self::B(b) => B::kind_offset(b),
        }
    }

    fn offset_offset(self) -> u64 {
        match self {
            Self::A(a) => A::offset_offset(a),
            Self::B(b) => B::offset_offset(b),
        }
    }

    fn virtual_address_offset(self) -> u64 {
        match self {
            Self::A(a) => A::virtual_address_offset(a),
            Self::B(b) => B::virtual_address_offset(b),
        }
    }

    fn physical_address_offset(self) -> u64 {
        match self {
            Self::A(a) => A::physical_address_offset(a),
            Self::B(b) => B::physical_address_offset(b),
        }
    }

    fn file_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::file_size_offset(a),
            Self::B(b) => B::file_size_offset(b),
        }
    }

    fn memory_size_offset(self) -> u64 {
        match self {
            Self::A(a) => A::memory_size_offset(a),
            Self::B(b) => B::memory_size_offset(b),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::A(a) => A::flags_offset(a),
            Self::B(b) => B::flags_offset(b),
        }
    }

    fn align_offset(self) -> u64 {
        match self {
            Self::A(a) => A::align_offset(a),
            Self::B(b) => B::align_offset(b),
        }
    }

    fn expected_program_header_size(self) -> u64 {
        match self {
            Self::A(a) => A::expected_program_header_size(a),
            Self::B(b) => B::expected_program_header_size(b),
        }
    }
}

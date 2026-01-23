//! Merged ELF file parsing.

use crate::{
    class::{ClassBase, UnsupportedClassError, class_32::Class32, class_64::Class64},
    dynamic::ClassDynamic,
    encoding::Encoding,
    header::ClassElfHeader,
    ident,
    medium::{Medium, MediumError},
    program_header::ClassProgramHeader,
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
};

/// A zero-sized object offering methods for safe parsing of merged ELF classes.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnyClass {
    /// ELF files are 32-bit.
    Class32,
    /// ELF files are 64-bit.
    Class64,
}

impl ClassBase for AnyClass {
    type ClassUsize = u64;
    type ClassIsize = i64;

    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError> {
        match class {
            ident::Class::CLASS32 => Ok(AnyClass::Class32),
            ident::Class::CLASS64 => Ok(AnyClass::Class64),
            class => Err(UnsupportedClassError(class)),
        }
    }

    fn read_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassUsize, MediumError<M::Error>> {
        match self {
            Self::Class32 => Class32
                .read_class_usize(encoding, offset, medium)
                .map(u64::from),
            Self::Class64 => Class64.read_class_usize(encoding, offset, medium),
        }
    }

    fn read_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassIsize, MediumError<M::Error>> {
        match self {
            Self::Class32 => Class32
                .read_class_isize(encoding, offset, medium)
                .map(i64::from),
            Self::Class64 => Class64.read_class_isize(encoding, offset, medium),
        }
    }
}

impl ClassElfHeader for AnyClass {
    fn elf_type_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::elf_type_offset(Class32),
            Self::Class64 => ClassElfHeader::elf_type_offset(Class64),
        }
    }

    fn machine_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::machine_offset(Class32),
            Self::Class64 => ClassElfHeader::machine_offset(Class64),
        }
    }

    fn version_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::version_offset(Class32),
            Self::Class64 => ClassElfHeader::version_offset(Class64),
        }
    }

    fn entry_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::entry_offset(Class32),
            Self::Class64 => ClassElfHeader::entry_offset(Class64),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::flags_offset(Class32),
            Self::Class64 => ClassElfHeader::flags_offset(Class64),
        }
    }

    fn header_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::header_size_offset(Class32),
            Self::Class64 => ClassElfHeader::header_size_offset(Class64),
        }
    }

    fn program_header_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::program_header_offset_offset(Class32),
            Self::Class64 => ClassElfHeader::program_header_offset_offset(Class64),
        }
    }

    fn program_header_count_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::program_header_count_offset(Class32),
            Self::Class64 => ClassElfHeader::program_header_count_offset(Class64),
        }
    }

    fn program_header_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::program_header_size_offset(Class32),
            Self::Class64 => ClassElfHeader::program_header_size_offset(Class64),
        }
    }

    fn section_header_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::section_header_offset_offset(Class32),
            Self::Class64 => ClassElfHeader::section_header_offset_offset(Class64),
        }
    }

    fn section_header_count_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::section_header_count_offset(Class32),
            Self::Class64 => ClassElfHeader::section_header_count_offset(Class64),
        }
    }

    fn section_header_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::section_header_size_offset(Class32),
            Self::Class64 => ClassElfHeader::section_header_size_offset(Class64),
        }
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::section_header_string_table_index_offset(Class32),
            Self::Class64 => ClassElfHeader::section_header_string_table_index_offset(Class64),
        }
    }

    fn expected_elf_header_size(self) -> u64 {
        match self {
            Self::Class32 => ClassElfHeader::expected_elf_header_size(Class32),
            Self::Class64 => ClassElfHeader::expected_elf_header_size(Class64),
        }
    }
}

impl ClassSectionHeader for AnyClass {
    fn name_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::name_offset_offset(Class32),
            Self::Class64 => ClassSectionHeader::name_offset_offset(Class64),
        }
    }

    fn section_type_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::section_type_offset(Class32),
            Self::Class64 => ClassSectionHeader::section_type_offset(Class64),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::flags_offset(Class32),
            Self::Class64 => ClassSectionHeader::flags_offset(Class64),
        }
    }

    fn address_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::address_offset(Class32),
            Self::Class64 => ClassSectionHeader::address_offset(Class64),
        }
    }

    fn offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::offset_offset(Class32),
            Self::Class64 => ClassSectionHeader::offset_offset(Class64),
        }
    }

    fn size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::size_offset(Class32),
            Self::Class64 => ClassSectionHeader::size_offset(Class64),
        }
    }

    fn link_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::link_offset(Class32),
            Self::Class64 => ClassSectionHeader::link_offset(Class64),
        }
    }

    fn info_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::info_offset(Class32),
            Self::Class64 => ClassSectionHeader::info_offset(Class64),
        }
    }

    fn address_align_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::address_align_offset(Class32),
            Self::Class64 => ClassSectionHeader::address_align_offset(Class64),
        }
    }

    fn entry_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::entry_size_offset(Class32),
            Self::Class64 => ClassSectionHeader::entry_size_offset(Class64),
        }
    }

    fn expected_section_header_size(self) -> u64 {
        match self {
            Self::Class32 => ClassSectionHeader::expected_section_header_size(Class32),
            Self::Class64 => ClassSectionHeader::expected_section_header_size(Class64),
        }
    }
}

impl ClassProgramHeader for AnyClass {
    fn segment_type_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::segment_type_offset(Class32),
            Self::Class64 => ClassProgramHeader::segment_type_offset(Class64),
        }
    }

    fn offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::offset_offset(Class32),
            Self::Class64 => ClassProgramHeader::offset_offset(Class64),
        }
    }

    fn virtual_address_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::virtual_address_offset(Class32),
            Self::Class64 => ClassProgramHeader::virtual_address_offset(Class64),
        }
    }

    fn physical_address_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::physical_address_offset(Class32),
            Self::Class64 => ClassProgramHeader::physical_address_offset(Class64),
        }
    }

    fn file_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::file_size_offset(Class32),
            Self::Class64 => ClassProgramHeader::file_size_offset(Class64),
        }
    }

    fn memory_size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::memory_size_offset(Class32),
            Self::Class64 => ClassProgramHeader::memory_size_offset(Class64),
        }
    }

    fn flags_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::flags_offset(Class32),
            Self::Class64 => ClassProgramHeader::flags_offset(Class64),
        }
    }

    fn align_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::align_offset(Class32),
            Self::Class64 => ClassProgramHeader::align_offset(Class64),
        }
    }

    fn expected_program_header_size(self) -> u64 {
        match self {
            Self::Class32 => ClassProgramHeader::expected_program_header_size(Class32),
            Self::Class64 => ClassProgramHeader::expected_program_header_size(Class64),
        }
    }
}

impl ClassRelocation for AnyClass {
    type SymbolIndex = u32;
    type RelocationType = u32;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        match self {
            #[expect(clippy::cast_possible_truncation)]
            Self::Class32 => ClassRelocation::symbol_index_raw(Class32, info as u32),
            Self::Class64 => ClassRelocation::symbol_index_raw(Class64, info),
        }
    }

    fn relocation_type_raw(self, info: Self::ClassUsize) -> Self::RelocationType {
        match self {
            #[expect(clippy::cast_possible_truncation)]
            Self::Class32 => u32::from(ClassRelocation::relocation_type_raw(Class32, info as u32)),
            Self::Class64 => ClassRelocation::relocation_type_raw(Class64, info),
        }
    }

    fn rel_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::rel_offset_offset(Class32),
            Self::Class64 => ClassRelocation::rel_offset_offset(Class64),
        }
    }

    fn rel_info_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::rel_info_offset(Class32),
            Self::Class64 => ClassRelocation::rel_info_offset(Class64),
        }
    }

    fn rela_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::rela_offset_offset(Class32),
            Self::Class64 => ClassRelocation::rela_offset_offset(Class64),
        }
    }

    fn rela_info_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::rela_info_offset(Class32),
            Self::Class64 => ClassRelocation::rela_info_offset(Class64),
        }
    }

    fn rela_addend_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::rela_addend_offset(Class32),
            Self::Class64 => ClassRelocation::rela_addend_offset(Class64),
        }
    }

    fn expected_rel_size(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::expected_rel_size(Class32),
            Self::Class64 => ClassRelocation::expected_rel_size(Class64),
        }
    }

    fn expected_rela_size(self) -> u64 {
        match self {
            Self::Class32 => ClassRelocation::expected_rela_size(Class32),
            Self::Class64 => ClassRelocation::expected_rela_size(Class64),
        }
    }
}

impl ClassDynamic for AnyClass {
    fn dynamic_tag_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassDynamic::dynamic_tag_offset(Class32),
            Self::Class64 => ClassDynamic::dynamic_tag_offset(Class64),
        }
    }

    fn dynamic_val_ptr_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassDynamic::dynamic_val_ptr_offset(Class32),
            Self::Class64 => ClassDynamic::dynamic_val_ptr_offset(Class64),
        }
    }

    fn expected_dynamic_size(self) -> u64 {
        match self {
            Self::Class32 => ClassDynamic::expected_dynamic_size(Class32),
            Self::Class64 => ClassDynamic::expected_dynamic_size(Class64),
        }
    }
}

impl ClassSymbol for AnyClass {
    fn name_offset_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::name_offset_offset(Class32),
            Self::Class64 => ClassSymbol::name_offset_offset(Class64),
        }
    }

    fn value_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::value_offset(Class32),
            Self::Class64 => ClassSymbol::value_offset(Class64),
        }
    }

    fn size_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::size_offset(Class32),
            Self::Class64 => ClassSymbol::size_offset(Class64),
        }
    }

    fn info_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::info_offset(Class32),
            Self::Class64 => ClassSymbol::info_offset(Class64),
        }
    }

    fn other_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::other_offset(Class32),
            Self::Class64 => ClassSymbol::other_offset(Class64),
        }
    }

    fn section_header_index_offset(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::section_header_index_offset(Class32),
            Self::Class64 => ClassSymbol::section_header_index_offset(Class64),
        }
    }

    fn expected_symbol_size(self) -> u64 {
        match self {
            Self::Class32 => ClassSymbol::expected_symbol_size(Class32),
            Self::Class64 => ClassSymbol::expected_symbol_size(Class64),
        }
    }
}

//! 64-bit ELF file parsing.

use core::mem;

use crate::{
    Encoding, Medium,
    class::{ClassBase, UnsupportedClassError},
    file_header::ClassFileHeader,
    ident,
    program_header::ClassProgramHeader,
    raw::{Elf64Header, Elf64ProgramHeader, Elf64Rel, Elf64Rela, Elf64SectionHeader, Elf64Symbol},
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
};

/// A zero-sized object offering methods for safe parsing of 64-bit ELF files.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Class64;

impl ClassBase for Class64 {
    type ClassUsize = u64;
    type ClassIsize = i64;

    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError> {
        if class != ident::Class::CLASS64 {
            return Err(UnsupportedClassError(class));
        }

        Ok(Self)
    }

    fn parse_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassUsize {
        encoding.parse_u64(offset, medium)
    }

    fn parse_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassIsize {
        encoding.parse_i64(offset, medium)
    }
}

impl ClassFileHeader for Class64 {
    fn elf_kind_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, kind) as u64
    }

    fn machine_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, machine) as u64
    }

    fn version_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, version) as u64
    }

    fn entry_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, entry) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, flags) as u64
    }

    fn header_size_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, ehsize) as u64
    }

    fn program_header_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, phoff) as u64
    }

    fn program_header_count_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, phnum) as u64
    }

    fn program_header_size_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, phentsize) as u64
    }

    fn section_header_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, shoff) as u64
    }

    fn section_header_count_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, shnum) as u64
    }

    fn section_header_size_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, shentsize) as u64
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        mem::offset_of!(Elf64Header, shstrndx) as u64
    }

    fn expected_elf_header_size(self) -> u64 {
        mem::size_of::<Elf64Header>() as u64
    }
}

impl ClassSectionHeader for Class64 {
    fn name_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, name) as u64
    }

    fn kind_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, kind) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, flags) as u64
    }

    fn address_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, addr) as u64
    }

    fn offset_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, offset) as u64
    }

    fn size_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, size) as u64
    }

    fn link_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, link) as u64
    }

    fn info_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, info) as u64
    }

    fn address_align_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, addralign) as u64
    }

    fn entry_size_offset(self) -> u64 {
        mem::offset_of!(Elf64SectionHeader, entsize) as u64
    }

    fn expected_section_header_size(self) -> u64 {
        mem::size_of::<Elf64SectionHeader>() as u64
    }
}

impl ClassSymbol for Class64 {
    fn name_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, name) as u64
    }

    fn value_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, value) as u64
    }

    fn size_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, size) as u64
    }

    fn info_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, info) as u64
    }

    fn other_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, other) as u64
    }

    fn section_header_index_offset(self) -> u64 {
        mem::offset_of!(Elf64Symbol, shndx) as u64
    }

    fn expected_symbol_size(self) -> u64 {
        mem::size_of::<Elf64Symbol>() as u64
    }
}

impl ClassRelocation for Class64 {
    type SymbolIndex = u32;
    type RelocationKind = u32;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        (info >> 32) as u32
    }

    fn relocation_kind_raw(self, info: Self::ClassUsize) -> Self::RelocationKind {
        (info & 0xFFFF_FFFF) as u32
    }

    fn rel_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64Rel, offset) as u64
    }

    fn rel_info_offset(self) -> u64 {
        mem::offset_of!(Elf64Rel, info) as u64
    }

    fn rela_offset_offset(self) -> u64 {
        mem::offset_of!(Elf64Rela, offset) as u64
    }

    fn rela_info_offset(self) -> u64 {
        mem::offset_of!(Elf64Rela, info) as u64
    }

    fn rela_addend_offset(self) -> u64 {
        mem::offset_of!(Elf64Rela, addend) as u64
    }

    fn expected_rel_size(self) -> u64 {
        mem::size_of::<Elf64Rel>() as u64
    }

    fn expected_rela_size(self) -> u64 {
        mem::size_of::<Elf64Rela>() as u64
    }
}

impl ClassProgramHeader for Class64 {
    fn kind_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, kind) as u64
    }

    fn offset_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, offset) as u64
    }

    fn virtual_address_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, vaddr) as u64
    }

    fn physical_address_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, paddr) as u64
    }

    fn file_size_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, filesz) as u64
    }

    fn memory_size_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, memsz) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, flags) as u64
    }

    fn align_offset(self) -> u64 {
        mem::offset_of!(Elf64ProgramHeader, align) as u64
    }

    fn expected_program_header_size(self) -> u64 {
        mem::size_of::<Elf64ProgramHeader>() as u64
    }
}

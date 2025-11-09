//! 32-bit ELF file parsing.

use core::mem;

use crate::{
    Encoding, Medium,
    class::{ClassBase, UnsupportedClassError},
    file_header::ClassFileHeader,
    ident,
    program_header::ClassProgramHeader,
    raw::{Elf32Header, Elf32ProgramHeader, Elf32Rel, Elf32Rela, Elf32SectionHeader, Elf32Symbol},
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
};

/// A zero-sized object offering methods for safe parsing of 32-bit ELF files.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Class32;

impl ClassBase for Class32 {
    type ClassUsize = u32;
    type ClassIsize = i32;

    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError> {
        if class != ident::Class::CLASS32 {
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
        encoding.parse_u32(offset, medium)
    }

    fn parse_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassIsize {
        encoding.parse_i32(offset, medium)
    }
}

impl ClassFileHeader for Class32 {
    fn elf_kind_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, kind) as u64
    }

    fn machine_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, machine) as u64
    }

    fn version_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, version) as u64
    }

    fn entry_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, entry) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, flags) as u64
    }

    fn header_size_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, ehsize) as u64
    }

    fn program_header_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, phoff) as u64
    }

    fn program_header_count_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, phnum) as u64
    }

    fn program_header_size_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, phentsize) as u64
    }

    fn section_header_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, shoff) as u64
    }

    fn section_header_count_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, shnum) as u64
    }

    fn section_header_size_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, shentsize) as u64
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        mem::offset_of!(Elf32Header, shstrndx) as u64
    }

    fn expected_elf_header_size(self) -> u64 {
        mem::size_of::<Elf32Header>() as u64
    }
}

impl ClassSectionHeader for Class32 {
    fn name_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, name) as u64
    }

    fn kind_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, kind) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, flags) as u64
    }

    fn address_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, addr) as u64
    }

    fn offset_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, offset) as u64
    }

    fn size_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, size) as u64
    }

    fn link_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, link) as u64
    }

    fn info_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, info) as u64
    }

    fn address_align_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, addralign) as u64
    }

    fn entry_size_offset(self) -> u64 {
        mem::offset_of!(Elf32SectionHeader, entsize) as u64
    }

    fn expected_section_header_size(self) -> u64 {
        mem::size_of::<Elf32SectionHeader>() as u64
    }
}

impl ClassSymbol for Class32 {
    fn name_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, name) as u64
    }

    fn value_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, value) as u64
    }

    fn size_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, size) as u64
    }

    fn info_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, info) as u64
    }

    fn other_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, other) as u64
    }

    fn section_header_index_offset(self) -> u64 {
        mem::offset_of!(Elf32Symbol, shndx) as u64
    }

    fn expected_symbol_size(self) -> u64 {
        mem::size_of::<Elf32Symbol>() as u64
    }
}

impl ClassRelocation for Class32 {
    type SymbolIndex = u32;
    type RelocationKind = u8;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        info >> 8
    }

    fn relocation_kind_raw(self, info: Self::ClassUsize) -> Self::RelocationKind {
        (info & 0xFF) as u8
    }

    fn rel_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32Rel, offset) as u64
    }

    fn rel_info_offset(self) -> u64 {
        mem::offset_of!(Elf32Rel, info) as u64
    }

    fn rela_offset_offset(self) -> u64 {
        mem::offset_of!(Elf32Rela, offset) as u64
    }

    fn rela_info_offset(self) -> u64 {
        mem::offset_of!(Elf32Rela, info) as u64
    }

    fn rela_addend_offset(self) -> u64 {
        mem::offset_of!(Elf32Rela, addend) as u64
    }

    fn expected_rel_size(self) -> u64 {
        mem::size_of::<Elf32Rel>() as u64
    }

    fn expected_rela_size(self) -> u64 {
        mem::size_of::<Elf32Rela>() as u64
    }
}

impl ClassProgramHeader for Class32 {
    fn kind_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, kind) as u64
    }

    fn offset_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, offset) as u64
    }

    fn virtual_address_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, vaddr) as u64
    }

    fn physical_address_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, paddr) as u64
    }

    fn file_size_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, filesz) as u64
    }

    fn memory_size_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, memsz) as u64
    }

    fn flags_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, flags) as u64
    }

    fn align_offset(self) -> u64 {
        mem::offset_of!(Elf32ProgramHeader, align) as u64
    }

    fn expected_program_header_size(self) -> u64 {
        mem::size_of::<Elf32ProgramHeader>() as u64
    }
}

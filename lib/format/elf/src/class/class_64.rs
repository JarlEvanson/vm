//! 64-bit ELF file parsing.

use core::mem;

use conversion::usize_to_u64;

use crate::{
    class::{ClassBase, UnsupportedClassError},
    dynamic::ClassDynamic,
    encoding::Encoding,
    header::ClassElfHeader,
    ident,
    medium::{Medium, MediumError},
    program_header::ClassProgramHeader,
    raw::{
        Elf64Dynamic, Elf64Header, Elf64ProgramHeader, Elf64Rel, Elf64Rela, Elf64SectionHeader,
        Elf64Symbol,
    },
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
        if class != ident::Class::CLASS32 {
            return Err(UnsupportedClassError(class));
        }

        Ok(Self)
    }

    fn read_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassUsize, MediumError<M::Error>> {
        encoding.read_u64(offset, medium)
    }

    fn read_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassIsize, MediumError<M::Error>> {
        encoding.read_i64(offset, medium)
    }
}

impl ClassElfHeader for Class64 {
    fn elf_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, kind))
    }

    fn machine_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, machine))
    }

    fn version_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, version))
    }

    fn entry_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, entry))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, flags))
    }

    fn header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, ehsize))
    }

    fn program_header_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, phoff))
    }

    fn program_header_count_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, phnum))
    }

    fn program_header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, phentsize))
    }

    fn section_header_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, shoff))
    }

    fn section_header_count_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, shnum))
    }

    fn section_header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, shentsize))
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Header, shstrndx))
    }

    fn expected_elf_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64Header>())
    }
}

impl ClassSectionHeader for Class64 {
    fn name_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, name))
    }

    fn section_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, kind))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, flags))
    }

    fn address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, addr))
    }

    fn offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, offset))
    }

    fn size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, size))
    }

    fn link_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, link))
    }

    fn info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, info))
    }

    fn address_align_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, addralign))
    }

    fn entry_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64SectionHeader, entsize))
    }

    fn expected_section_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64SectionHeader>())
    }
}

impl ClassProgramHeader for Class64 {
    fn segment_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, kind))
    }

    fn offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, offset))
    }

    fn virtual_address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, vaddr))
    }

    fn physical_address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, paddr))
    }

    fn file_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, filesz))
    }

    fn memory_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, memsz))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, flags))
    }

    fn align_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64ProgramHeader, align))
    }

    fn expected_program_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64ProgramHeader>())
    }
}

impl ClassRelocation for Class64 {
    type SymbolIndex = u32;
    type RelocationType = u32;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        (info >> 32) as u32
    }

    fn relocation_type_raw(self, info: Self::ClassUsize) -> Self::RelocationType {
        (info & 0xFFFF_FFFF) as u32
    }

    fn rel_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Rel, offset))
    }

    fn rel_info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Rel, info))
    }

    fn rela_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Rela, offset))
    }

    fn rela_info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Rela, info))
    }

    fn rela_addend_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Rela, addend))
    }

    fn expected_rel_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64Rel>())
    }

    fn expected_rela_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64Rela>())
    }
}

impl ClassDynamic for Class64 {
    fn dynamic_tag_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Dynamic, tag))
    }

    fn dynamic_val_ptr_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Dynamic, val_ptr))
    }

    fn expected_dynamic_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64Dynamic>())
    }
}

impl ClassSymbol for Class64 {
    fn name_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, name))
    }

    fn value_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, value))
    }

    fn size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, size))
    }

    fn info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, info))
    }

    fn other_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, other))
    }

    fn section_header_index_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf64Symbol, shndx))
    }

    fn expected_symbol_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf64Symbol>())
    }
}

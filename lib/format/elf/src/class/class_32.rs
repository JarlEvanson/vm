//! 32-bit ELF file parsing.

use core::mem;

use crate::{
    class::{ClassBase, UnsupportedClassError},
    dynamic::ClassDynamic,
    encoding::Encoding,
    header::ClassElfHeader,
    ident,
    medium::{Medium, MediumError},
    program_header::ClassProgramHeader,
    raw::{
        Elf32Dynamic, Elf32Header, Elf32ProgramHeader, Elf32Rel, Elf32Rela, Elf32SectionHeader,
        Elf32Symbol,
    },
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
    usize_to_u64,
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

    fn read_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassUsize, MediumError<M::Error>> {
        encoding.read_u32(offset, medium)
    }

    fn read_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassIsize, MediumError<M::Error>> {
        encoding.read_i32(offset, medium)
    }
}

impl ClassElfHeader for Class32 {
    fn elf_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, kind))
    }

    fn machine_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, machine))
    }

    fn version_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, version))
    }

    fn entry_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, entry))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, flags))
    }

    fn header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, ehsize))
    }

    fn program_header_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, phoff))
    }

    fn program_header_count_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, phnum))
    }

    fn program_header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, phentsize))
    }

    fn section_header_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, shoff))
    }

    fn section_header_count_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, shnum))
    }

    fn section_header_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, shentsize))
    }

    fn section_header_string_table_index_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Header, shstrndx))
    }

    fn expected_elf_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32Header>())
    }
}

impl ClassSectionHeader for Class32 {
    fn name_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, name))
    }

    fn section_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, kind))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, flags))
    }

    fn address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, addr))
    }

    fn offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, offset))
    }

    fn size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, size))
    }

    fn link_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, link))
    }

    fn info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, info))
    }

    fn address_align_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, addralign))
    }

    fn entry_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32SectionHeader, entsize))
    }

    fn expected_section_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32SectionHeader>())
    }
}

impl ClassProgramHeader for Class32 {
    fn segment_type_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, kind))
    }

    fn offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, offset))
    }

    fn virtual_address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, vaddr))
    }

    fn physical_address_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, paddr))
    }

    fn file_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, filesz))
    }

    fn memory_size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, memsz))
    }

    fn flags_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, flags))
    }

    fn align_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32ProgramHeader, align))
    }

    fn expected_program_header_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32ProgramHeader>())
    }
}

impl ClassRelocation for Class32 {
    type SymbolIndex = u32;
    type RelocationType = u8;

    fn symbol_index_raw(self, info: Self::ClassUsize) -> Self::SymbolIndex {
        info >> 8
    }

    #[expect(clippy::as_conversions)]
    fn relocation_type_raw(self, info: Self::ClassUsize) -> Self::RelocationType {
        (info & 0xFF) as u8
    }

    fn rel_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Rel, offset))
    }

    fn rel_info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Rel, info))
    }

    fn rela_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Rela, offset))
    }

    fn rela_info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Rela, info))
    }

    fn rela_addend_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Rela, addend))
    }

    fn expected_rel_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32Rel>())
    }

    fn expected_rela_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32Rela>())
    }
}

impl ClassDynamic for Class32 {
    fn dynamic_tag_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Dynamic, tag))
    }

    fn dynamic_val_ptr_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Dynamic, val_ptr))
    }

    fn expected_dynamic_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32Dynamic>())
    }
}

impl ClassSymbol for Class32 {
    fn name_offset_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, name))
    }

    fn value_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, value))
    }

    fn size_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, size))
    }

    fn info_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, info))
    }

    fn other_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, other))
    }

    fn section_header_index_offset(self) -> u64 {
        usize_to_u64(mem::offset_of!(Elf32Symbol, shndx))
    }

    fn expected_symbol_size(self) -> u64 {
        usize_to_u64(mem::size_of::<Elf32Symbol>())
    }
}

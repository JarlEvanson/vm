//! Raw definitions of PE structures.

#![expect(missing_docs, reason = "no need to document raw definitions")]

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct DosHeader {
    pub magic: u16,
    pub cblp: u16,
    pub cp: u16,
    pub crlc: u16,
    pub cparhdr: u16,
    pub minalloc: u16,
    pub maxalloc: u16,
    pub ss: u16,
    pub sp: u16,
    pub csum: u16,
    pub ip: u16,
    pub cs: u16,
    pub lfarlc: u16,
    pub ovno: u16,
    pub res: [u16; 4],
    pub oemid: u16,
    pub oeminfo: u16,
    pub res_2: [u16; 10],
    pub lfanew: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct NtHeaders32 {
    pub signature: u32,
    pub file_header: FileHeader,
    pub optional_header: OptionalHeader32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct NtHeaders64 {
    pub signature: u32,
    pub file_header: FileHeader,
    pub optional_header: OptionalHeader64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FileHeader {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_data_stamp: u32,
    pub symbol_table_ptr: u32,
    pub symbol_count: u32,
    pub optional_header_size: u16,
    pub characteristics: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct OptionalHeader32 {
    pub magic: u16,
    pub linker_major_version: u8,
    pub linker_minor_version: u8,
    pub code_size: u32,
    pub initialized_data_size: u32,
    pub uninitialized_data_size: u32,
    pub entry_point: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,

    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub operating_system_major_version: u16,
    pub operating_system_minor_version: u16,
    pub image_major_version: u16,
    pub image_minor_version: u16,
    pub subsystem_major_version: u16,
    pub subsystem_minor_version: u16,
    pub win32_version_value: u32,
    pub image_size: u32,
    pub header_size: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directories: [DataDirectory; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct OptionalHeader64 {
    pub magic: u16,
    pub linker_major_version: u8,
    pub linker_minor_version: u8,
    pub code_size: u32,
    pub initialized_data_size: u32,
    pub uninitialized_data_size: u32,
    pub entry_point: u32,
    pub base_of_code: u32,

    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub operating_system_major_version: u16,
    pub operating_system_minor_version: u16,
    pub image_major_version: u16,
    pub image_minor_version: u16,
    pub subsystem_major_version: u16,
    pub subsystem_minor_version: u16,
    pub win32_version_value: u32,
    pub image_size: u32,
    pub header_size: u32,
    pub checksum: u32,
    pub subsystem: u16,
    pub dll_characteristics: u16,
    pub size_of_stack_reserve: u64,
    pub size_of_stack_commit: u64,
    pub size_of_heap_reserve: u64,
    pub size_of_heap_commit: u64,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directories: [DataDirectory; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct DataDirectory {
    pub virtual_address: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SectionHeader {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_line_numbers: u32,
    pub number_of_relocations: u16,
    pub number_of_line_numbers: u16,
    pub characteristics: u32,
}

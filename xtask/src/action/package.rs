//! Helper functions to package `revm` and `revm-stub` given a [`PackageConfig`].

use std::{ffi::CStr, fs, mem, path::PathBuf};

use anyhow::Result;
use elf::{
    AnyClass, AnyEndian,
    file_header::Machine,
    program_header::{ProgramHeader, ProgramHeaderTable, SegmentFlags, SegmentKind},
};
use pe::raw::{DataDirectory, FileHeader, NtHeaders64, OptionalHeader64, SectionHeader};

use crate::{
    action::{build_revm::build_revm, build_stub::build_revm_stub},
    cli::{
        build_revm::BuildRevmConfig,
        build_stub::BuildStubConfig,
        package::{CrateConfig, PackageConfig},
    },
    common::Arch,
};

/// Builds `revm` and `revm_stub` as specified by `config`, then packages `revm` together with
/// `revm_stub`.
///
/// # Errors
///
/// Returns errors when the `cargo build` command fails or an error in the packaging process
/// occurs.
pub fn package(config: PackageConfig) -> Result<PathBuf> {
    let stub_path = match config.stub {
        CrateConfig::Path(path) => path,
        CrateConfig::Build { arch, profile } => {
            let stub_config = BuildStubConfig { arch, profile };
            build_revm_stub(stub_config)?
        }
    };

    let revm_path = match config.revm {
        CrateConfig::Path(path) => path,
        CrateConfig::Build { arch, profile } => {
            let revm_config = BuildRevmConfig { arch, profile };
            build_revm(revm_config)?
        }
    };

    let stub = fs::read(stub_path)?;
    let revm = fs::read(revm_path)?;

    let package = create_package(&stub, &revm)?;
    fs::write(&config.output_path, package)?;

    Ok(config.output_path)
}

fn create_package(stub: &[u8], revm: &[u8]) -> Result<Vec<u8>> {
    let mut package = Vec::new();
    let mut pe_header_offset = 64;

    let elf_data = extract_elf_data(stub)?;
    if let Some(linux_efi_header) = elf_data.linux_efi_header {
        package.write_bytes(0, linux_efi_header);
        pe_header_offset = linux_efi_header.len() as u64;
    }

    // We need 2 extra sections (one for .reloc, one for embedding `revm`)
    let section_count = elf_data.load_segments().count() as u16 + 2;
    pe_header_offset = pe_header_offset.next_multiple_of(mem::align_of::<NtHeaders64>() as u64);
    let mut data = PeData {
        pe_header_offset,
        section_count,

        code_size: 0,
        initialized_data_size: 0,
        uninitialized_data_size: 0,
        base_of_code: u32::MAX,
        image_size: 0,

        section_index: 0,
        section_data_offset: 0,
    };
    data.section_data_offset = u32::try_from(data.section_data_start_offset())?;

    for (index, segment) in elf_data.load_segments().enumerate() {
        let mut name = format!(".seg{}", index);
        let virtual_size = segment.memory_size();
        let virtual_address =
            segment.virtual_address() - elf_data.image_base + SECTION_ALIGNMENT as u64;
        let size_of_raw_data = (segment.file_size() as u32).next_multiple_of(FILE_ALIGNMENT);
        let mut characteristics = 0;

        if segment.flags().contains(SegmentFlags::EXECUTE) {
            characteristics |= 0x20;
        } else {
            characteristics |= 0x40;
        }

        if segment.flags().contains(SegmentFlags::READ) {
            characteristics |= 0x4000_0000;
        }
        if segment.flags().contains(SegmentFlags::WRITE) {
            characteristics |= 0x8000_0000;
        }
        if segment.flags().contains(SegmentFlags::EXECUTE) {
            characteristics |= 0x2000_0000;
        }

        assert!(virtual_address.is_multiple_of(u64::from(FILE_ALIGNMENT)));
        let mut header = SectionHeader {
            name: [0; 8],
            virtual_size: u32::try_from(virtual_size).unwrap(),
            virtual_address: u32::try_from(virtual_address).unwrap(),
            size_of_raw_data,
            pointer_to_raw_data: data.section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        // Adjust name to be at most 8 bytes.
        name.truncate(8);
        header.name[..name.len()].copy_from_slice(name.as_bytes());

        let segment_bytes = segment.segment().unwrap_or(&[]);
        package.write_bytes(u64::from(data.section_data_offset), segment_bytes);
        package.fill(
            u64::from(data.section_data_offset) + segment_bytes.len() as u64,
            u64::from(size_of_raw_data) - segment.file_size(),
            0,
        );

        write_section(&mut package, &mut data, header)?;
    }

    {
        let virtual_size = u32::try_from(revm.len() + 8).unwrap();
        let virtual_address = data.image_size;
        let size_of_raw_data = virtual_size.next_multiple_of(FILE_ALIGNMENT);
        let characteristics = 0x4000_0040;

        let header = SectionHeader {
            name: [b'.', b'b', b'l', b'o', b'b', 0, 0, 0],
            virtual_size,
            virtual_address,
            size_of_raw_data,
            pointer_to_raw_data: data.section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        package.write_u64(u64::from(data.section_data_offset), u64::from(virtual_size));
        package.write_bytes(u64::from(data.section_data_offset.strict_add(8)), revm);

        write_section(&mut package, &mut data, header)?;
    }

    let (reloc_addr, reloc_size) = {
        let virtual_size = 8;
        let virtual_address = data.image_size;
        let size_of_raw_data = 8u32.next_multiple_of(FILE_ALIGNMENT);
        let characteristics = 0x4200_0040;

        let header = SectionHeader {
            name: [b'.', b'r', b'e', b'l', b'o', b'c', 0, 0],
            virtual_size,
            virtual_address: u32::try_from(virtual_address).unwrap(),
            size_of_raw_data,
            pointer_to_raw_data: data.section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        package.write_u32(u64::from(data.section_data_offset), data.base_of_code);
        package.write_u32(u64::from(data.section_data_offset.strict_add(4)), 8);
        package.fill(
            u64::from(data.section_data_offset.strict_add(8)),
            (header.pointer_to_raw_data + header.size_of_raw_data - 8) as u64,
            0,
        );

        write_section(&mut package, &mut data, header)?;
        (header.virtual_address, header.virtual_size)
    };

    let file_header = FileHeader {
        machine: match elf_data.arch {
            Arch::Aarch64 => 0xaa64,
            Arch::X86_64 => 0x8664,
        },
        number_of_sections: data.section_count,
        time_data_stamp: 0,
        symbol_table_ptr: 0,
        symbol_count: 0,
        optional_header_size: mem::size_of::<OptionalHeader64>() as u16,
        characteristics: 0x20 | 0x02, // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    };

    let mut data_directories = [const {
        DataDirectory {
            virtual_address: 0,
            size: 0,
        }
    }; 16];

    data_directories[5] = DataDirectory {
        virtual_address: reloc_addr,
        size: reloc_size,
    };
    let optional_header = OptionalHeader64 {
        magic: 0x020b,
        linker_major_version: 0,
        linker_minor_version: 0,
        code_size: data.code_size,
        initialized_data_size: data.initialized_data_size,
        uninitialized_data_size: data.uninitialized_data_size,
        entry_point: u32::try_from(elf_data.relative_entry_point).unwrap(),
        base_of_code: data.base_of_code,

        image_base: 0x10000,
        section_alignment: SECTION_ALIGNMENT,
        file_alignment: FILE_ALIGNMENT,
        operating_system_major_version: 0,
        operating_system_minor_version: 0,
        image_major_version: 0,
        image_minor_version: 0,
        subsystem_major_version: 0,
        subsystem_minor_version: 0,
        win32_version_value: 0,
        image_size: data.image_size,
        header_size: u32::try_from(data.size_of_headers()).unwrap(),
        checksum: 0,
        subsystem: 10,                            // UEFI Application
        dll_characteristics: 0x100 | 0x40 | 0x20, // NX | Movable | High-entropy
        size_of_stack_reserve: 0x100000,
        size_of_stack_commit: 0x1000,
        size_of_heap_reserve: 0x100000,
        size_of_heap_commit: 0x1000,
        loader_flags: 0,
        number_of_rva_and_sizes: 16,
        data_directories,
    };

    let nt_headers_64 = NtHeaders64 {
        signature: u32::from_le_bytes([b'P', b'E', 0, 0]),
        file_header,
        optional_header,
    };

    nt_headers_64.write(data.pe_header_offset, &mut package);
    package.write_bytes(0, b"MZ");
    package.write_u32(60, u32::try_from(data.pe_header_offset).unwrap());

    Ok(package)
}

const SECTION_ALIGNMENT: u32 = 4096;
const FILE_ALIGNMENT: u32 = 512;

struct PeData {
    pe_header_offset: u64,
    section_count: u16,

    code_size: u32,
    initialized_data_size: u32,
    uninitialized_data_size: u32,
    base_of_code: u32,
    image_size: u32,

    section_index: u16,
    section_data_offset: u32,
}

impl PeData {
    fn section_header_start_offset(&self) -> u64 {
        self.pe_header_offset
            .strict_add(mem::size_of::<NtHeaders64>() as u64)
    }

    fn section_data_start_offset(&self) -> u64 {
        let section_header_table_size =
            u64::from(self.section_count).strict_mul(mem::size_of::<SectionHeader>() as u64);
        self.section_header_start_offset()
            .strict_add(section_header_table_size)
            .next_multiple_of(u64::from(SECTION_ALIGNMENT))
    }

    fn size_of_headers(&self) -> u64 {
        self.section_data_start_offset()
    }

    fn next_section_header_offset(&self) -> u64 {
        self.section_header_start_offset()
            + u64::from(self.section_index).strict_mul(mem::size_of::<SectionHeader>() as u64)
    }
}

/// Container for important ELF data.
struct ElfData<'elf> {
    arch: Arch,
    image_base: u64,

    relative_entry_point: u64,
    program_header_table: ProgramHeaderTable<'elf, [u8], AnyClass, AnyEndian>,

    linux_efi_header: Option<&'elf [u8]>,
}

impl<'elf> ElfData<'elf> {
    /// Returns the PT_LOAD segments from the ELF file.
    pub fn load_segments(
        &self,
    ) -> impl Iterator<Item = ProgramHeader<'elf, [u8], AnyClass, AnyEndian>> {
        self.program_header_table
            .into_iter()
            .filter(|header| header.kind() == SegmentKind::LOAD)
    }
}

fn extract_elf_data<'elf>(stub: &'elf [u8]) -> Result<ElfData<'elf>> {
    let elf = elf::File::<_, AnyClass, AnyEndian>::new(stub)?;

    let section_header_table = elf
        .section_header_table()
        .ok_or(anyhow::anyhow!("missing section header table"))?;
    let section_header_string_table_header = section_header_table
        .get(u64::from(elf.header().section_header_string_table_index()))
        .ok_or_else(|| anyhow::anyhow!("invalid section header string table index"))?;
    let section_header_string_table = section_header_string_table_header.section().unwrap();

    let mut linux_efi_header = None;
    for section_header in section_header_table {
        let name_bytes = &section_header_string_table[section_header.name_offset() as usize..];
        let name = CStr::from_bytes_until_nul(name_bytes).unwrap();

        if name == c".linux-efi-header" {
            linux_efi_header = section_header.section();
        }
    }

    let program_header_table = elf
        .program_header_table()
        .ok_or_else(|| anyhow::anyhow!("missing program header table"))?;
    let image_base = program_header_table
        .into_iter()
        .find(|header| header.kind() == SegmentKind::LOAD)
        .map_or(0, |header| header.virtual_address());

    assert!(image_base <= elf.header().entry());
    let relative_entry_point = elf.header().entry() - image_base + u64::from(SECTION_ALIGNMENT);

    let arch = match elf.header().machine() {
        Machine::AARCH64 => Arch::Aarch64,
        Machine::X86_64 => Arch::X86_64,
        _ => anyhow::bail!("only aarch64 and x86_64 are supported"),
    };

    let data = ElfData {
        arch,
        image_base,
        relative_entry_point,
        program_header_table,
        linux_efi_header,
    };

    Ok(data)
}

fn write_section<W: Writer>(
    writer: &mut W,
    pe_data: &mut PeData,
    section: SectionHeader,
) -> Result<()> {
    section.write(u64::from(pe_data.next_section_header_offset()), writer);
    pe_data.section_index += 1;
    pe_data.section_data_offset = pe_data
        .section_data_offset
        .strict_add(section.size_of_raw_data);

    if section.characteristics & 0x20 == 0x20 {
        pe_data.code_size = pe_data.code_size.strict_add(section.size_of_raw_data);
        pe_data.base_of_code = pe_data.base_of_code.min(section.virtual_address);
    } else {
        pe_data.initialized_data_size = pe_data
            .initialized_data_size
            .strict_add(section.size_of_raw_data);
    }

    let end_of_section = section.virtual_address.strict_add(section.virtual_size);
    pe_data.image_size = pe_data
        .image_size
        .max(end_of_section)
        .next_multiple_of(SECTION_ALIGNMENT);

    Ok(())
}

trait Writer {
    fn write_u8(&mut self, offset: u64, value: u8);
    fn write_u16(&mut self, offset: u64, value: u16) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    fn write_u32(&mut self, offset: u64, value: u32) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    fn write_u64(&mut self, offset: u64, value: u64) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    fn write_bytes(&mut self, offset: u64, bytes: &[u8]) {
        let max_offset = offset.wrapping_add(bytes.len() as u64);
        assert!(max_offset >= offset || max_offset == 0);

        for (index, &byte) in bytes.iter().enumerate() {
            self.write_u8(offset + index as u64, byte)
        }
    }

    fn fill(&mut self, offset: u64, len: u64, value: u8) {
        let max_offset = offset.wrapping_add(len);
        assert!(max_offset >= offset || max_offset == 0);

        for index in 0..len {
            self.write_u8(offset + index, value);
        }
    }
}

impl Writer for Vec<u8> {
    fn write_u8(&mut self, offset: u64, value: u8) {
        let offset = usize::try_from(offset).expect("offset is too large");
        let required_size = offset.strict_add(1);
        if required_size > self.len() {
            self.resize(required_size, 0xEE);
        }

        self[offset] = value;
    }
}

trait Writable {
    fn write<W: Writer>(self, offset: u64, writer: &mut W);
}

impl Writable for NtHeaders64 {
    fn write<W: Writer>(self, offset: u64, writer: &mut W) {
        writer.write_u32(offset, self.signature);
        self.file_header.write(offset + 4, writer);
        self.optional_header.write(offset + 24, writer);
    }
}

impl Writable for FileHeader {
    fn write<W: Writer>(self, offset: u64, writer: &mut W) {
        writer.write_u16(offset, self.machine);
        writer.write_u16(offset + 2, self.number_of_sections);

        writer.write_u32(offset + 4, self.time_data_stamp);
        writer.write_u32(offset + 8, self.symbol_table_ptr);
        writer.write_u32(offset + 12, self.symbol_count);

        writer.write_u16(offset + 16, self.optional_header_size);
        writer.write_u16(offset + 18, self.characteristics);
    }
}

impl Writable for OptionalHeader64 {
    fn write<W: Writer>(self, offset: u64, writer: &mut W) {
        writer.write_u16(offset, self.magic);

        writer.write_u8(offset + 2, self.linker_major_version);
        writer.write_u8(offset + 3, self.linker_minor_version);

        writer.write_u32(offset + 4, self.code_size);
        writer.write_u32(offset + 8, self.initialized_data_size);
        writer.write_u32(offset + 12, self.uninitialized_data_size);
        writer.write_u32(offset + 16, self.entry_point);
        writer.write_u32(offset + 20, self.base_of_code);

        writer.write_u64(offset + 24, self.image_base);
        writer.write_u32(offset + 32, self.section_alignment);
        writer.write_u32(offset + 36, self.file_alignment);

        writer.write_u16(offset + 40, self.operating_system_major_version);
        writer.write_u16(offset + 42, self.operating_system_minor_version);
        writer.write_u16(offset + 44, self.image_major_version);
        writer.write_u16(offset + 46, self.image_minor_version);
        writer.write_u16(offset + 48, self.subsystem_major_version);
        writer.write_u16(offset + 50, self.subsystem_minor_version);

        writer.write_u32(offset + 52, self.win32_version_value);

        writer.write_u32(offset + 56, self.image_size);
        writer.write_u32(offset + 60, self.header_size);
        writer.write_u32(offset + 64, self.checksum);
        writer.write_u16(offset + 68, self.subsystem);

        writer.write_u16(offset + 70, self.dll_characteristics);

        writer.write_u64(offset + 72, 0x100000);
        writer.write_u64(offset + 80, 0x1000);
        writer.write_u64(offset + 88, 0x100000);
        writer.write_u64(offset + 96, 0x1000);

        writer.write_u32(offset + 104, self.loader_flags);
        writer.write_u32(offset + 108, self.number_of_rva_and_sizes);

        for (index, directory) in self.data_directories.into_iter().enumerate() {
            directory.write(
                offset + 112 + (index * mem::size_of::<DataDirectory>()) as u64,
                writer,
            );
        }
    }
}

impl Writable for DataDirectory {
    fn write<W: Writer>(self, offset: u64, writer: &mut W) {
        writer.write_u32(offset, self.virtual_address);
        writer.write_u32(offset + 4, self.size);
    }
}

impl Writable for SectionHeader {
    fn write<W: Writer>(self, offset: u64, writer: &mut W) {
        writer.write_bytes(offset, &self.name);
        writer.write_u32(offset + 8, self.virtual_size);
        writer.write_u32(offset + 12, self.virtual_address);
        writer.write_u32(offset + 16, self.size_of_raw_data);
        writer.write_u32(offset + 20, self.pointer_to_raw_data);

        writer.write_u32(offset + 24, self.pointer_to_relocations);
        writer.write_u32(offset + 28, self.pointer_to_line_numbers);
        writer.write_u16(offset + 32, self.number_of_relocations);
        writer.write_u16(offset + 34, self.number_of_line_numbers);

        writer.write_u32(offset + 36, self.characteristics);
    }
}

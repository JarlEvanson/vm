use std::{error, fmt, mem, process::ExitCode};

use elf::{
    class::class_any::AnyClass,
    encoding::AnyEndian,
    header::ElfHeaderError,
    medium::Medium,
    program_header::{ProgramHeader, ProgramHeaderTable, SegmentType},
};
use pe::raw::{DataDirectory, FileHeader, NtHeaders64, OptionalHeader64, SectionHeader};

fn main() -> ExitCode {
    let mut args = std::env::args();
    let executable_name = args.next().unwrap_or_else(|| String::from("package"));
    if args.len() != 3 {
        eprintln!("Usage: {executable_name} <STUB_PATH> <REVM_PATH> <OUTPUT_PATH>");
        return ExitCode::SUCCESS;
    }

    let stub_path = args.next().unwrap();
    let revm_path = args.next().unwrap();
    let output_path = args.next().unwrap();

    let stub = match std::fs::read(&stub_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("error reading from '{stub_path}': {error}");
            return ExitCode::FAILURE;
        }
    };

    let revm = match std::fs::read(&revm_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("error reading from '{revm_path}': {error}");
            return ExitCode::FAILURE;
        }
    };

    let output = Vec::new();

    match std::fs::write(&output_path, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error writing to '{output_path}': {error}");
            ExitCode::FAILURE
        }
    }
}

fn generate_package(
    output: &mut Vec<u8>,
    stub: &[u8],
    revm: &[u8],
) -> Result<(), GeneratePackageError> {
    let mut pe_header_offset = 64;

    let elf_data = extract_elf_data(stub)?;

    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeneratePackageError {}

impl fmt::Display for GeneratePackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl error::Error for GeneratePackageError {}

/// Container for important ELF data.
struct ElfData<'elf> {
    /// The architecture for which the ELF file is targeted.
    arch: Arch,
    /// The virtual address at which the ELF file is targeted.
    image_base: u64,

    /// The offset from the [`ElfData::image_base`] at which the entry point is located.
    relative_entry_point: u64,
    /// The program header table corresponding to the ELF file.
    program_header_table: ProgramHeaderTable<'elf, [u8], AnyClass, AnyEndian>,

    /// The contents of a particular section useful for creating a binary capable of being booted
    /// using UEFI and the Linux boot protocol for the architecture.
    linux_efi_header: Option<&'elf [u8]>,
}

impl<'elf> ElfData<'elf> {
    /// Returns the PT_LOAD segments from the ELF file.
    pub fn load_segments(
        &self,
    ) -> impl Iterator<Item = ProgramHeader<'elf, [u8], AnyClass, AnyEndian>> {
        self.program_header_table.into_iter().filter(|header| {
            header
                .segment_type()
                .is_ok_and(|segment_type| segment_type == SegmentType::LOAD)
        })
    }
}

/// Extracts all necessary data required to structure the ELF file from `stub`.
fn extract_elf_data<'elf>(
    stub: &'elf [u8],
) -> Result<ElfData<'elf>, ExtractElfDataError<<[u8] as Medium>::Error>> {
    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExtractElfDataError<E> {
    HeaderError(ElfHeaderError<E>),
}

impl<E: fmt::Display> fmt::Display for ExtractElfDataError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for ExtractElfDataError<E> {}

enum Arch {
    Aarch64,
    I686,
    X86_64,
}

/// Abstraction over a device capable of writing at arbitrary offsets.
trait Writer {
    /// Writes the provided `u8` at `offset`.
    fn write_u8(&mut self, offset: u64, value: u8);

    /// Writes the provided `u16` at `offset`.
    fn write_u16(&mut self, offset: u64, value: u16) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    /// Writes the provided `u32` at `offset`.
    fn write_u32(&mut self, offset: u64, value: u32) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    /// Writes the provided `u64` at `offset`.
    fn write_u64(&mut self, offset: u64, value: u64) {
        let buf = value.to_le_bytes();
        self.write_bytes(offset, &buf);
    }

    /// Writes the provided `bytes` at `offset`.
    fn write_bytes(&mut self, offset: u64, bytes: &[u8]) {
        let max_offset = offset.wrapping_add(bytes.len() as u64);
        assert!(max_offset >= offset || max_offset == 0);

        for (index, &byte) in bytes.iter().enumerate() {
            self.write_u8(offset + index as u64, byte)
        }
    }

    /// Fills the region described by `offset` and `len` with the provided `value`.
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

/// Abstraction over structures that know how to write themselves using [`Writer`].
trait Writable {
    /// Writes themself into `writer` at `offset`.
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

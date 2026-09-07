use std::{mem, process::ExitCode};

use conversion::{usize_to_u16_strict, usize_to_u32_strict};
use elf::{
    class::class_any::AnyClass,
    encoding::AnyEndian,
    header::Machine,
    program_header::{SegmentFlags, SegmentType},
};
use pe::raw::{DataDirectory, FileHeader, NtHeaders64, OptionalHeader64, SectionHeader};

/// The alignment of sections with in the PE file.
const SECTION_ALIGNMENT: u32 = 4096;
/// The alignment of sections in memory.
const FILE_ALIGNMENT: u32 = 512;

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

    let mut output = Vec::new();
    if generate_package(&mut output, &stub, &revm) == ExitCode::FAILURE {
        return ExitCode::FAILURE;
    }

    match std::fs::write(&output_path, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error writing to '{output_path}': {error}");
            ExitCode::FAILURE
        }
    }
}

fn generate_package(output: &mut Vec<u8>, stub: &[u8], revm: &[u8]) -> ExitCode {
    let mut package = Vec::new();
    let pe_header_offset = 64u32;

    let elf = match elf::Elf::<_, AnyClass, AnyEndian>::new(stub) {
        Ok(elf) => elf,
        Err(error) => {
            eprintln!("error parsing revm-stub: {error}");
            return ExitCode::FAILURE;
        }
    };

    let program_header_table = match elf.program_header_table() {
        Ok(program_header_table) => {
            if let Some(program_header_table) = program_header_table {
                program_header_table
            } else {
                eprintln!("error locating program header table: not included");
                eprintln!("aborting due to missing required component");
                return ExitCode::FAILURE;
            }
        }
        Err(error) => {
            eprintln!("error locating program header table: {error}");
            return ExitCode::FAILURE;
        }
    };

    let (load_segment_count, image_base) = 'image_base: {
        let mut load_segment_count = 0u16;
        let mut image_base = None;
        for (index, program_header) in program_header_table.into_iter().enumerate() {
            let segment_type = match program_header.segment_type() {
                Ok(segment_type) => segment_type,
                Err(error) => {
                    eprintln!("error accessing segment type of segment {index}: {error}");
                    eprintln!("skipping segment {index}");
                    continue;
                }
            };

            if segment_type == SegmentType::LOAD {
                let Some(new_load_segment_count) = load_segment_count.checked_add(1) else {
                    eprintln!("aborting since there are too many segments of segment type LOAD");
                    return ExitCode::SUCCESS;
                };
                load_segment_count = new_load_segment_count;

                if image_base.is_some() {
                    continue;
                }

                match program_header.virtual_address() {
                    Ok(virtual_address) => image_base = Some(virtual_address),
                    Err(error) => {
                        eprintln!("error accessing first segment of SegmentType::LOAD: {error}");
                        eprintln!("aborting due to malformed ELF file");
                        return ExitCode::FAILURE;
                    }
                }
            }
        }

        if let Some(image_base) = image_base {
            break 'image_base (load_segment_count, image_base);
        }

        eprintln!("error accessing first segment of SegmentType::LOAD: not found");
        eprintln!("aborting due to unloadable ELF file");
        return ExitCode::FAILURE;
    };

    let Some(section_count) = load_segment_count.checked_add(2) else {
        eprintln!("aborting since there are too many segments of segment type LOAD");
        return ExitCode::FAILURE;
    };

    let Some(section_header_start_offset) =
        pe_header_offset.checked_add(usize_to_u32_strict(mem::size_of::<NtHeaders64>()))
    else {
        eprintln!("'.linux-efi-header' is too large");
        return ExitCode::FAILURE;
    };

    let Some(section_header_table_size) =
        u32::from(section_count).checked_mul(usize_to_u32_strict(mem::size_of::<SectionHeader>()))
    else {
        eprintln!("section table size is too large: too many segments");
        return ExitCode::FAILURE;
    };

    let mut code_size = 0;
    let mut initialized_data_size = 0;
    let uninitialized_data_size = 0;
    let mut base_of_code = u32::MAX;
    let mut image_size = 0;
    let mut section_index = 0;

    let Some(header_size) = section_header_start_offset
        .checked_add(section_header_table_size)
        .and_then(|offset| offset.checked_next_multiple_of(FILE_ALIGNMENT))
    else {
        eprintln!("header size is too large: reduce size of load segments");
        return ExitCode::FAILURE;
    };

    let Some(base_virtual_address) = header_size.checked_next_multiple_of(SECTION_ALIGNMENT) else {
        eprintln!("section data offset is too large: reduce size of load segments");
        return ExitCode::FAILURE;
    };

    let mut section_data_offset = base_virtual_address;
    for (index, program_header) in program_header_table.into_iter().enumerate() {
        let segment_type = match program_header.segment_type() {
            Ok(segment_type) => segment_type,
            Err(error) => {
                eprintln!(
                    "aborting due to error accessing segment type of segment {index}: {error}"
                );
                continue;
            }
        };

        if segment_type != SegmentType::LOAD {
            continue;
        }

        let name = format!(".seg{index}");
        let virtual_address = match program_header.virtual_address() {
            Ok(virtual_address) => virtual_address,
            Err(error) => {
                eprintln!(
                    "aborting due to error accessing virtual address of segment {index}: {error}"
                );
                return ExitCode::FAILURE;
            }
        };

        let Some(virtual_address) = virtual_address.checked_sub(image_base) else {
            eprintln!("virtual address of segment {index} is less than image base");
            eprintln!("aborting due to malformed ELF file");
            return ExitCode::FAILURE;
        };

        let Some(virtual_address) = virtual_address.checked_add(u64::from(base_virtual_address))
        else {
            eprintln!("virtual address of segment {index} exceeds the allowed size");
            eprintln!("aborting due to unsupported ELF file");
            return ExitCode::FAILURE;
        };

        let virtual_address = match u32::try_from(virtual_address) {
            Ok(virtual_address) => virtual_address,
            Err(error) => {
                eprintln!(
                    "aborting due to virtual address of segment {index} being too large: {error}"
                );
                return ExitCode::FAILURE;
            }
        };

        let virtual_size = match program_header.memory_size() {
            Ok(virtual_size) => virtual_size,
            Err(error) => {
                eprintln!(
                    "aborting due to error accessing memory size of segment {index}: {error}"
                );
                return ExitCode::FAILURE;
            }
        };

        let virtual_size = match u32::try_from(virtual_size) {
            Ok(virtual_size) => virtual_size,
            Err(error) => {
                eprintln!(
                    "aborting due to virtual size of segment {index} being too large: {error}"
                );
                return ExitCode::FAILURE;
            }
        };

        let file_size = match program_header.file_size() {
            Ok(file_size) => file_size,
            Err(error) => {
                eprintln!("aborting due to error accessing file size of segment {index}: {error}");
                return ExitCode::FAILURE;
            }
        };

        let file_size = match u32::try_from(file_size) {
            Ok(file_size) => file_size,
            Err(error) => {
                eprintln!("aborting due to file size of segment {index} being too large: {error}");
                return ExitCode::FAILURE;
            }
        };

        let Some(file_size) = file_size.checked_next_multiple_of(FILE_ALIGNMENT) else {
            eprintln!("aborting due to file size of segment {index} being too large");
            return ExitCode::FAILURE;
        };

        let segment_flags = match program_header.flags() {
            Ok(segment_flags) => segment_flags,
            Err(error) => {
                eprintln!("aborting due to error accessing flags of segment {index}: {error}");
                return ExitCode::FAILURE;
            }
        };

        let mut characteristics = 0;

        if segment_flags.contains(SegmentFlags::EXECUTE) {
            characteristics |= 0x2000_0020;
        } else {
            characteristics |= 0x40;
        }

        if segment_flags.contains(SegmentFlags::READ) {
            characteristics |= 0x4000_0000;
        }
        if segment_flags.contains(SegmentFlags::WRITE) {
            characteristics |= 0x8000_0000;
        }

        let mut header = SectionHeader {
            name: [0; 8],
            virtual_size,
            virtual_address,
            size_of_raw_data: file_size,
            pointer_to_raw_data: section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        // Force name to be at most 8 bytes (this means up to 10,000 segments are supported).
        assert!(name.len() <= 8);
        header.name[..name.len()].copy_from_slice(name.as_bytes());

        let segment_bytes = match program_header.segment() {
            Ok(segment) => segment,
            Err(error) => {
                eprintln!("aborting due to error accessing data of segment {index}: {error}");
                return ExitCode::FAILURE;
            }
        };

        package.fill(u64::from(section_data_offset), u64::from(file_size), 0);
        package.write_bytes(u64::from(section_data_offset), segment_bytes);

        let result = write_section_header(
            &mut package,
            section_header_start_offset,
            section_index,
            &mut section_data_offset,
            &mut code_size,
            &mut base_of_code,
            &mut initialized_data_size,
            &mut image_size,
            header,
        );
        if result == ExitCode::FAILURE {
            return ExitCode::FAILURE;
        }
        section_index += 1;
    }

    {
        let virtual_address = image_size;
        let virtual_size = match u32::try_from(revm.len()) {
            Ok(virtual_size) => virtual_size,
            Err(error) => {
                eprintln!("aborting due to provided blob being too large: {error}");
                return ExitCode::FAILURE;
            }
        };

        let Some(virtual_size) = virtual_size.checked_add(8) else {
            eprintln!("aborting due to provided blob being too large");
            return ExitCode::FAILURE;
        };

        let Some(file_size) = virtual_size.checked_next_multiple_of(FILE_ALIGNMENT) else {
            eprintln!("aborting due to provided blob being too large");
            return ExitCode::FAILURE;
        };

        let characteristics = 0x4000_0040;

        let header = SectionHeader {
            name: [b'.', b'b', b'l', b'o', b'b', 0, 0, 0],
            virtual_size,
            virtual_address,
            size_of_raw_data: file_size,
            pointer_to_raw_data: section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        package.fill(u64::from(section_data_offset), u64::from(file_size), 0);
        package.write_u64(u64::from(section_data_offset), u64::from(virtual_size - 8));
        // Overflow will never happen since the previous `fill` has already validated the
        // calculation.
        package.write_bytes(u64::from(section_data_offset.strict_add(8)), revm);

        let result = write_section_header(
            &mut package,
            section_header_start_offset,
            section_index,
            &mut section_data_offset,
            &mut code_size,
            &mut base_of_code,
            &mut initialized_data_size,
            &mut image_size,
            header,
        );
        if result == ExitCode::FAILURE {
            return ExitCode::FAILURE;
        }
        section_index += 1;
    }

    let (relocation_address, relocation_size) = {
        let virtual_address = image_size;
        let virtual_size = 8;
        let file_size = 8u32.next_multiple_of(FILE_ALIGNMENT);
        let characteristics = 0x4200_0040;

        let header = SectionHeader {
            name: [b'.', b'r', b'e', b'l', b'o', b'c', 0, 0],
            virtual_size,
            virtual_address,
            size_of_raw_data: file_size,
            pointer_to_raw_data: section_data_offset,

            pointer_to_relocations: 0,
            pointer_to_line_numbers: 0,
            number_of_relocations: 0,
            number_of_line_numbers: 0,

            characteristics,
        };

        package.fill(u64::from(section_data_offset), u64::from(file_size), 0);
        package.write_u32(u64::from(section_data_offset), base_of_code);
        // TODO: Handle overflow better.
        package.write_u32(u64::from(section_data_offset.strict_add(4)), virtual_size);

        let result = write_section_header(
            &mut package,
            section_header_start_offset,
            section_index,
            &mut section_data_offset,
            &mut code_size,
            &mut base_of_code,
            &mut initialized_data_size,
            &mut image_size,
            header,
        );
        if result == ExitCode::FAILURE {
            return ExitCode::FAILURE;
        }

        (header.virtual_address, header.virtual_size)
    };

    let arch = match elf.header().machine() {
        Ok(machine) => match machine {
            Machine::AARCH64 => Arch::Aarch64,
            Machine::INTEL_386 => Arch::I686,
            Machine::X86_64 => Arch::X86_64,
            _ => {
                eprintln!("aborting due to unsupported target machine: {machine:?}");
                return ExitCode::FAILURE;
            }
        },
        Err(error) => {
            eprintln!("aborting due to error accessing target machine: {error}");
            return ExitCode::FAILURE;
        }
    };

    let entry = match elf.header().entry() {
        Ok(entry) => entry,
        Err(error) => {
            eprintln!("aborting due to error accessing entry point: {error}");
            return ExitCode::FAILURE;
        }
    };

    let Some(entry) = entry.checked_sub(image_base) else {
        eprintln!("aborting since entry is outside of image");
        return ExitCode::FAILURE;
    };

    let Some(entry) = entry.checked_add(u64::from(base_virtual_address)) else {
        eprintln!("virtual address of entry point execeeds allowed address range");
        eprintln!("aborting due to unsupported ELF file");
        return ExitCode::FAILURE;
    };

    let entry = match u32::try_from(entry) {
        Ok(entry) => entry,
        Err(error) => {
            eprintln!("error converting entry point: {error}");
            return ExitCode::FAILURE;
        }
    };

    let file_header = FileHeader {
        machine: match arch {
            Arch::Aarch64 => 0xaa64,
            Arch::I686 => 0x014c,
            Arch::X86_64 => 0x8664,
        },
        number_of_sections: section_count,
        time_data_stamp: 0,
        symbol_table_ptr: 0,
        symbol_count: 0,
        optional_header_size: usize_to_u16_strict(mem::size_of::<OptionalHeader64>()),
        characteristics: 0x20 | 0x02, // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    };

    let mut data_directories = [DataDirectory {
        virtual_address: 0,
        size: 0,
    }; 16];
    data_directories[5] = DataDirectory {
        virtual_address: relocation_address,
        size: relocation_size,
    };

    let optional_header = OptionalHeader64 {
        magic: 0x020b,
        linker_major_version: 0,
        linker_minor_version: 0,
        code_size,
        initialized_data_size,
        uninitialized_data_size,

        entry_point: entry,
        base_of_code,

        image_base: 0x1_0000,
        section_alignment: SECTION_ALIGNMENT,
        file_alignment: FILE_ALIGNMENT,
        operating_system_major_version: 0,
        operating_system_minor_version: 0,
        image_major_version: 0,
        image_minor_version: 0,
        subsystem_major_version: 0,
        subsystem_minor_version: 0,
        win32_version_value: 0,
        image_size,
        header_size,
        checksum: 0,
        subsystem: 10,                            // UEFI Application.
        dll_characteristics: 0x100 | 0x40 | 0x20, // NX | Movable | High-entropy
        size_of_stack_reserve: 0x10_0000,
        size_of_stack_commit: 0x1000,
        size_of_heap_reserve: 0x10_0000,
        size_of_heap_commit: 0x1000,
        loader_flags: 0,
        number_of_rva_and_sizes: 16,
        data_directories,
    };

    let nt_headers = NtHeaders64 {
        signature: u32::from_le_bytes([b'P', b'E', 0, 0]),
        file_header,
        optional_header,
    };

    // Write the NT Header structure.
    nt_headers.write(u64::from(pe_header_offset), &mut package);

    // Ensure the DOS header structure is initialized (as much as required by most UEFI
    // implementations).
    package.write_bytes(0, b"MZ");
    package.write_u32(60, pe_header_offset);

    *output = package;
    ExitCode::SUCCESS
}

#[expect(clippy::too_many_arguments)]
fn write_section_header<W: Writer>(
    writer: &mut W,
    section_header_start_offset: u32,
    section_index: u16,
    section_data_offset: &mut u32,
    code_size: &mut u32,
    base_of_code: &mut u32,
    initialized_data_size: &mut u32,
    image_size: &mut u32,
    section_header: SectionHeader,
) -> ExitCode {
    // These calculations cannot fail since we have previously calculated the offset of the end of
    // the section header table.
    let section_header_offset =
        u32::from(section_index).strict_mul(usize_to_u32_strict(mem::size_of::<SectionHeader>()));
    let section_header_offset = section_header_start_offset.strict_add(section_header_offset);

    section_header.write(u64::from(section_header_offset), writer);
    *section_data_offset = match section_data_offset.checked_add(section_header.size_of_raw_data) {
        Some(new_section_data_offset) => new_section_data_offset,
        None => {
            eprintln!("error calculating section data offset: overflowed");
            return ExitCode::FAILURE;
        }
    };

    if section_header.characteristics & 0x20 != 0 {
        *code_size = code_size.saturating_add(section_header.size_of_raw_data);
        *base_of_code = (*base_of_code).min(section_header.virtual_address);
    } else {
        *initialized_data_size =
            initialized_data_size.saturating_add(section_header.size_of_raw_data);
    }

    let end_of_section = section_header
        .virtual_address
        .strict_add(section_header.virtual_size);
    *image_size = (*image_size)
        .max(end_of_section)
        .checked_next_multiple_of(SECTION_ALIGNMENT)
        .unwrap_or(u32::MAX - SECTION_ALIGNMENT + 1);

    ExitCode::SUCCESS
}

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

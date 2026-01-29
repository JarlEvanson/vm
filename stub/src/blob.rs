//! Utilities for obtaining and interacting with the blob.

use core::{error, fmt, mem, ptr, slice};

use elf::{
    class::class_any::AnyClass,
    dynamic::{ClassDynamic, DynamicTable, DynamicTag},
    encoding::AnyEndian,
    header::{ElfHeaderError, ElfType, Machine},
    ident::Encoding,
    medium::MediumError,
    program_header::{SegmentFlags, SegmentType},
    relocation::Rela,
    table::TableItem,
};

#[cfg(target_arch = "x86_64")]
use crate::platform::{read_u64_at, write_u64_at};
use crate::{
    arch::{
        AddressSpaceImpl,
        generic::address_space::{AddressSpace, MapError, ProtectionFlags},
        relocate,
    },
    debug,
    platform::{
        AllocationPolicy, OutOfMemory, allocate_frames, frame_size, read_bytes_at, write_bytes_at,
        write_u8_at,
    },
    trace, warn,
};

unsafe extern "C" {
    static _blob_start: u8;
}

/// Returns a slice that represents the embedded blob.
fn extract_blob() -> &'static [u8] {
    let blob_start_ptr = ptr::addr_of!(_blob_start);
    // SAFETY:
    //
    // When the program is properly packaged, this read is valid since the blob section will be
    // filled with at least 8 bytes.
    let blob_size = unsafe { blob_start_ptr.cast::<u64>().read() };
    let blob_size = usize::try_from(blob_size).expect("blob is too large");

    let blob_ptr = blob_start_ptr.wrapping_byte_add(mem::size_of::<u64>());

    // SAFETY:
    //
    // When the program is properly packaged, this operation is valid since the blob size indicator
    // must be correct and the blob section shall not be editable.
    unsafe { slice::from_raw_parts(blob_ptr, blob_size) }
}

/// Loads the executable contained in the blob into the `address_space`.
pub fn load() -> Result<(AddressSpaceImpl, u64), LoadExecutableError> {
    let blob = extract_blob();
    let elf = elf::Elf::<_, AnyClass, AnyEndian>::new(blob)?;

    let mut address_space = match elf.header().machine()? {
        #[cfg(target_arch = "x86_64")]
        Machine::X86_64 => AddressSpaceImpl::new_current(read_u64_at, write_u64_at)?,
        #[cfg(target_arch = "aarch64")]
        Machine::AARCH64 => todo!(),
        machine => return Err(machine.into()),
    };

    let program_headers = elf
        .program_header_table()?
        .ok_or(LoadExecutableError::MissingProgramHeaderTable)?;

    let slide = match elf.header().elf_type()? {
        ElfType::EXECUTABLE => 0,
        ElfType::SHARED => {
            let mut min_address = u64::MAX;
            let mut max_address = u64::MIN;
            let mut alignment = address_space.page_size();

            for header in program_headers.into_iter().filter(|program_header| {
                program_header
                    .segment_type()
                    .is_ok_and(|segment_type| segment_type == SegmentType::LOAD)
            }) {
                alignment = alignment.max(header.alignment()?);

                min_address = min_address.min(header.virtual_address()?);
                max_address =
                    max_address.max(header.virtual_address()?.strict_add(header.memory_size()?));
            }

            let aligned_min_address = min_address - min_address % alignment;
            let aligned_max_address = max_address
                .checked_next_multiple_of(alignment)
                .ok_or(LoadExecutableError::ExecutableTooLarge)?;
            let byte_span = aligned_max_address - aligned_min_address;
            let base = address_space.max_virtual_address() - byte_span;

            base - base % alignment
        }
        elf_type => return Err(elf_type.into()),
    };

    debug!("Slide: {slide:X}");
    for (index, header) in program_headers.into_iter().enumerate() {
        match header.segment_type()? {
            SegmentType::LOAD => {
                let start_address = slide + header.virtual_address()?;
                let end_address = start_address + header.memory_size()?;

                // Page aligned addresses and total bytes on mapped pages.
                let aligned_start_address =
                    start_address - start_address % address_space.page_size();
                let aligned_end_address = end_address.next_multiple_of(address_space.page_size());
                let page_bytes = aligned_end_address - aligned_start_address;

                // Total number of frames required for page mapping.
                let required_frames = page_bytes.div_ceil(frame_size() as u64);
                let frame_allocation = allocate_frames(
                    required_frames,
                    AllocationPolicy::Below(address_space.max_physical_address()),
                )?;

                let offset = start_address - aligned_start_address;
                let mut protection = ProtectionFlags::READ;
                if header.flags()?.0 & SegmentFlags::WRITE.0 == SegmentFlags::WRITE.0 {
                    protection |= ProtectionFlags::WRITE;
                }
                if header.flags()?.0 & SegmentFlags::EXECUTE.0 == SegmentFlags::EXECUTE.0 {
                    protection |= ProtectionFlags::EXECUTE;
                }

                trace!(
                    "Segment {index} loaded at {start_address:#x} ({:#x})",
                    frame_allocation.physical_address() + offset
                );
                address_space.map(
                    aligned_start_address,
                    frame_allocation.physical_address(),
                    page_bytes / address_space.page_size(),
                    protection,
                )?;

                let file_bytes = header.segment().unwrap_or(&[]);

                write_bytes_at(frame_allocation.physical_address() + offset, file_bytes);

                let zero_base =
                    frame_allocation.physical_address() + offset + file_bytes.len() as u64;
                for i in 0..(header.memory_size()? - file_bytes.len() as u64) {
                    write_u8_at(zero_base + i, 0);
                }

                // Forget the [`FrameAllocation`] to prevent early freeing.
                mem::forget(frame_allocation);
            }
            SegmentType::NULL
            | SegmentType::DYNAMIC
            | SegmentType::INTERP
            | SegmentType::NOTE
            | SegmentType::TLS
            | SegmentType::PHDR => {}
            segment_type => warn!("unknown segment type: {segment_type:?}"),
        }
    }

    for header in program_headers.into_iter().filter(|header| {
        header
            .segment_type()
            .is_ok_and(|segment_type| segment_type == SegmentType::DYNAMIC)
    }) {
        let data = header.segment()?;

        let mut rela_table = None;
        let mut rela_table_size = None;
        let mut rela_entry_size = None;

        let Some(dynamic_table) = DynamicTable::<_, AnyClass, AnyEndian>::new(
            elf.header().class(),
            elf.header().encoding(),
            data,
            0,
            data.len() as u64 / elf.header().class().expected_dynamic_size(),
            elf.header().class().expected_dynamic_size(),
        ) else {
            continue;
        };

        for dynamic in dynamic_table {
            if dynamic.tag()? == DynamicTag::RELA_TABLE {
                trace!("found rela table offset: {}", dynamic.val_ptr()?);
                rela_table.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::RELA_SIZE {
                trace!("found rela table size: {}", dynamic.val_ptr()?);
                rela_table_size.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::RELA_ENTRY_SIZE {
                trace!("found rela entry size: {}", dynamic.val_ptr()?);
                rela_entry_size.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::NULL {
                break;
            }
        }

        let Some(rela_table) = rela_table else {
            warn!("dynamic table missing rela table");
            continue;
        };

        let rela_table_size = rela_table_size.ok_or(LoadExecutableError::MissingRelaTableSize)?;
        let rela_entry_size = rela_entry_size.ok_or(LoadExecutableError::MissingRelaEntrySize)?;

        let num_entries = rela_table_size / rela_entry_size;

        debug!("executing {num_entries} relocation entries");
        for index in 0..num_entries {
            let mut buffer = [0; 128];
            assert!(rela_entry_size <= buffer.len() as u64);

            let rela_entry_virtual_address = slide + rela_table + index * rela_entry_size;
            let rela_entry_physical_address = address_space
                .translate_virt(rela_entry_virtual_address)
                .map_err(|_| LoadExecutableError::OutOfBoundsRelocation)?;
            read_bytes_at(
                rela_entry_physical_address,
                &mut buffer[..rela_entry_size as usize],
            );
            let rela = Rela::new_panicking(
                elf.header().class(),
                elf.header().encoding(),
                0,
                buffer.as_slice(),
            );

            let relocation_info = RelocationInfo {
                relocation_type: rela.relocation_type()?,
                addend: rela.addend()?,
                slide,
            };

            let byte_count;
            let mut buffer = [0; 8];
            match relocate(&relocation_info)
                .map_err(|()| LoadExecutableError::UnsupportedRelocation)?
            {
                FinalizedRelocation::Bits16(value) => {
                    match elf.header().ident()?.encoding()? {
                        Encoding::LSB2 => buffer[..2].copy_from_slice(&value.to_le_bytes()),
                        Encoding::MSB2 => buffer[..2].copy_from_slice(&value.to_be_bytes()),
                        _ => todo!(),
                    };
                    byte_count = 2;
                }
                FinalizedRelocation::Bits32(value) => {
                    match elf.header().ident()?.encoding()? {
                        Encoding::LSB2 => buffer[..4].copy_from_slice(&value.to_le_bytes()),
                        Encoding::MSB2 => buffer[..4].copy_from_slice(&value.to_be_bytes()),
                        _ => todo!(),
                    }
                    byte_count = 4;
                }
                FinalizedRelocation::Bits64(value) => {
                    match elf.header().ident()?.encoding()? {
                        Encoding::LSB2 => buffer.copy_from_slice(&value.to_le_bytes()),
                        Encoding::MSB2 => buffer.copy_from_slice(&value.to_be_bytes()),
                        _ => todo!(),
                    }
                    byte_count = 8;
                }
            }

            let relocation_virtual_address = slide + rela.offset()?;
            let relocation_physical_address = address_space
                .translate_virt(relocation_virtual_address)
                .map_err(|_| LoadExecutableError::OutOfBoundsRelocation)?;
            write_bytes_at(relocation_physical_address, &buffer[..byte_count]);
        }
    }

    debug!("entry point at {:#x}", slide + elf.header().entry()?);
    Ok((address_space, slide + elf.header().entry()?))
}

/// An error occurred loading `revm` into its address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadExecutableError {
    /// An error occurred when accessing the [`Medium`].
    MediumError(MediumError<core::convert::Infallible>),
    /// The ELF file failed to be parsed.
    UnsupportedElfFile(ElfHeaderError<core::convert::Infallible>),
    /// The [`Machine`] of the ELF file was not supported.
    UnsupportedMachine(Machine),
    /// The ELF file is missing its program header table, which means it cannot be loaded.
    MissingProgramHeaderTable,
    /// The ELF file's [`ElfType`] was not supported.
    UnsupportedFileType(ElfType),
    /// Allocation failed when attempting to acquire memory into which the executable's segment
    /// data would be copied.
    AllocationError(OutOfMemory),
    /// An error occurred while mapping the executable's segment data into the alternative address
    /// space.
    MapError(MapError),
    /// The executable to be loaded is too large.
    ExecutableTooLarge,
    /// The executable has a RELA_TABLE tag but not a RELA_SIZE tag.
    MissingRelaTableSize,
    /// The executable has a RELA_TABLE tag but not a RELA_ENTRY_SIZE tag.
    MissingRelaEntrySize,
    /// The executable has an unsupported relocation type.
    UnsupportedRelocation,
    /// A relocation contained in the executable attempted to perform an out-of-bounds relocation.
    OutOfBoundsRelocation,
}

impl From<MediumError<core::convert::Infallible>> for LoadExecutableError {
    fn from(value: MediumError<core::convert::Infallible>) -> Self {
        Self::MediumError(value)
    }
}

impl From<ElfHeaderError<core::convert::Infallible>> for LoadExecutableError {
    fn from(value: ElfHeaderError<core::convert::Infallible>) -> Self {
        Self::UnsupportedElfFile(value)
    }
}

impl From<Machine> for LoadExecutableError {
    fn from(value: Machine) -> Self {
        Self::UnsupportedMachine(value)
    }
}

impl From<ElfType> for LoadExecutableError {
    fn from(value: ElfType) -> Self {
        Self::UnsupportedFileType(value)
    }
}

impl From<OutOfMemory> for LoadExecutableError {
    fn from(value: OutOfMemory) -> Self {
        Self::AllocationError(value)
    }
}

impl From<MapError> for LoadExecutableError {
    fn from(value: MapError) -> Self {
        Self::MapError(value)
    }
}

impl fmt::Display for LoadExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediumError(error) => write!(f, "error accessing medium: {error}"),
            Self::UnsupportedElfFile(error) => write!(f, "error parsing executable: {error}"),
            Self::UnsupportedMachine(machine) => {
                write!(f, "executable machine type not supported: {machine:?}")
            }
            Self::MissingProgramHeaderTable => {
                write!(f, "executable file is missing program header table")
            }
            Self::UnsupportedFileType(file_type) => {
                write!(f, "executable file kind not supported: {file_type:?}")
            }
            Self::AllocationError(error) => write!(
                f,
                "allocation of backing memory for segment failed: {error}"
            ),
            Self::MapError(error) => write!(f, "error occurred mapping segment data: {error}"),
            Self::ExecutableTooLarge => {
                write!(f, "executable's requested address space is too large")
            }
            Self::MissingRelaTableSize => write!(f, "rela table tag is missing table size"),
            Self::MissingRelaEntrySize => write!(f, "rela table tag is missing entry size"),
            Self::OutOfBoundsRelocation => {
                write!(f, "relocation attempted modification outside of executable")
            }
            Self::UnsupportedRelocation => write!(f, "unsupported relocation type"),
        }
    }
}

impl error::Error for LoadExecutableError {}

/// Information required to properly handle a relocation entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelocationInfo {
    /// THe type of relocation to perform.
    pub relocation_type: u32,
    /// The value stored for use in a relocation.
    pub addend: i64,

    /// The slide of the relocated executable.
    pub slide: u64,
}

/// The size and value of a resolved relocation.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum FinalizedRelocation {
    /// Write the given 16-bits at the address.
    Bits16(u16),
    /// Write the given 32-bits at the address.
    Bits32(u32),
    /// Write the given 64-bits at the address.
    Bits64(u64),
}

//! Functionality related to handling relocations.

use core::{error, fmt};

use conversion::{u64_to_usize_strict, usize_to_u64};
use elf::{
    class::class_any::AnyClass,
    dynamic::{ClassDynamic, DynamicTable, DynamicTag},
    encoding::AnyEndian,
    header::Machine,
    ident::Encoding,
    medium::MediumError,
    program_header::SegmentType,
    relocation::{ClassRelocation, Rel, Rela},
    table::TableItem,
};
use memory::{
    address::{Address, PhysicalAddress},
    phys::PhysicalMemorySpace,
    translation::TranslationScheme,
};

use crate::{
    arch::{
        paging::ArchScheme,
        relocation::{read_size, relocate},
    },
    executable::{elf::ParsedElf, layout::Layout},
    platform::StubPhysicalMemory,
    trace,
};

/// Applies relocations to the loaded executable.
///
/// # Errors
///
/// Errors are returned as according to the provided [`ApplyRelocationsError`] variant
/// descriptions.
pub fn apply_relocations(
    elf: &ParsedElf,
    layout: &Layout,
    arch_scheme: &mut ArchScheme,
) -> Result<(), ApplyRelocationsError> {
    for header in elf.program_headers {
        if header.segment_type()? != SegmentType::DYNAMIC {
            continue;
        }

        let data = header.segment()?;
        let Some(dynamic_table) = DynamicTable::<_, AnyClass, AnyEndian>::new(
            elf.elf.header().class(),
            elf.elf.header().encoding(),
            data,
            0,
            usize_to_u64(data.len()) / elf.elf.header().class().expected_dynamic_size(),
            elf.elf.header().class().expected_dynamic_size(),
        ) else {
            continue;
        };

        let mut rel_table_offset = None;
        let mut rel_table_size = None;
        let mut rel_entry_size = None;

        let mut rela_table_offset = None;
        let mut rela_table_size = None;
        let mut rela_entry_size = None;

        for dynamic in dynamic_table {
            if dynamic.tag()? == DynamicTag::REL_TABLE {
                trace!("found rel table offset: {}", dynamic.val_ptr()?);
                rel_table_offset.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::REL_SIZE {
                trace!("found rel table size: {}", dynamic.val_ptr()?);
                rel_table_size.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::REL_ENTRY_SIZE {
                trace!("found rel entry size: {}", dynamic.val_ptr()?);
                rel_entry_size.replace(dynamic.val_ptr()?);
            } else if dynamic.tag()? == DynamicTag::RELA_TABLE {
                trace!("found rela table offset: {}", dynamic.val_ptr()?);
                rela_table_offset.replace(dynamic.val_ptr()?);
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

        'rel_table: {
            if rel_table_offset.is_none() && rel_table_size.is_none() && rel_entry_size.is_none() {
                break 'rel_table;
            }

            let rel_table_offset =
                rel_table_offset.ok_or(ApplyRelocationsError::MissingRelTableOffset)?;
            let rel_table_size =
                rel_table_size.ok_or(ApplyRelocationsError::MissingRelTableSize)?;
            let rel_entry_size =
                rel_entry_size.ok_or(ApplyRelocationsError::MissingRelEntrySize)?;

            let expected_rel_size = elf.elf.header().class().expected_rel_size();
            if rel_entry_size < elf.elf.header().class().expected_rel_size() {
                return Err(ApplyRelocationsError::InvalidRelaEntrySize);
            }

            let num_entries = rel_table_size / rel_entry_size;

            let rel_table_virtual_address = layout.slide.strict_add(rel_table_offset);
            for index in 0..num_entries {
                let mut buffer = [0; 128];
                if rel_entry_size > usize_to_u64(buffer.len()) {
                    return Err(ApplyRelocationsError::InvalidRelEntrySize);
                }

                let entry_offset = index.strict_mul(rel_entry_size);
                let rel_entry_virtual_address = rel_table_virtual_address.strict_add(entry_offset);

                read_bytes_from(
                    arch_scheme,
                    rel_entry_virtual_address,
                    &mut buffer[..u64_to_usize_strict(expected_rel_size)],
                )
                .ok_or(ApplyRelocationsError::OutOfBoundsRelocationEntry)?;

                let rel = Rel::new_panicking(
                    elf.elf.header().class(),
                    elf.elf.header().encoding(),
                    0,
                    buffer.as_slice(),
                );

                let relocation_virtual_address = layout.slide.strict_add(rel.offset()?);
                let relocation_type = rel.relocation_type()?;
                let addend = match read_size(elf.machine, relocation_type)? {
                    AddendSize::Bits16 => {
                        read_bytes_from(arch_scheme, relocation_virtual_address, &mut buffer[..2])
                            .ok_or(ApplyRelocationsError::OutOfBoundsRelocation)?;

                        let mut tmp = [0; 2];
                        tmp.copy_from_slice(&buffer[..2]);
                        let val = match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => i16::from_le_bytes(tmp),
                            Encoding::MSB2 => i16::from_be_bytes(tmp),
                            _ => todo!(),
                        };
                        i64::from(val)
                    }
                    AddendSize::Bits32 => {
                        read_bytes_from(arch_scheme, relocation_virtual_address, &mut buffer[..4])
                            .ok_or(ApplyRelocationsError::OutOfBoundsRelocation)?;

                        let mut tmp = [0; 4];
                        tmp.copy_from_slice(&buffer[..4]);
                        let val = match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => i32::from_le_bytes(tmp),
                            Encoding::MSB2 => i32::from_be_bytes(tmp),
                            _ => todo!(),
                        };
                        i64::from(val)
                    }
                    AddendSize::Bits64 => {
                        read_bytes_from(arch_scheme, relocation_virtual_address, &mut buffer[..8])
                            .ok_or(ApplyRelocationsError::OutOfBoundsRelocation)?;

                        let mut tmp = [0; 8];
                        tmp.copy_from_slice(&buffer[..8]);
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => i64::from_le_bytes(tmp),
                            Encoding::MSB2 => i64::from_be_bytes(tmp),
                            _ => todo!(),
                        }
                    }
                };

                let relocation_info = RelocationInfo {
                    relocation_type,
                    addend,
                    slide: layout.slide,
                };

                let mut buffer = [0; 8];
                let byte_count = match relocate(elf.machine, &relocation_info)? {
                    FinalizedRelocation::Bits16(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer[..2].copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer[..2].copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        };
                        2
                    }
                    FinalizedRelocation::Bits32(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer[..4].copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer[..4].copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        }
                        4
                    }
                    FinalizedRelocation::Bits64(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer.copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer.copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        }
                        8
                    }
                };

                write_bytes_into(
                    arch_scheme,
                    relocation_virtual_address,
                    &buffer[..byte_count],
                )
                .ok_or(ApplyRelocationsError::OutOfBoundsRelocation)?;
            }
        }

        'rela_table: {
            if rela_table_offset.is_none() && rela_table_size.is_none() && rela_entry_size.is_none()
            {
                break 'rela_table;
            }

            let rela_table_offset =
                rela_table_offset.ok_or(ApplyRelocationsError::MissingRelaTableOffset)?;
            let rela_table_size =
                rela_table_size.ok_or(ApplyRelocationsError::MissingRelaTableSize)?;
            let rela_entry_size =
                rela_entry_size.ok_or(ApplyRelocationsError::MissingRelaEntrySize)?;

            let expected_rela_size = elf.elf.header().class().expected_rela_size();
            if rela_entry_size < elf.elf.header().class().expected_rela_size() {
                return Err(ApplyRelocationsError::InvalidRelaEntrySize);
            }

            let num_entries = rela_table_size / rela_entry_size;

            let rela_table_virtual_address = layout.slide.strict_add(rela_table_offset);
            for index in 0..num_entries {
                let mut buffer = [0; 128];
                if rela_entry_size > usize_to_u64(buffer.len()) {
                    return Err(ApplyRelocationsError::InvalidRelEntrySize);
                }

                let entry_offset = index.strict_mul(rela_entry_size);
                let rela_entry_virtual_address =
                    rela_table_virtual_address.strict_add(entry_offset);

                read_bytes_from(
                    arch_scheme,
                    rela_entry_virtual_address,
                    &mut buffer[..u64_to_usize_strict(expected_rela_size)],
                )
                .ok_or(ApplyRelocationsError::OutOfBoundsRelocationEntry)?;

                let rela = Rela::new_panicking(
                    elf.elf.header().class(),
                    elf.elf.header().encoding(),
                    0,
                    buffer.as_slice(),
                );

                let relocation_info = RelocationInfo {
                    relocation_type: rela.relocation_type()?,
                    addend: rela.addend()?,
                    slide: layout.slide,
                };

                let mut buffer = [0; 8];
                let byte_count = match relocate(elf.machine, &relocation_info)? {
                    FinalizedRelocation::Bits16(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer[..2].copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer[..2].copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        };
                        2
                    }
                    FinalizedRelocation::Bits32(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer[..4].copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer[..4].copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        }
                        4
                    }
                    FinalizedRelocation::Bits64(value) => {
                        match elf.elf.header().ident()?.encoding()? {
                            Encoding::LSB2 => buffer.copy_from_slice(&value.to_le_bytes()),
                            Encoding::MSB2 => buffer.copy_from_slice(&value.to_be_bytes()),
                            _ => todo!(),
                        }
                        8
                    }
                };

                let relocation_virtual_address = layout.slide.strict_add(rela.offset()?);
                write_bytes_into(
                    arch_scheme,
                    relocation_virtual_address,
                    &buffer[..byte_count],
                )
                .ok_or(ApplyRelocationsError::OutOfBoundsRelocation)?;
            }
        }
    }

    Ok(())
}

/// Reads from the provided [`ArchScheme`] at `virtual_address` into `bytes`.
#[must_use]
fn read_bytes_from(arch_scheme: &ArchScheme, virtual_address: u64, bytes: &mut [u8]) -> Option<()> {
    for (index, byte) in bytes.iter_mut().enumerate() {
        let index = usize_to_u64(index);
        let virtual_address = virtual_address.strict_add(index);
        let (physical_address, _) = arch_scheme.translate_input(Address::new(virtual_address))?;
        // SAFETY:
        //
        // The underlying frames were correctly allocated and are managed by [`ArchScheme`].
        unsafe {
            *byte = StubPhysicalMemory
                .read_u8(PhysicalAddress::new(physical_address.value()))
                .unwrap()
        }
    }

    Some(())
}

/// Writes the provided `bytes` into the provided [`ArchScheme`] at `virtual_address`.
#[must_use]
fn write_bytes_into(
    arch_scheme: &mut ArchScheme,
    virtual_address: u64,
    bytes: &[u8],
) -> Option<()> {
    for (index, byte) in bytes.iter().enumerate() {
        let index = usize_to_u64(index);
        let virtual_address = virtual_address.strict_add(index);
        let (physical_address, _) = arch_scheme.translate_input(Address::new(virtual_address))?;
        // SAFETY:
        //
        // The underlying frames were correctly allocated and are managed by [`ArchScheme`].
        unsafe {
            StubPhysicalMemory
                .write_u8(PhysicalAddress::new(physical_address.value()), *byte)
                .unwrap()
        }
    }

    Some(())
}

/// Various errors that can occur while handling relocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyRelocationsError {
    /// An error occurred while accessing the underlying medium.
    MediumError(MediumError<core::convert::Infallible>),
    /// The relocation table offset for a `REL` table could not be located while other `REL`
    /// descriptor values could be located.
    MissingRelTableOffset,
    /// The relocation table size for a `REL` table could not be located while other `REL`
    /// descriptor values could be located.
    MissingRelTableSize,
    /// The relocation table entry size for a `REL` table could not be located while other `REL`
    /// descriptor values could be located.
    MissingRelEntrySize,
    /// The relocation table entry size for a `REL` table is too small.
    InvalidRelEntrySize,
    /// The relocation table offset for a `RELA` table could not be located while other `RELA`
    /// descriptor values could be located.
    MissingRelaTableOffset,
    /// The relocation table size for a `RELA` table could not be located while other `RELA`
    /// descriptor values could be located.
    MissingRelaTableSize,
    /// The relocation table entry size for a `RELA` table could not be located while other `RELA`
    /// descriptor values could be located.
    MissingRelaEntrySize,
    /// The relocation table entry size for a `RELA` table is too small.
    InvalidRelaEntrySize,
    /// The location of the relocation entry is not within the loaded [`Elf`][e] file.
    ///
    /// [e]: elf::Elf
    OutOfBoundsRelocationEntry,
    /// An error occurred when computing the relocation.
    RelocationError(RelocationError),
    /// The location of the relocation is not within the loaded [`Elf`][e] file.
    ///
    /// [e]: elf::Elf
    OutOfBoundsRelocation,
}

impl From<MediumError<core::convert::Infallible>> for ApplyRelocationsError {
    fn from(error: MediumError<core::convert::Infallible>) -> Self {
        Self::MediumError(error)
    }
}

impl From<RelocationError> for ApplyRelocationsError {
    fn from(error: RelocationError) -> Self {
        Self::RelocationError(error)
    }
}

impl fmt::Display for ApplyRelocationsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediumError(error) => write!(f, "error accessing ELF data: {error}"),
            Self::MissingRelTableOffset => write!(f, "missing DT_REL"),
            Self::MissingRelTableSize => write!(f, "missing DT_RELSZ"),
            Self::MissingRelEntrySize => write!(f, "missing DT_RELENT"),
            Self::InvalidRelEntrySize => write!(f, "value of DT_RELENT is too small"),
            Self::MissingRelaTableOffset => write!(f, "missing DT_RELA"),
            Self::MissingRelaTableSize => write!(f, "missing DT_RELASZ"),
            Self::MissingRelaEntrySize => write!(f, "missing DT_RELAENT"),
            Self::InvalidRelaEntrySize => write!(f, "value of DT_RELAENT is too small"),
            Self::OutOfBoundsRelocationEntry => {
                write!(f, "relocation entry metatdata is out of bounds")
            }
            Self::RelocationError(error) => write!(f, "error computing relocation: {error:?}"),
            Self::OutOfBoundsRelocation => write!(f, "relocation location is out of bounds"),
        }
    }
}

impl error::Error for ApplyRelocationsError {}

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

/// The size of a [`Rel`] addend.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddendSize {
    /// The addend should be produced by reading 16-bits at the address.
    Bits16,
    /// The addend should be produced by reading 32-bits at the address.
    Bits32,
    /// The addend should be produced by reading 64-bits at the address.
    Bits64,
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

/// Various errors that can occur when computing a relocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocationError {
    /// An error occurred while converting values.
    ConversionError,
    /// The relocation type is not supported.
    UnsupportedRelocationType {
        /// The relocation type that is not supported.
        relocation_type: u32,
    },
    /// Relocations are not supported for this [`Machine`].
    UnsupportedMachine {
        /// The [`Machine`] for which relocation are not supported.
        machine: Machine,
    },
}

impl fmt::Display for RelocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConversionError => write!(f, "error converting values"),
            Self::UnsupportedMachine { machine } => {
                write!(f, "unsupported relocation architecture: {machine:?}")
            }
            Self::UnsupportedRelocationType { relocation_type } => {
                write!(f, "unsupported relocation type: {relocation_type}")
            }
        }
    }
}

impl error::Error for RelocationError {}

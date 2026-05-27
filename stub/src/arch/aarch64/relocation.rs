//! Implementation of ELF relocation primitives for `aarch64`.

use elf::header::Machine;

use crate::executable::relocation::{
    AddendSize, FinalizedRelocation, RelocationError, RelocationInfo,
};

/// Handles the ELF relocation types on `aarch64`.
///
/// # Errors
///
/// - [`RelocationError::ConversionError`]: Returned if an error occurs while converting values.
/// - [`RelocationError::UnsupportedRelocationType`]: Returned if the provided relocation is of an
///   unsupported type.
/// - [`RelocationError::UnsupportedMachine`]: Returned if the provided [`Machine`] is of an
///   unsupported type.
pub fn relocate(
    machine: Machine,
    info: &RelocationInfo,
) -> Result<FinalizedRelocation, RelocationError> {
    match machine {
        Machine::AARCH64 => match info.relocation_type {
            1027 => Ok(FinalizedRelocation::Bits64(
                info.slide.wrapping_add_signed(info.addend),
            )),
            _ => Err(RelocationError::UnsupportedRelocationType {
                relocation_type: info.relocation_type,
            }),
        },
        _ => Err(RelocationError::UnsupportedMachine { machine }),
    }
}

/// Returns the size of the addend for an ELF relocation type on `aarch64`.
///
/// # Errors
///
/// - [`RelocationError::ConversionError`]: Never returned by this function.
/// - [`RelocationError::UnsupportedRelocationType`]: Returned if the provided relocation is of an
///   unsupported type.
/// - [`RelocationError::UnsupportedMachine`]: Returned if the provided [`Machine`] is of an
///   unsupported type.
pub fn read_size(machine: Machine, relocation_type: u32) -> Result<AddendSize, RelocationError> {
    match machine {
        Machine::AARCH64 => match relocation_type {
            1027 => Ok(AddendSize::Bits64),
            _ => Err(RelocationError::UnsupportedRelocationType { relocation_type }),
        },
        _ => Err(RelocationError::UnsupportedMachine { machine }),
    }
}

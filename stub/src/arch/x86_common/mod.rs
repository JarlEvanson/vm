//! Structures and functionality that are common to both `x86_32` and `x86_64`.

use elf::header::Machine;

use crate::executable::relocation::{FinalizedRelocation, RelocationError, RelocationInfo};

pub mod paging;

/// Handles the ELF relocation types on `x86_32` and `x86_64`.
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
        Machine::INTEL_386 => match info.relocation_type {
            8 => {
                let slide =
                    u32::try_from(info.slide).map_err(|_| RelocationError::ConversionError)?;
                let addend =
                    i32::try_from(info.addend).map_err(|_| RelocationError::ConversionError)?;
                Ok(FinalizedRelocation::Bits32(
                    slide.wrapping_add_signed(addend),
                ))
            }
            _ => Err(RelocationError::UnsupportedRelocationType {
                relocation_type: info.relocation_type,
            }),
        },

        Machine::X86_64 => match info.relocation_type {
            8 => Ok(FinalizedRelocation::Bits64(
                info.slide.wrapping_add_signed(info.addend),
            )),
            _ => Err(RelocationError::UnsupportedRelocationType {
                relocation_type: info.relocation_type,
            }),
        },
        _ => Err(RelocationError::UnsupportedMachine { machine }),
    }
}

//! Implementations of `x86_64` specific code.

use crate::blob::{FinalizedRelocation, RelocationInfo};

pub mod address_space;
pub mod switch;

/// Handles the ELF relocation types on `x86_64`.
pub fn relocate(info: &RelocationInfo) -> Result<FinalizedRelocation, ()> {
    let relocation = match info.relocation_type {
        8 => FinalizedRelocation::Bits64(info.slide.checked_add_signed(info.addend).ok_or(())?),
        _ => return Err(()),
    };

    Ok(relocation)
}

//! Functionality required to compute the layout of the potential loaded executable.

use core::{error, fmt};

use elf::{header::ElfType, medium::MediumError, program_header::SegmentType};

use crate::{
    arch::{ArchAddressSpace, generic::address_space::AddressSpace},
    executable::elf::ParsedElf,
};

/// The computed layout of the loaded [`Elf`][e] file.
///
/// [e]: elf::Elf
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The offset, in bytes, that should be added to the [`Elf`][e]'s virtual addresses.
    ///
    /// [e]: elf::Elf
    pub slide: u64,
    /// The number of bytes between the lowest address and the highest address.
    pub byte_span: u64,
}

/// Computes the layout of the loaded [`Elf`][e] file.
///
/// # Errors
///
/// - [`ComputeLayoutError::MediumError`]: Returned if an error occurs accessing the underlying
///   [`Medium`][m].
/// - [`ComputeLayoutError::TooLarge`]: Returned if the executable is too large to be contained in
///   an [`Elf`][e] file.
/// - [`ComputeLayoutError::UnsupportedFileType`]: Returned if the [`Elf`][e] file's [`ElfType`] is
///   not supported for layout computation.
///
/// [e]: elf::Elf
/// [m]: elf::medium::Medium
pub fn compute_layout(
    elf: &ParsedElf,
    address_space: &ArchAddressSpace,
) -> Result<Layout, ComputeLayoutError> {
    // Initialize `min_address` and `max_address` with the complete opposite values to ensure any
    // value will be chosen over the initial values.
    let mut min_address = u64::MAX;
    let mut max_address = u64::MIN;
    // Minimum alignment for the loaded [`Elf`] file is the application's page size.
    let mut alignment = address_space.page_size();

    let loadable_headers = elf.program_headers.into_iter().filter(|header| {
        header
            .segment_type()
            .is_ok_and(|segment_type| segment_type == SegmentType::LOAD)
    });
    for header in loadable_headers {
        let virtual_address = header.virtual_address()?;
        let memory_size = header.memory_size()?;
        let segment_alignment = header.alignment()?;

        min_address = min_address.min(virtual_address);
        max_address = max_address.max(virtual_address.strict_add(memory_size));
        alignment = alignment.max(segment_alignment);
    }

    let aligned_min_address = min_address - min_address % alignment;
    let aligned_max_address = max_address
        .checked_next_multiple_of(alignment)
        .ok_or(ComputeLayoutError::TooLarge)?;
    let byte_span = aligned_max_address - aligned_min_address;

    let slide = match elf.elf_type {
        ElfType::EXECUTABLE => 0,
        ElfType::SHARED => {
            let base = address_space.max_virtual_address() - byte_span;
            base - base % alignment
        }
        elf_type => return Err(ComputeLayoutError::UnsupportedFileType(elf_type)),
    };

    Ok(Layout { slide, byte_span })
}

/// Various errors that can occur when computing the layout of the [`Elf`][e] file.
///
/// [e]: elf::Elf
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputeLayoutError {
    /// An error occurred when accessing the [`Medium`][m].
    ///
    /// [m]: elf::medium::Medium
    MediumError(MediumError<core::convert::Infallible>),
    /// The executable to be loaded is too large.
    TooLarge,
    /// The [`Elf`][e] file's [`ElfType`] was not supported.
    ///
    /// [e]: elf::Elf
    UnsupportedFileType(ElfType),
}

impl From<MediumError<core::convert::Infallible>> for ComputeLayoutError {
    fn from(error: MediumError<core::convert::Infallible>) -> Self {
        Self::MediumError(error)
    }
}

impl fmt::Display for ComputeLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediumError(error) => write!(f, "error accessing segment data: {error}"),
            Self::TooLarge => {
                f.pad("the provided ELF executable is too large for the address space")
            }
            Self::UnsupportedFileType(file_type) => {
                write!(f, "executable file type not supported: {file_type:?}")
            }
        }
    }
}

impl error::Error for ComputeLayoutError {}

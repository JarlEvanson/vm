//! Functionality required to compute the layout of the potential loaded executable.

use core::{error, fmt};

use elf::{header::ElfType, medium::MediumError, program_header::SegmentType};
use memory::{
    address::{Address, AddressRange},
    translation::TranslationScheme,
};

use crate::{arch::paging::ArchScheme, executable::elf::ParsedElf};

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
    arch_scheme: &ArchScheme,
) -> Result<Layout, ComputeLayoutError> {
    // Initialize `min_address` and `max_address` with the complete opposite values to ensure any
    // value will be chosen over the initial values.
    let mut min_address = u64::MAX;
    let mut max_address = u64::MIN;
    // Minimum alignment for the loaded [`Elf`] file is the application's page size.
    let mut alignment = arch_scheme.chunk_size();

    let loadable_headers = elf.program_headers.into_iter().filter(|header| {
        header
            .segment_type()
            .is_ok_and(|segment_type| segment_type == SegmentType::LOAD)
    });

    let mut prev_end_address = 0u64;
    for header in loadable_headers {
        let virtual_address = header.virtual_address()?;
        let memory_size = header.memory_size()?;
        let segment_alignment = header.alignment()?;
        let end_address = virtual_address
            .checked_add(memory_size)
            .ok_or(ComputeLayoutError::TooLarge)?;

        min_address = min_address.min(virtual_address);
        max_address = max_address.max(end_address);
        alignment = alignment.max(segment_alignment);

        let aligned_prev_end_address = prev_end_address
            .checked_next_multiple_of(arch_scheme.chunk_size())
            .ok_or(ComputeLayoutError::TooLarge)?;
        let aligned_start_address =
            (virtual_address / arch_scheme.chunk_size()) * arch_scheme.chunk_size();

        if aligned_prev_end_address > aligned_start_address {
            return Err(ComputeLayoutError::OverlappingSegments);
        }

        if prev_end_address > end_address {
            return Err(ComputeLayoutError::NonAscending);
        }
        prev_end_address = end_address;
    }

    let min_address = Address::new(min_address);
    let max_address = Address::new(max_address);

    let aligned_min_address = min_address.align_down(alignment);
    let aligned_max_address = max_address.strict_align_up(alignment);
    let aligned_address_range =
        AddressRange::from_exclusive(aligned_min_address, aligned_max_address);

    let slide = match elf.elf_type {
        ElfType::EXECUTABLE => 0,
        ElfType::SHARED => {
            let mut base_storage = None;
            for (start, end) in arch_scheme.input_descriptor().valid_ranges() {
                if start > end {
                    // Skip empty ranges.
                    continue;
                }

                if end - start < aligned_address_range.count() {
                    continue;
                }

                let base = Address::new(end - aligned_address_range.count());
                base_storage = Some(base.align_down(alignment));
            }

            let Some(base) = base_storage else {
                return Err(ComputeLayoutError::TooLarge);
            };

            base.value()
        }
        elf_type => return Err(ComputeLayoutError::UnsupportedFileType(elf_type)),
    };

    Ok(Layout {
        slide,
        byte_span: aligned_address_range.count(),
    })
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
    /// The [`Elf`][e] file's loadable program headers are overlapping when aligned to the page
    /// size.
    ///
    /// [e]: elf::Elf
    OverlappingSegments,
    /// The [`Elf`][e] file's loadable program headers are not ascending.
    ///
    /// [e]: elf::Elf
    NonAscending,
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
            Self::OverlappingSegments => f.pad(
                "the provided ELF executable's loadable segments \
                overlap when aligned to page boundaries",
            ),
            Self::NonAscending => {
                f.pad("the provided ELF executable's loadable program headers are not ascending")
            }
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

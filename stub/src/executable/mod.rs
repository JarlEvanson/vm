//! Various implementations and functionality related to the embedded executable.

use core::{error, fmt};

use crate::{
    arch::paging::{ArchScheme, ArchSchemeError},
    executable::{
        elf::ParseElfError, layout::ComputeLayoutError, mapping::MapSegmentsError,
        relocation::ApplyRelocationsError,
    },
    platform::FrameAllocation,
};

pub mod blob;
pub mod elf;
pub mod layout;
pub mod mapping;
pub mod relocation;

/// Loads the emebdded executable.
#[expect(clippy::missing_errors_doc)]
pub fn load() -> Result<(ArchScheme, u64, FrameAllocation, u64), LoadExecutableError> {
    let blob = blob::extract_blob();
    let parsed = elf::parse(blob)?;

    let mut scheme = ArchScheme::max_supported(parsed.machine)?;

    let layout = layout::compute_layout(&parsed, &scheme)?;
    let image = mapping::map_segments(&parsed, &layout, &mut scheme)?;
    relocation::apply_relocations(&parsed, &layout, &mut scheme)?;

    Ok((
        scheme,
        parsed.entry_point.strict_add(layout.slide),
        image,
        layout.slide,
    ))
}

/// Various errors that can occur while loading the embedded executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadExecutableError {
    /// An error occurred while parsing the ELF file.
    ParseElfError(ParseElfError<core::convert::Infallible>),
    /// An error occurred while creating the new [`ArchScheme`].
    ArchSchemeError(ArchSchemeError),
    /// An error occurred while computing the layout of the embedded executable when loaded.
    ComputeLayoutError(ComputeLayoutError),
    /// An error occurred while mapping the embedded executable into the [`ArchScheme`].
    MapSegmentsError(MapSegmentsError),
    /// An error occurred while relocating the loaded executable.
    ApplyRelocationsError(ApplyRelocationsError),
}

impl From<ParseElfError<core::convert::Infallible>> for LoadExecutableError {
    fn from(error: ParseElfError<core::convert::Infallible>) -> Self {
        Self::ParseElfError(error)
    }
}

impl From<ArchSchemeError> for LoadExecutableError {
    fn from(error: ArchSchemeError) -> Self {
        Self::ArchSchemeError(error)
    }
}

impl From<ComputeLayoutError> for LoadExecutableError {
    fn from(error: ComputeLayoutError) -> Self {
        Self::ComputeLayoutError(error)
    }
}

impl From<MapSegmentsError> for LoadExecutableError {
    fn from(error: MapSegmentsError) -> Self {
        Self::MapSegmentsError(error)
    }
}

impl From<ApplyRelocationsError> for LoadExecutableError {
    fn from(error: ApplyRelocationsError) -> Self {
        Self::ApplyRelocationsError(error)
    }
}

impl fmt::Display for LoadExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseElfError(error) => write!(f, "error parsing embedded ELF file: {error}"),
            Self::ArchSchemeError(error) => {
                write!(f, "error creating new arch translation scheme: {error}")
            }
            Self::ComputeLayoutError(error) => write!(f, "error computing ELF layout: {error}"),
            Self::MapSegmentsError(error) => {
                write!(f, "error mapping ELF segments into memory: {error}")
            }
            Self::ApplyRelocationsError(error) => {
                write!(f, "error applying ELF relocations: {error}")
            }
        }
    }
}

impl error::Error for LoadExecutableError {}

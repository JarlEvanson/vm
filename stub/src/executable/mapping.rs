//! Functionality related to mapping and loading the [`Elf`][e] file.
//!
//! [e]: elf::Elf

use core::{error, fmt};

use conversion::usize_to_u64;
use elf::{
    medium::MediumError,
    program_header::{SegmentFlags, SegmentType},
};

use crate::{
    arch::generic::memory::paging::{
        ExternalFrame, ExternalFrameRange, ExternalPage, ExternalPageRange,
        ExternalPhysicalAddress, ExternalVirtualAddress, ExternalVirtualAddressRange,
        TranslationScheme,
    },
    executable::{elf::ParsedElf, layout::Layout},
    platform::{
        AllocationPolicy, FrameAllocation, MapError, MappingType, OutOfMemory, Permissions,
        PhysicalAddress, allocate_frames_aligned, frame_size, write_bytes_at, write_u8_at,
    },
};

/// Maps the provided [`Elf`][e] file into the provided [`TranslationScheme`].
///
/// # Errors
///
/// - [`MapSegmentsError::OutOfMemory`]: Returned if the allocation of underlying physical memory
///   failed.
/// - [`MapSegmentsError::MediumError`]: Returned if an error occurs while accessing segment data.
/// - [`MapError`]: Returned if an error occurs while mapping a sgement.
///
/// [e]: elf::Elf
#[expect(clippy::missing_panics_doc)]
pub fn map_segments<T: TranslationScheme>(
    elf: &ParsedElf,
    layout: &Layout,
    scheme: &mut T,
) -> Result<FrameAllocation, MapSegmentsError> {
    let frame_count = layout.byte_span.div_ceil(frame_size());
    let frame_allocation = allocate_frames_aligned(
        frame_count,
        scheme.chunk_size(),
        AllocationPolicy::InclusiveMax(scheme.output_descriptor().valid_ranges()[0].1),
    )?;

    let mut frame_index = 0;
    for (index, header) in elf.program_headers.into_iter().enumerate() {
        match header.segment_type()? {
            SegmentType::LOAD => {
                if header.memory_size()? == 0 {
                    crate::warn!("zero-sized loadable memory segment");
                    continue;
                }

                let start_address =
                    ExternalVirtualAddress::new(layout.slide + header.virtual_address()?);
                let end_address = start_address.strict_add(header.memory_size()?.saturating_sub(1));
                let address_range = ExternalVirtualAddressRange::new(start_address, end_address);

                let start_page =
                    ExternalPage::containing_address(address_range.start(), scheme.chunk_size());
                let end_page = ExternalPage::containing_address(
                    address_range.end_inclusive(),
                    scheme.chunk_size(),
                );
                let page_range = ExternalPageRange::new(start_page, end_page);

                // Total number of frames required for page mapping.
                let required_frames = page_range
                    .byte_count(scheme.chunk_size())
                    .div_ceil(frame_size());

                // Allocate some frames from the previously allocated [`FrameAllocation`].
                let segment_physical_address =
                    frame_allocation.range().start().start_address().value()
                        + frame_index * frame_size();
                frame_index += required_frames;

                let offset = start_address.value()
                    - page_range
                        .start()
                        .start_address(scheme.chunk_size())
                        .value();

                let writable = header.flags()?.0 & SegmentFlags::WRITE.0 == SegmentFlags::WRITE.0;
                let executable =
                    header.flags()?.0 & SegmentFlags::EXECUTE.0 == SegmentFlags::EXECUTE.0;
                let permissions = match (writable, executable) {
                    (true, true) => Permissions::ReadWriteExecute,
                    (true, false) => Permissions::ReadWrite,
                    (false, true) => Permissions::ReadExecute,
                    (false, false) => Permissions::Read,
                };

                let physical_range = ExternalFrameRange::new(
                    ExternalFrame::containing_address(
                        ExternalPhysicalAddress::new(segment_physical_address),
                        scheme.chunk_size(),
                    ),
                    page_range.count(),
                );

                scheme.map_at(page_range, physical_range, permissions, MappingType::Normal)?;
                crate::debug!(
                    "Segment {index} loaded at {start_address:x?} ({:#x})",
                    segment_physical_address + offset
                );

                let file_bytes = header.segment().unwrap_or(&[]);

                write_bytes_at(
                    PhysicalAddress::new(segment_physical_address + offset),
                    file_bytes,
                );

                let zero_base = segment_physical_address + offset + usize_to_u64(file_bytes.len());
                for i in 0..(header.memory_size()? - usize_to_u64(file_bytes.len())) {
                    if !write_u8_at(PhysicalAddress::new(zero_base + i), 0) {
                        panic!("failed to write to physical memory");
                    }
                }
            }
            SegmentType::NULL
            | SegmentType::DYNAMIC
            | SegmentType::INTERP
            | SegmentType::NOTE
            | SegmentType::TLS
            | SegmentType::PHDR => {}
            segment_type => crate::warn!("unknown segment type: {segment_type:?}"),
        }
    }

    Ok(frame_allocation)
}

/// Various errors that can occur when mapping an executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapSegmentsError {
    /// An error allocating memory for the executable.
    OutOfMemory(OutOfMemory),
    /// An error occured accessing the underlying medium.
    MediumError(MediumError<core::convert::Infallible>),
    /// An error occurred mapping a segment into the provided [`TranslationScheme`].
    MapError(MapError),
}

impl From<OutOfMemory> for MapSegmentsError {
    fn from(error: OutOfMemory) -> Self {
        Self::OutOfMemory(error)
    }
}

impl From<MediumError<core::convert::Infallible>> for MapSegmentsError {
    fn from(error: MediumError<core::convert::Infallible>) -> Self {
        Self::MediumError(error)
    }
}

impl From<MapError> for MapSegmentsError {
    fn from(error: MapError) -> Self {
        Self::MapError(error)
    }
}

impl fmt::Display for MapSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory(error) => {
                write!(f, "error allocating memory for the executable: {error}")
            }
            Self::MediumError(error) => write!(f, "error accessing ELF segment bytes: {error}"),
            Self::MapError(error) => {
                write!(f, "error mapping the provided segment into memory: {error}")
            }
        }
    }
}

impl error::Error for MapSegmentsError {}

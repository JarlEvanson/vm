//! Functionality related to mapping and loading the [`Elf`][e] file.
//!
//! [e]: elf::Elf

use core::{error, fmt};

use conversion::usize_to_u64;
use elf::{
    medium::MediumError,
    program_header::{SegmentFlags, SegmentType},
};
use memory::{
    address::{Address, AddressChunk, AddressChunkRange, AddressRange, PhysicalAddress},
    phys::PhysicalMemorySpace,
    translation::{MapError, MapFlags, TranslationScheme},
};

use crate::{
    arch::paging::ArchScheme,
    executable::{elf::ParsedElf, layout::Layout},
    platform::{
        AllocationPolicy, FrameAllocation, OutOfMemory, StubPhysicalMemory,
        allocate_frames_aligned, frame_size, write_bytes_at,
    },
};

/// Maps the provided [`Elf`][e] file into the provided [`ArchScheme`].
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
pub fn map_segments(
    elf: &ParsedElf,
    layout: &Layout,
    arch_scheme: &mut ArchScheme,
) -> Result<FrameAllocation, MapSegmentsError> {
    let frame_count = layout.byte_span.div_ceil(arch_scheme.chunk_size());
    let frame_allocation = allocate_frames_aligned(
        frame_count,
        arch_scheme.chunk_size(),
        AllocationPolicy::Below(arch_scheme.output_descriptor().valid_ranges()[0].1),
    )?;

    let mut frame_index = 0;
    for (index, header) in elf.program_headers.into_iter().enumerate() {
        match header.segment_type()? {
            SegmentType::LOAD => {
                let start_address = Address::new(layout.slide + header.virtual_address()?);
                let address_range = AddressRange::new(start_address, header.memory_size()?);

                let start_chunk = AddressChunk::containing_address(
                    address_range.start(),
                    arch_scheme.chunk_size(),
                );
                let end_chunk = AddressChunk::containing_address(
                    address_range.end_inclusive(),
                    arch_scheme.chunk_size(),
                );
                let chunk_range = if address_range.is_empty() {
                    AddressChunkRange::empty()
                } else {
                    AddressChunkRange::from_inclusive(start_chunk, end_chunk)
                };

                // Total number of frames required for page mapping.
                let required_frames = chunk_range
                    .byte_count(arch_scheme.chunk_size())
                    .div_ceil(frame_size());

                // Allocate some frames from the previously allocated [`FrameAllocation`].
                let segment_physical_address = frame_allocation
                    .range()
                    .start()
                    .start_address(frame_size())
                    .value()
                    + frame_index * arch_scheme.chunk_size();
                frame_index += required_frames;

                let offset = start_address.value()
                    - chunk_range
                        .start()
                        .start_address(arch_scheme.chunk_size())
                        .value();
                let mut protection = MapFlags::READ;
                if header.flags()?.0 & SegmentFlags::WRITE.0 == SegmentFlags::WRITE.0 {
                    protection |= MapFlags::WRITE;
                }
                if header.flags()?.0 & SegmentFlags::EXECUTE.0 == SegmentFlags::EXECUTE.0 {
                    protection |= MapFlags::EXEC;
                }

                let physical_range = AddressChunkRange::new(
                    AddressChunk::containing_address(
                        Address::new(segment_physical_address),
                        arch_scheme.chunk_size(),
                    ),
                    chunk_range.count(),
                );

                // SAFETY:
                //
                // The provided range is unoccupied and valid.
                unsafe {
                    arch_scheme.map(chunk_range, physical_range, protection)?;
                }

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
                    // SAFETY:
                    //
                    // The memory written to by this operation is within the bounds of
                    // `frame_allocation` and thus is safe to write to.
                    unsafe {
                        StubPhysicalMemory
                            .write_u8(PhysicalAddress::new(zero_base + i), 0)
                            .expect("failed to write to physical memory")
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
    /// An error occurred mapping a segment into the provided [`ArchScheme`].
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

//! Functionality related to mapping and loading the [`Elf`][e] file.
//!
//! [e]: elf::Elf

use core::{error, fmt};

use elf::{
    medium::MediumError,
    program_header::{SegmentFlags, SegmentType},
};

use crate::{
    arch::{
        ArchAddressSpace,
        generic::address_space::{AddressSpace, MapError, ProtectionFlags},
    },
    debug,
    executable::{elf::ParsedElf, layout::Layout},
    platform::{
        AllocationPolicy, FrameAllocation, OutOfMemory, PhysicalAddress, allocate_frames_aligned,
        frame_size, write_bytes_at, write_u8_at,
    },
    util::usize_to_u64,
    warn,
};

/// Maps the provided [`Elf`][e] file into the provided [`AddressSpace`].
///
/// # Errors
///
/// - [`MapSegmentsError::OutOfMemory`]: Returned if the allocation of underlying physical memory
///   failed.
/// - [`MapSegmentsError::MediumError`]: Returned if an error occurs while accessing segment data.
/// - [`MapError`]: Returned if an error occurs while mapping a sgement.
///
/// [e]: elf::Elf
pub fn map_segments(
    elf: &ParsedElf,
    layout: &Layout,
    address_space: &mut ArchAddressSpace,
) -> Result<FrameAllocation, MapSegmentsError> {
    let frame_count = layout.byte_span.div_ceil(address_space.page_size());
    let frame_allocation = allocate_frames_aligned(
        frame_count,
        address_space.page_size(),
        AllocationPolicy::Below(address_space.max_physical_address()),
    )?;

    let mut frame_index = 0;
    for (index, header) in elf.program_headers.into_iter().enumerate() {
        match header.segment_type()? {
            SegmentType::LOAD => {
                let start_address = layout.slide + header.virtual_address()?;
                let end_address = start_address + header.memory_size()?;

                // Page aligned addresses and total bytes on mapped pages.
                let aligned_start_address =
                    start_address - start_address % address_space.page_size();
                let aligned_end_address = end_address.next_multiple_of(address_space.page_size());
                let page_bytes = aligned_end_address - aligned_start_address;

                // Total number of frames required for page mapping.
                let required_frames = page_bytes.div_ceil(frame_size());

                // Allocate some frames from the prior [`FrameAllocation`].
                let segment_physical_address =
                    frame_allocation.range().start().start_address().value()
                        + frame_index * address_space.page_size();
                frame_index += required_frames;

                let offset = start_address - aligned_start_address;
                let mut protection = ProtectionFlags::READ;
                if header.flags()?.0 & SegmentFlags::WRITE.0 == SegmentFlags::WRITE.0 {
                    protection |= ProtectionFlags::WRITE;
                }
                if header.flags()?.0 & SegmentFlags::EXECUTE.0 == SegmentFlags::EXECUTE.0 {
                    protection |= ProtectionFlags::EXEC;
                }

                debug!(
                    "Segment {index} loaded at {start_address:#x} ({:#x})",
                    segment_physical_address + offset
                );
                address_space.map(
                    aligned_start_address,
                    segment_physical_address,
                    page_bytes / address_space.page_size(),
                    protection,
                )?;

                let file_bytes = header.segment().unwrap_or(&[]);

                write_bytes_at(
                    PhysicalAddress::new(segment_physical_address + offset),
                    file_bytes,
                );

                let zero_base = segment_physical_address + offset + usize_to_u64(file_bytes.len());
                for i in 0..(header.memory_size()? - usize_to_u64(file_bytes.len())) {
                    write_u8_at(PhysicalAddress::new(zero_base + i), 0);
                }
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

    Ok(frame_allocation)
}

/// Various errors that can occur when mapping an executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapSegmentsError {
    /// An error allocating memory for the executable.
    OutOfMemory(OutOfMemory),
    /// An error occured accessing the underlying medium.
    MediumError(MediumError<core::convert::Infallible>),
    /// An error occurred mapping a segment into the provided [`AddressSpace`].
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

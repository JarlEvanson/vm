//! Definitions and implementations of physical memory management APIs for `revm`.

use core::{error, fmt};

use crate::{
    memory::{
        page_frame_size,
        phys::structs::{Frame, FrameRange, PhysicalAddress},
    },
    util::usize_to_u64,
};

pub mod structs;

/// Allocates a region of `count` frames aligned to `alignment` bytes.
///
/// # Errors
///
/// [`FrameAllocationError`] is returned if an error occurs while allocating [`Frame`]s.
pub fn allocate_frames(count: u64, alignment: u64) -> Result<FrameRange, FrameAllocationError> {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let total_bytes = count.strict_mul(usize_to_u64(page_frame_size()));
        let stub_frame_count = total_bytes.div_ceil(generic_table.page_frame_size);

        let mut physical_address = 0;

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.allocate_frames)(
                stub_frame_count,
                alignment.max(alignment),
                stub_api::AllocationFlags::ANY,
                &mut physical_address,
            )
        };
        if result != stub_api::Status::SUCCESS {
            return Err(FrameAllocationError);
        }

        let start = Frame::containing_address(PhysicalAddress::new(physical_address));
        Ok(FrameRange::new(start, count))
    } else {
        todo!("implement post-takeover frame allocation")
    }
}

/// Deallocates the [`FrameRange`].
///
/// # Safety
///
/// The physical memory region described by [`FrameRange`] must not be in use.
pub unsafe fn deallocate_frames(frame_range: FrameRange) {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let stub_frame_count = frame_range
            .byte_count()
            .div_ceil(generic_table.page_frame_size);

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.deallocate_frames)(
                frame_range.start().start_address().value(),
                stub_frame_count,
            )
        };
        if result != stub_api::Status::SUCCESS {
            crate::warn!("error deallocating frames: {result:?}");
        }
    } else {
        todo!("implement post-takeover frame allocation")
    }
}

/// Various errors that can occur while allocating [`Frame`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameAllocationError;

impl fmt::Display for FrameAllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error allocating frames")
    }
}

impl error::Error for FrameAllocationError {}

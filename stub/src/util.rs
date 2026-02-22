//! Various utility functions.

use core::mem;

use memory::address::{Frame, FrameRange, PhysicalAddressRange};

use crate::platform::{AllocationPolicy, allocate_frames_aligned, deallocate_frames, frame_size};

unsafe extern "C" {
    #[link_name = "_image_start"]
    static IMAGE_START: u8;
}

/// Returns the virtual address of the start of the image.
pub fn image_start() -> usize {
    (&raw const IMAGE_START).addr()
}

/// Allocates `count` bytes with an alignment of `alignment`.
pub fn alloc_physical(count: u64, alignment: u64) -> Option<PhysicalAddressRange> {
    allocate_frames_aligned(
        count.div_ceil(frame_size()),
        alignment,
        AllocationPolicy::Any,
    )
    .ok()
    .map(|allocation| {
        let range = PhysicalAddressRange::new(
            allocation.range().start().start_address(frame_size()),
            allocation.range().byte_count(frame_size()),
        );
        mem::forget(allocation);
        range
    })
}

/// Deallocates `count` bytes with an alignment of `alignment`.
pub fn dealloc_physical(range: PhysicalAddressRange) {
    // SAFETY:
    //
    // The invariants of this function ensure that the physical memory is not in use.
    unsafe {
        deallocate_frames(FrameRange::from_inclusive(
            Frame::containing_address(range.start(), frame_size()),
            Frame::containing_address(range.end_inclusive(), frame_size()),
        ))
    }
}

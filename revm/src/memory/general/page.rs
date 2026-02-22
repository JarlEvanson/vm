//! A page-sized allocator.

use core::ptr::NonNull;

use crate::{
    memory::{
        page_frame_size,
        phys::{allocate_frames, deallocate_frames},
        virt::{Permissions, map},
    },
    util::usize_to_u64,
};

/// Allocates a region of memory of `size` bytes directly utilizing pages.
pub fn allocate(size: usize, alignment: usize) -> Option<NonNull<u8>> {
    let page_size = page_frame_size();
    let page_count = size.div_ceil(page_size);

    // We align to the requested alignment, or at least a page boundary.
    let alignment = alignment.max(page_size);
    if alignment > page_size {
        todo!("{page_size} alignment is not supported");
    }

    let frame_range = allocate_frames(usize_to_u64(page_count), usize_to_u64(alignment)).ok()?;

    let Ok(page_range) = map(frame_range, Permissions::ReadWrite) else {
        // SAFETY:
        //
        // `frame_range` has not escaped this function and is not and will not be in use.
        unsafe { deallocate_frames(frame_range) }
        return None;
    };

    NonNull::new(page_range.start().start_address().value() as *mut u8)
}

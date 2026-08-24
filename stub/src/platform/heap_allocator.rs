//! Implementation of a generic heap allocator.

use core::{alloc::Layout, mem, ptr::NonNull};

use conversion::usize_to_u64;

use crate::platform::{
    AllocationPolicy, Frame, FrameRange, Page, PageRange, Permissions, VirtualAddress,
    allocate_frames_aligned, deallocate_frames, frame_size, map, page_size, translate_virt, unmap,
};

/// Implementation of [`crate::platform::allocate()`] using [`allocate_frames_aligned()`] and
/// [`map()`].
pub fn allocate(layout: Layout) -> Option<NonNull<u8>> {
    assert!(layout.align() <= page_size());

    let count = usize_to_u64(layout.size()).div_ceil(frame_size());
    let alignment = usize_to_u64(page_size()).max(frame_size());
    let frames = allocate_frames_aligned(count, alignment, AllocationPolicy::Any).ok()?;

    let mapping = map(frames.range(), Permissions::ReadWrite).ok()?;

    let ptr = core::ptr::with_exposed_provenance_mut(mapping.range().start_address().value());

    mem::forget(frames);
    mem::forget(mapping);
    NonNull::new(ptr)
}

/// Implementation of [`crate::platform::deallocate()`] using [`deallocate_frames()`] and
/// [`unmap()`].
pub unsafe fn deallocate(ptr: NonNull<u8>, layout: Layout) {
    let Some((_, _, physical_address)) = translate_virt(VirtualAddress::new(ptr.addr().get()))
    else {
        crate::warn!("translation of {ptr:p} failed during deallocation");
        return;
    };

    let start_page = Page::containing_address(VirtualAddress::new(ptr.addr().get()));
    let page_count = layout.size().div_ceil(page_size());
    let page_range = PageRange::new(
        start_page,
        start_page.strict_add(page_count.saturating_sub(1)),
    );

    // SAFETY:
    //
    // The invariants of this function ensure that the virtual memory region will not be used
    // again.
    unsafe { unmap(page_range) }

    let start_frame = Frame::containing_address(physical_address);
    let frame_count = usize_to_u64(layout.size()).div_ceil(frame_size());
    let frame_range = FrameRange::new(start_frame, frame_count);

    // SAFETY:
    //
    // The invariants of this function ensure that the underlying physical memory region is not in
    // use.
    unsafe { deallocate_frames(frame_range) }
}

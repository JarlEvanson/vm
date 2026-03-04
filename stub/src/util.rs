//! Various utility functions.

use core::{mem, ptr};

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

/// Wrapper around running a function on data when dropped.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropWrapper<T, F: FnMut(&mut T)> {
    /// The value to run the function.
    pub val: T,
    /// The function to run.
    pub drop_func: F,
}

impl<T, F: FnMut(&mut T)> DropWrapper<T, F> {
    /// Returns the `val` inside of [`DropWrapper`] without running the provided
    /// [`DropWrapper::drop_func`].
    pub fn into_inner(self) -> T {
        // SAFETY:
        //
        // - `self.val` is valid for reads, initialized, and properly aligned.
        let val = unsafe { ptr::read(&self.val) };
        mem::forget(self);
        val
    }
}

impl<T, F: FnMut(&mut T)> Drop for DropWrapper<T, F> {
    fn drop(&mut self) {
        (self.drop_func)(&mut self.val)
    }
}

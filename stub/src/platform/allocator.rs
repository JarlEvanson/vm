//! Implementation of a generic allocator.

use core::ptr::NonNull;

use sync::ControlledModificationCell;

use crate::{
    platform::{
        AllocationPolicy, Frame, FrameRange, PhysicalAddress, VirtualAddress, frame_size,
        page_size, translate_virtual,
    },
    util::usize_to_u64,
    warn,
};

/// Function to map a physical memory region into virtual memory.
#[expect(clippy::type_complexity)]
pub(in crate::platform) static MAP: ControlledModificationCell<
    Option<fn(PhysicalAddress, u64) -> Option<NonNull<u8>>>,
> = ControlledModificationCell::new(None);
/// Function to unmap a virtual memory region.
#[expect(clippy::type_complexity)]
pub(in crate::platform) static UNMAP: ControlledModificationCell<
    Option<unsafe fn(NonNull<u8>, u64)>,
> = ControlledModificationCell::new(None);

/// Implementation of [`crate::platform::allocate()`] using
/// [`crate::platform::allocate_frames_aligned()`] and a memory mapping function.
pub fn allocate(size: usize, alignment: usize) -> Option<NonNull<u8>> {
    assert!(alignment <= page_size());

    let count = usize_to_u64(size).div_ceil(frame_size());
    let alignment = usize_to_u64(page_size()).max(frame_size());
    let frames = crate::platform::generic::platform()
        .allocate_frames_aligned(count, alignment, AllocationPolicy::Any)
        .ok()?;

    let map = MAP.get().expect("map function not provided");
    map(frames.start().start_address(), usize_to_u64(size))
}

/// Implementation of [`crate::platform::deallocate()`] using
/// [`crate::platform::deallocate_frames`] and a memory unmapping function.
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize, _: usize) {
    let Some(physical_address) = translate_virtual(VirtualAddress::new(ptr.addr().get())) else {
        warn!("translation of {ptr:p} failed during deallocation");
        return;
    };
    let count = usize_to_u64(size).div_ceil(frame_size());

    let unmap = UNMAP.get().expect("unmap function not provided");

    // SAFETY:
    //
    // The invariants of this function ensure that the virtual memory region will not be used
    // again.
    unsafe { unmap(ptr, usize_to_u64(size)) }

    // SAFETY:
    //
    // The invariants of this function ensure that the underlying physical memory region is not in
    // use.
    unsafe {
        crate::platform::generic::platform().deallocate_frames(FrameRange::new(
            Frame::containing_address(PhysicalAddress::new(physical_address)),
            count,
        ))
    }
}

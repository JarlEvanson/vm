//! Implementation of a generic allocator.

use core::ptr::NonNull;

use sync::ControlledModificationCell;

use crate::{
    platform::{AllocationPolicy, frame_size, page_size, translate_virtual},
    util::usize_to_u64,
    warn,
};

pub(in crate::platform) static MAP: ControlledModificationCell<
    Option<fn(u64, u64) -> Option<NonNull<u8>>>,
> = ControlledModificationCell::new(None);
pub(in crate::platform) static UNMAP: ControlledModificationCell<
    Option<unsafe fn(NonNull<u8>, u64)>,
> = ControlledModificationCell::new(None);

pub fn allocate(size: usize, alignment: usize) -> Option<NonNull<u8>> {
    assert!(alignment <= page_size());

    let count = usize_to_u64(size).div_ceil(frame_size());
    let alignment = usize_to_u64(page_size()).max(frame_size());
    let frames = crate::platform::generic::platform()
        .allocate_frames_aligned(count, alignment, AllocationPolicy::Any)
        .ok()?;

    let map = MAP.get().expect("map function not provided");
    let Some(ptr) = map(frames, usize_to_u64(size)) else {
        return None;
    };

    Some(ptr)
}

pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize, _: usize) {
    let Some(physical_address) = translate_virtual(ptr.addr().get()) else {
        warn!("translation of {ptr:p} failed during deallocation");
        return;
    };
    let count = usize_to_u64(size).div_ceil(frame_size());

    let unmap = UNMAP.get().expect("unmap function not provided");
    unsafe { unmap(ptr, usize_to_u64(size)) }

    unsafe { crate::platform::generic::platform().deallocate_frames(physical_address, count) }
}

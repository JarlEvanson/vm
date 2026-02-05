//! Definitions and implementations of physical memory management APIs for `revm`.

use core::mem;

use crate::memory::{OutOfMemory, page_frame_size};

/// Allocates a region of `count` frames in accordance. The
/// starting physical address of the region will be a multiple of `alignment`.
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames.
pub fn allocate_frames(count: u64, alignment: u64) -> Result<FrameRegion, OutOfMemory> {
    todo!()
}

/// Deallocates a region of `count` frames with the starting `physical_address`.
///
/// # Safety
///
/// These frames must have been allocated by a call to [`allocate_frames()`] and must not be
/// referenced after the start of this function.
pub unsafe fn deallocate_frames(physical_address: u64, count: u64) {
    todo!()
}

/// Wrapper around a region of frames allocated with [`allocate_frames()`].
///
/// This structure automatically frees the region of frames when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameRegion {
    /// The physical address of the start of the allocated frame region.
    physical_address: u64,
    /// The number of frames in the allocated frame region.
    count: u64,
}

impl FrameRegion {
    /// Returns the physical address of the start of the allocated frame region.
    pub const fn physical_address(&self) -> u64 {
        self.physical_address
    }

    /// Returns the number of frames in the allocated frame region.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Returns the total number of bytes controlled by the allocated frame region.
    pub fn total_size(&self) -> u64 {
        self.count().strict_mul(page_frame_size())
    }

    /// Returns `true` if `address` is contained within the [`FrameRegion`].
    pub fn contains(&self, address: u64) -> bool {
        self.physical_address <= address && address < self.physical_address + self.total_size()
    }

    /// Splits the [`FrameRegion`] into two [`FrameRegion`]s. The resulting [`FrameRegion`]s must
    /// not be empty.
    pub fn split(self, split_at: u64) -> Option<(FrameRegion, FrameRegion)> {
        if split_at == 0 || split_at >= self.count {
            return None;
        }

        let lower_address = self.physical_address;
        let lower_count = split_at;

        let upper_address = self.physical_address + split_at * page_frame_size();
        let upper_count = self.count - split_at;

        let lower = FrameRegion {
            physical_address: lower_address,
            count: lower_count,
        };

        let upper = FrameRegion {
            physical_address: upper_address,
            count: upper_count,
        };

        mem::forget(self);
        Some((lower, upper))
    }

    /// Merges the two [`FrameRegion`]s into a single [`FrameRegion`].
    ///
    /// This requires that the two [`FrameRegion`] are adjacent.
    pub fn merge(&self, other: &FrameRegion) -> Option<FrameRegion> {

    }
}

impl Drop for FrameRegion {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of frames indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.physical_address, self.count) }
    }
}

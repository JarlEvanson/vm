//! Definitions and implementations of physical memory management services for use by `revm`.

use core::{error, fmt};

mod structs;

pub use structs::*;

/// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`].
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocated the requested frames.
pub fn allocate_frames(
    count: u64,
    policy: AllocationPolicy,
    alignment: u64,
) -> Result<FrameAllocation, OutOfMemory> {
    todo!("allocate_frames({count:#x}, {policy:?}, {alignment:#x})")
}

/// Deallocates the provided physical [`FrameRange`].
///
/// # Safety
///
/// - The provided [`FrameRange`] must be valid to deallocate at this time and must not be used
///   after this call.
pub unsafe fn deallocate_frames(range: FrameRange) {
    todo!("deallocate_frames({range})")
}

/// Wrapper around a [`FrameRange`] allocated with [`allocate_frames()`] that automatically frees
/// the allocated [`FrameRange`] when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameAllocation(FrameRange);

impl FrameAllocation {
    /// Creates a new [`FrameAllocation`] for automatic physical memory management purposes.
    ///
    /// # Safety
    ///
    /// The provided [`FrameRange`] must not be used after this [`FrameAllocation`] is dropped, and
    /// if this [`FrameAllocation`] is dropped, it must be valid to deallocate.
    pub unsafe fn new(range: FrameRange) -> Self {
        Self(range)
    }

    /// Returns the [`FrameRange`] this [`FrameAllocation`] controls.
    pub const fn range(&self) -> FrameRange {
        self.0
    }
}

impl Drop for FrameAllocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The [`FrameRange`] indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.0) }
    }
}

/// The policy determining the valid [`FrameRange`]s for this allocation request.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
pub enum AllocationPolicy {
    /// Any frame region is suitable for allocation.
    #[default]
    Any,
}

impl fmt::Debug for AllocationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.pad("Any"),
        }
    }
}

/// Indicates that the memory allocation operation failed.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("out of memory")
    }
}

impl error::Error for OutOfMemory {}

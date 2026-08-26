mod structs;

pub use structs::*;

use crate::memory::phys::{FrameRange, PhysicalAddress};

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for physical memory corresponding to RAM.
pub fn map(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    todo!()
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for physical memory that should bypass the CPU cache (e.g., DMA buffers).
pub fn map_noncacheable(
    frames: FrameRange,
    permissions: Permissions,
) -> Result<PageMapping, MapError> {
    todo!()
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for memory-mapped device registers where normal caching is unsafe.
pub fn map_device(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    todo!()
}

/// Identity maps the provided [`FrameRange`] into `revm`'s virtual memory with the requested
/// [`Permissions`]s.
pub fn map_identity(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    todo!()
}

/// Maps the [`page_frame_size()`][pfs] physical memory region inside of which `address` is
/// contained into `revm`'s virtual address space with [`Permissions::ReadWrite`] and
/// [`MappingType::Normal`]. The corresponding virtual address is returned. Any call to this
/// function invalidates all previous mappings produced by
/// [`map_temporary()`].
///
/// This means that if `physical_address` is 1 byte from the top of a [`page_frame_size()`][pfs]-ed
/// chunk of memory, only 1 byte may be accessible.
///
/// [pfs]: crate::memory::page_frame_size()
pub fn map_temporary(address: PhysicalAddress) -> VirtualAddress {
    todo!()
}

/// Translates the provided [`VirtualAddress`] to its corresponding [`PhysicalAddress`].
///
/// This also returns the [`Permissions`] and [`MappingType`] associated with the mapping.
/// If the address is not mapped, [`None`] is returned.
pub fn translate_virt(
    address: VirtualAddress,
) -> Option<(Permissions, MappingType, PhysicalAddress)> {
    todo!()
}

/// Unmaps the provided [`PageRange`] from virtual memory.
///
/// # Safety
///
/// - The provided [`PageRange`] must not be accessed after this call.
pub unsafe fn unmap(range: PageRange) {
    todo!()
}

/// Wrapper around a region of pages mapped with [`map()`], [`map_noncacheable()`], [`map_device`],
/// or [`map_identity()`].
///
/// This structure automatically unmaps the [`PageRange`] when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageMapping(PageRange);

impl PageMapping {
    /// Creates a new [`PageMapping`] for automatic virtual memory management.
    ///
    /// # Safety
    ///
    /// The provided [`PageRange`] must not be used after this [`PageMapping`] is dropped, and if
    /// this [`PageMapping`] is dropped, it must be valid to unmap.
    pub unsafe fn new(range: PageRange) -> Self {
        Self(range)
    }

    /// Returns the [`PageRange`] that this [`PageMapping`] owns.
    pub const fn range(&self) -> PageRange {
        self.0
    }
}

impl Drop for PageMapping {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of virtual memory owned by this mapping is no longer
        // accessible once the wrapper is dropped.
        unsafe { unmap(self.0) }
    }
}

pub enum MapError {}

/// The permissions of the [`PageRange`], thereby determining the valid access types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Permissions {
    /// The [`PageRange`] should be readable.
    #[default]
    Read,
    /// The [`PageRange`] should be readable and writable.
    ReadWrite,
    /// The [`PageRange`] should be readable and executable.
    ReadExecute,
    /// The [`PageRange`] should be readable, writeable, and executable.
    ReadWriteExecute,
}

/// The use case of the [`PageRange`] (which determines its cacheability and shareability
/// requirements).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MappingType {
    /// The [`PageRange`] represents normal memory.
    #[default]
    Normal,
    /// The [`PageRange`] represents uncacheable normal memory (typically for DMA).
    NormalNoncacheable,
    /// The [`PageRange`] represents device memory (memory-mapped registers).
    Device,
}

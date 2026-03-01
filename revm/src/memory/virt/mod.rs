//! Definitions and implementations of virtual memory management APIs for `revm`.

use core::{error, fmt};

use conversion::u64_to_usize_strict;

use crate::memory::{
    phys::{
        FrameAllocationError,
        structs::{Frame, FrameRange},
    },
    virt::structs::{Page, PageRange},
};

pub mod structs;

/// Maps a range of physical frames into virtual memory with normal caching.
///
/// This is typically used for physical memory corresponding to RAM.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm`'s virtual address space does not have
///   a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map(frame_range: FrameRange, permissions: Permissions) -> Result<PageRange, MapError> {
    map_internal(frame_range, permissions, MappingType::Normal)
}

/// Maps a range of physical frames as non-cacheable memory.
///
/// Typically used for memory that should bypass the CPU cache (e.g., DMA buffers).
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm`'s virtual address space does not have
///   a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_noncacheable(
    frame_range: FrameRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_internal(frame_range, permissions, MappingType::NormalNoncacheable)
}

/// Maps a range of physical frames as device memory.
///
/// Suitable for memory-mapped device registers where normal caching is unsafe.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm`'s virtual address space does not have
///   a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_device(
    frame_range: FrameRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_internal(frame_range, permissions, MappingType::Device)
}

/// Maps a range of physical frames with write-combining enabled.
///
/// Useful for framebuffers or other memory regions where write-combining improves performance.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm`'s virtual address space does not have
///   a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_write_combining(
    frame_range: FrameRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_internal(frame_range, permissions, MappingType::WriteCombining)
}

/// Maps a [`FrameRange`] into virtual memory at [`PageRange`] with normal caching.
///
/// This is typically used for physical memory corresponding to RAM.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Never returned, any existing mapping that overlap with
///   the requested [`PageRange`] are overwritten.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_at(
    frame_range: FrameRange,
    page_range: PageRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_at_internal(frame_range, page_range, permissions, MappingType::Normal)
}

/// Maps a [`FrameRange`] into virtual memory at [`PageRange`] as non-cacheable memory.
///
/// Typically used for memory that should bypass the CPU cache (e.g., DMA buffers).
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Never returned, any existing mapping that overlap with
///   the requested [`PageRange`] are overwritten.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_noncacheable_at(
    frame_range: FrameRange,
    page_range: PageRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_at_internal(
        frame_range,
        page_range,
        permissions,
        MappingType::NormalNoncacheable,
    )
}

/// Maps a [`FrameRange`] into virtual memory at [`PageRange`] as device memory.
///
/// Suitable for memory-mapped device registers where normal caching is unsafe.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Never returned, any existing mapping that overlap with
///   the requested [`PageRange`] are overwritten.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_device_at(
    frame_range: FrameRange,
    page_range: PageRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_at_internal(frame_range, page_range, permissions, MappingType::Device)
}

/// Maps a [`FrameRange`] into virtual memory at [`PageRange`] as write-combining memory.
///
/// Useful for framebuffers or other memory regions where write-combining improves performance.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Never returned, any existing mapping that overlap with
///   the requested [`PageRange`] are overwritten.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
pub fn map_write_combining_at(
    frame_range: FrameRange,
    page_range: PageRange,
    permissions: Permissions,
) -> Result<PageRange, MapError> {
    map_at_internal(
        frame_range,
        page_range,
        permissions,
        MappingType::WriteCombining,
    )
}

/// Helper function that locates a free virtual [`PageRange`] and then calls [`map_at()`].
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm`'s virtual address space does not have
///   a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
fn map_internal(
    frame_range: FrameRange,
    permissions: Permissions,
    mapping_type: MappingType,
) -> Result<PageRange, MapError> {
    let page_range = if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let total_bytes = frame_range.byte_count();
        let stub_page_frame_count =
            u64_to_usize_strict(total_bytes.div_ceil(generic_table.page_frame_size));

        let mut page = Page::zero().strict_add(1);
        loop {
            // SAFETY:
            //
            // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
            // functional.
            let result = unsafe {
                (generic_table.map)(
                    Frame::zero().start_address().value(),
                    page.start_address().value(),
                    stub_page_frame_count,
                    stub_api::MapFlags::READ | stub_api::MapFlags::WRITE,
                )
            };
            if result == stub_api::Status::SUCCESS {
                break PageRange::new(page, u64_to_usize_strict(frame_range.count()));
            } else if result != stub_api::Status::OVERLAP {
                crate::warn!("error while testing for free page: {result:?}");
            }

            page = page.checked_add(1).ok_or(MapError::FindFreeRegionError)?;
        }
    } else {
        todo!("implement post-takeover free region discovery")
    };

    map_at_internal(frame_range, page_range, permissions, mapping_type)
}

/// Root function that maps the provided [`FrameRange`] into `revm`'s address space at
/// [`PageRange`] with the requested [`Permissions`] and [`MappingType`].
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Never returned, any existing mapping that overlap with
///   the requested [`PageRange`] are overwritten.
/// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`]s that
///   are required to map the requested [`FrameRange`].
fn map_at_internal(
    frame_range: FrameRange,
    page_range: PageRange,
    permissions: Permissions,
    mapping_type: MappingType,
) -> Result<PageRange, MapError> {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let total_bytes = frame_range.byte_count();
        let stub_page_frame_count =
            u64_to_usize_strict(total_bytes.div_ceil(generic_table.page_frame_size));

        let mut flags = match permissions {
            Permissions::Read => stub_api::MapFlags::READ,
            Permissions::ReadWrite => stub_api::MapFlags::READ | stub_api::MapFlags::WRITE,
            Permissions::ReadExecute => stub_api::MapFlags::READ | stub_api::MapFlags::EXEC,
        };

        // This function always allows overwriting of existing mappings.
        flags |= stub_api::MapFlags::MAY_OVERWRITE;

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.map)(
                frame_range.start().start_address().value(),
                page_range.start().start_address().value(),
                stub_page_frame_count,
                flags,
            )
        };
        if result != stub_api::Status::SUCCESS {
            crate::trace!(
                "map_at({frame_range:x?}, {page_range:x?}, {permissions:?}, {mapping_type:?}) -> {result:?}"
            );
            return Err(MapError::FrameAllocation(FrameAllocationError));
        }

        Ok(page_range)
    } else {
        todo!("implement post-takeover page mapping")
    }
}

/// Unmaps the [`PageRange] from `revm`'s virtual memory.
///
/// # Safety
///
/// The virtual memory region described by [`PageRange`] must not be in use.
pub unsafe fn unmap(page_range: PageRange) {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let total_bytes = page_range.byte_count();
        let stub_page_count =
            total_bytes.div_ceil(u64_to_usize_strict(generic_table.page_frame_size));

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.unmap)(page_range.start().start_address().value(), stub_page_count)
        };
        if result != stub_api::Status::SUCCESS {
            crate::warn!("error unmapping pages: {result:?}");
        }
    } else {
        todo!("implement post-takeover page unmapping")
    }
}

/// Various errors that can occur while mapping a [`FrameRange`] into memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    /// The virtual address space of `revm` does not cotain any region large enough to fulfill the
    /// requested mapping.
    FindFreeRegionError,
    /// An error occurred while allocating physical memory required to map a [`FrameRange`] into
    /// `revm`'s virtual address space.
    FrameAllocation(FrameAllocationError),
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FindFreeRegionError => {
                write!(f, "error while search for for region: not found")
            }
            Self::FrameAllocation(error) => {
                write!(f, "error allocating page table frames: {error}")
            }
        }
    }
}

impl error::Error for MapError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::FindFreeRegionError => None,
            Self::FrameAllocation(error) => Some(error),
        }
    }
}

/// Determines the valid access types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Permissions {
    /// The [`PageRange`] should be readable.
    #[default]
    Read,
    /// The [`PageRange`] should be readable and writable.
    ReadWrite,
    /// The [`PageRange`] should be readable and executable.
    ReadExecute,
}

/// Determines the cacheability and shareability of the [`PageRange`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MappingType {
    /// The [`PageRange`] represents normal memory.
    #[default]
    Normal,
    /// The [`PageRange`] represents uncacheable normal memory (typically DMA memory).
    NormalNoncacheable,
    /// The [`PageRange`] represents device memory (memory-mapped registers).
    Device,
    /// The [`PageRange`] represents device memory on which it is safe to perform write-combining
    /// (typically framebuffers).
    WriteCombining,
}

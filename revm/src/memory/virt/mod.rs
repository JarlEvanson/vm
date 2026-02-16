//! Definitions and implementations of virtual memory management APIs for `revm`.

use core::{convert::Infallible, error, fmt};

use sync::ControlledModificationCell;

use crate::{
    arch::generic::memory::virt::FindFreeRegionError,
    memory::{
        page_frame_size,
        phys::{
            FrameAllocationError,
            structs::{Frame, FrameRange},
        },
        virt::structs::{Page, PageRange},
    },
    util::{u64_to_usize_panicking, usize_to_u64},
};

pub mod structs;

/// The [`Page`] at which all temporary mappings will occur.
static TEMPORARY_PAGE: ControlledModificationCell<Page> =
    ControlledModificationCell::new(Page::zero());

/// Initializes the virtual memory management subsystem for `revm`.
///
/// # Safety
///
/// This function must be called before any virtual memory APIs may be called and may only be
/// called a single time.
pub(in crate::memory) unsafe fn initialize_virtual() {
    let generic_table = crate::stub_protocol::generic_table()
        .expect("initialize_virtual() must be called before `takeover()`");

    let stub_page_frame_count = u64_to_usize_panicking(
        usize_to_u64(page_frame_size()).div_ceil(generic_table.page_frame_size),
    );

    let mut page = Page::zero().add(1);
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
            // SAFETY:
            //
            // This function has been called before any virtual memory APIs have been called and
            // thus currently has exclusive access to [`TEMPORARY_PAGE`].
            unsafe { *TEMPORARY_PAGE.get_mut() = page }
            return;
        } else if result != stub_api::Status::OVERLAP {
            crate::warn!("error while testing for free page: {result:?}");
        }

        page = page.add(1);
    }
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
        let total_bytes = frame_range
            .count()
            .strict_mul(usize_to_u64(page_frame_size()));
        let stub_page_frame_count =
            u64_to_usize_panicking(total_bytes.div_ceil(generic_table.page_frame_size));

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
        let total_bytes = page_range.count().strict_mul(page_frame_size());
        let stub_page_count =
            total_bytes.div_ceil(u64_to_usize_panicking(generic_table.page_frame_size));

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

/// Maps the [`Frame`] into memory temporarily.
///
/// Any call to [`map_temporary()`] invalidates the temporary mappings produced by any previous
/// call to [`map_temporary()`].
#[expect(clippy::missing_panics_doc)]
pub fn map_temporary(frame: Frame) -> Page {
    let frame_range = FrameRange::new(frame, 1);
    let page_range = PageRange::new(*TEMPORARY_PAGE.get(), 1);

    map_at_internal(
        frame_range,
        page_range,
        Permissions::ReadWrite,
        MappingType::Device,
    )
    .expect("failed to map temporary page");
    page_range.start()
}

/// Various errors that can occur while mapping a [`FrameRange`] into memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    /// The virtual address space of `revm` does not cotain any region large enough to fulfill the
    /// requested mapping.
    FindFreeRegionError(FindFreeRegionError<Infallible>),
    /// An error occurred while allocating physical memory required to map a [`FrameRange`] into
    /// `revm`'s virtual address space.
    FrameAllocation(FrameAllocationError),
}

impl From<FindFreeRegionError<Infallible>> for MapError {
    fn from(error: FindFreeRegionError<Infallible>) -> Self {
        Self::FindFreeRegionError(error)
    }
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FindFreeRegionError(error) => {
                write!(f, "error while search for free virtual region: {error}")
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
            Self::FindFreeRegionError(error) => Some(error),
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

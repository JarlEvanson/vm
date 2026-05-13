//! Definitions and interfaces that platforms utilize to provide virtual memory management services
//! for use by the rest of the executable.

use core::{cmp::min, error, fmt, ptr};

use conversion::usize_to_u64;
use sync::ControlledModificationCell;

use crate::platform::{
    FrameRange, OutOfMemory, PageRange, PhysicalAddress, VirtualAddress, page_size,
};

/// The current [`VirtualMemoryManager`].
static VIRTUAL_MEMORY_MANAGER: ControlledModificationCell<
    Option<&'static dyn VirtualMemoryManager>,
> = ControlledModificationCell::new(None);

/// Initializes the virtual memory management subsystem.
///
/// # Safety
///
/// - This function must not be called when any other physical memory management function is
///   active.
/// - The provided [`max_physical_address()`] implementation must return a value greater than or
///   equal to the current [`max_physical_address()`], if the physical memory management subsystem
///   has previously been initialized.
pub(in crate::platform) unsafe fn initialize_virtual_memory_manager(
    manager: &'static dyn VirtualMemoryManager,
) {
    // SAFETY:
    //
    // The invariants of [`initialize_virtual_memory_manager()`] ensure that this operation is safe.
    unsafe { *VIRTUAL_MEMORY_MANAGER.get_mut() = Some(manager) }
}

/// Returns the currently active [`VirtualMemoryManager`].
fn virtual_memory_manager() -> &'static dyn VirtualMemoryManager {
    VIRTUAL_MEMORY_MANAGER
        .get()
        .expect("virtual memory management subsystem is uninitialized")
}

/// Returns the inclusive maximum [`PhysicalAddress`] that can be mapped.
///
/// This is monontic value (i.e., it can only ever stay the same or increase).
pub fn max_physical_address() -> PhysicalAddress {
    virtual_memory_manager().max_physical_address()
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for physical memory corresponding to RAM.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
///   have a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating [`Frame`][f]s
///   that are required to map the requested [`FrameRange`] into memory.
///
/// [f]: crate::platform::Frame
pub fn map(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    virtual_memory_manager()
        .map(frames, permissions, MappingType::Normal)
        .map(PageMapping)
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for physical memory that should bypass the CPU cache (e.g., DMA buffers).
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
///   have a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating [`Frame`][f]s
///   that are required to map the requested [`FrameRange`] into memory.
///
/// [f]: crate::platform::Frame
pub fn map_noncacheable(
    frames: FrameRange,
    permissions: Permissions,
) -> Result<PageMapping, MapError> {
    virtual_memory_manager()
        .map(frames, permissions, MappingType::NormalNoncacheable)
        .map(PageMapping)
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for memory-mapped device registers where normal caching is unsafe.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
///   have a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating [`Frame`][f]s
///   that are required to map the requested [`FrameRange`] into memory.
///
/// [f]: crate::platform::Frame
pub fn map_device(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    virtual_memory_manager()
        .map(frames, permissions, MappingType::Device)
        .map(PageMapping)
}

/// Maps the provided [`FrameRange`] into virtual memory with the requested [`Permissions`].
///
/// This is typically used for memory regions where write-combining improves performance (e.g.,
/// framebuffers).
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
///   have a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating [`Frame`][f]s
///   that are required to map the requested [`FrameRange`] into memory.
///
/// [f]: crate::platform::Frame
pub fn map_write_combining(
    frames: FrameRange,
    permissions: Permissions,
) -> Result<PageMapping, MapError> {
    virtual_memory_manager()
        .map(frames, permissions, MappingType::WriteCombining)
        .map(PageMapping)
}

/// Identity maps the provided [`FrameRange`] into `revm-stub`'s virtual memory with the
/// requested [`Permissions`]s.
///
/// # Errors
///
/// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
///   have a suitable [`PageRange`] for the requested mapping.
/// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating [`Frame`][f]s
///   that are required to map the requested [`FrameRange`] into memory.
///
/// [f]: crate::platform::Frame
pub fn map_identity(frames: FrameRange, permissions: Permissions) -> Result<PageMapping, MapError> {
    virtual_memory_manager()
        .map_identity(frames, permissions)
        .map(PageMapping)
}

/// Maps the [`page_size()`] physical memory region inside of which `address` is contained into
/// `revm-stub`'s virtual address space with [`Permissions::ReadWrite`] and
/// [`MappingType::Normal`]. The corresponding virtual address is returned. Any call to this
/// function invalidates all previous mappings produced by
/// [`map_temporary()`].
///
/// This means that if `physical_address` is 1 byte from the top of a [`page_size()`] physical
/// memory chunk, only 1 byte may be accessible.
///
/// This returns [`None`] if `address` is greater than
/// [`max_physical_address()`].
pub fn map_temporary(address: PhysicalAddress) -> Option<VirtualAddress> {
    virtual_memory_manager().map_temporary(address)
}

/// Reads the bytes located in physical memory at `address` into `bytes`.
///
/// This function calls [`map_temporary()`].
pub fn read_bytes_at(address: PhysicalAddress, bytes: &mut [u8]) -> bool {
    virtual_memory_manager().read_bytes_at(address, bytes)
}

/// Writes the bytes in `bytes` into physical memory at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_bytes_at(address: PhysicalAddress, bytes: &[u8]) -> bool {
    virtual_memory_manager().write_bytes_at(address, bytes)
}

/// Reads the `u8` located at `address` in physical memory.
///
/// This function calls [`map_temporary()`].
pub fn read_u8_at(address: PhysicalAddress) -> Option<u8> {
    virtual_memory_manager().read_u8_at(address)
}

/// Reads the `u16` located at `address` in physical memory.
///
/// This function calls [`map_temporary()`].
pub fn read_u16_at(address: PhysicalAddress) -> Option<u16> {
    virtual_memory_manager().read_u16_at(address)
}

/// Reads the `u32` located at `address` in physical memory.
///
/// This function calls [`map_temporary()`].
pub fn read_u32_at(address: PhysicalAddress) -> Option<u32> {
    virtual_memory_manager().read_u32_at(address)
}

/// Reads the `u64` located at `address` in physical memory.
///
/// This function calls [`map_temporary()`].
pub fn read_u64_at(address: PhysicalAddress) -> Option<u64> {
    virtual_memory_manager().read_u64_at(address)
}

/// Writes the `u8` into physical memory at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u8_at(address: PhysicalAddress, value: u8) -> bool {
    virtual_memory_manager().write_u8_at(address, value)
}

/// Writes the `u16` into physical memory at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u16_at(address: PhysicalAddress, value: u16) -> bool {
    virtual_memory_manager().write_u16_at(address, value)
}

/// Writes the `u32` into physical memory at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u32_at(address: PhysicalAddress, value: u32) -> bool {
    virtual_memory_manager().write_u32_at(address, value)
}

/// Writes the `u64` into physical memory at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u64_at(address: PhysicalAddress, value: u64) -> bool {
    virtual_memory_manager().write_u64_at(address, value)
}

/// Translates the provided [`VirtualAddress`] to its corresponding [`PhysicalAddress`].
///
/// This also returns the [`Permissions`] and [`MappingType`] associated with the mapping.
/// If the address is not mapped, [`None`] is returned.
pub fn translate_virt(
    address: VirtualAddress,
) -> Option<(Permissions, MappingType, PhysicalAddress)> {
    virtual_memory_manager().translate_virtual(address)
}

/// Unmaps the provided [`PageRange`] from virtual memory.
///
/// # Safety
///
/// - The provided [`PageRange`] must not be accessed after this call.
pub unsafe fn unmap(range: PageRange) {
    // SAFETY:
    //
    // The safety invariants are passed through to the underlying manager.
    unsafe { virtual_memory_manager().unmap(range) }
}

/// Wrapper around a region of pages mapped with [`map()`] or [`map_identity()`].
///
/// This structure automatically unmaps the region of pages when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageMapping(PageRange);

impl PageMapping {
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

/// Trait representing a platform-independent mechanism for virtual memory management.
pub(in crate::platform) trait VirtualMemoryManager: Send + Sync {
    /// Returns the inclusive maximum [`PhysicalAddress`] that can be mapped.
    fn max_physical_address(&self) -> PhysicalAddress;

    /// Maps the provided [`FrameRange`] into `revm-stub`'s virtual memory with the requested
    /// [`Permissions`]s and [`MappingType`].
    ///
    /// # Errors
    ///
    /// - [`MapError::FindFreeRegionError`]: Returned when `revm-stub`'s virtual memory does not
    ///   have a suitable [`PageRange`] for the requested mapping.
    /// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating
    ///   [`Frame`][f]s that are required to map the requested [`FrameRange`] into memory.
    ///
    /// [f]: crate::platform::Frame
    fn map(
        &self,
        frames: FrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<PageRange, MapError>;

    /// Identity maps the provided [`FrameRange`] into `revm-stub`'s virtual memory with the
    /// requested [`Permissions`]s.
    ///
    /// # Errors
    ///
    /// - [`MapError::FindFreeRegionError`]: Returned if the requested [`PageRange`] has already
    ///   been mapped.
    /// - [`MapError::FrameAllocation`]: Returned when an error occurrs when allocating
    ///   [`Frame`][f]s that are required to map the requested [`FrameRange`] into memory.
    ///
    /// [f]: crate::platform::Frame
    fn map_identity(
        &self,
        frames: FrameRange,
        permissions: Permissions,
    ) -> Result<PageRange, MapError>;

    /// Maps the [`page_size()`] physical memory region inside of which `address` is contained into
    /// `revm-stub`'s virtual address space with [`Permissions::ReadWrite`] and
    /// [`MappingType::Normal`]. The corresponding virtual address is returned. Any call to this
    /// function invalidates all previous mappings produced by
    /// [`VirtualMemoryManager::map_temporary()`].
    ///
    /// This means that if `physical_address` is 1 byte from the top of a [`page_size()`] physical
    /// memory chunk, only 1 byte may be accessible.
    ///
    /// This returns [`None`] if `address` is greater than
    /// [`VirtualMemoryManager::max_physical_address()`].
    fn map_temporary(&self, address: PhysicalAddress) -> Option<VirtualAddress>;

    /// Translates the provided [`VirtualAddress`] to its corresponding [`PhysicalAddress`].
    ///
    /// This also returns the [`Permissions`] and [`MappingType`] associated with the mapping.
    /// If the address is not mapped, [`None`] is returned.
    fn translate_virtual(
        &self,
        address: VirtualAddress,
    ) -> Option<(Permissions, MappingType, PhysicalAddress)>;

    /// Unmaps the [`PageRange`] from `revm-stub`'s virtual memory.
    ///
    /// # Safety
    ///
    /// The virtual memory region described by [`PageRange`] must not be in use.
    unsafe fn unmap(&self, range: PageRange);

    // Default implementations of useful methods.

    /// Reads the bytes located in physical memory at `address` into `bytes`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn read_bytes_at(&self, mut address: PhysicalAddress, mut bytes: &mut [u8]) -> bool {
        if address.checked_add(usize_to_u64(bytes.len())).is_none() {
            return false;
        }

        while !bytes.is_empty() {
            let Some(virt) = self.map_temporary(address) else {
                return false;
            };

            let page_offset = virt.value() % page_size();
            let remaining_in_page = page_size() - page_offset;
            let chunk_len = min(bytes.len(), remaining_in_page);

            // SAFETY:
            //
            // - The temporary mapping covers the containing page.
            // - `page_offset + chunk_len <= page_size()`.
            // - Destination slice is valid for `chunk_len`.
            unsafe {
                ptr::copy_nonoverlapping(
                    ptr::with_exposed_provenance(virt.value()),
                    bytes.as_mut_ptr(),
                    chunk_len,
                );
            }

            address = address.strict_add(usize_to_u64(chunk_len));
            bytes = &mut bytes[chunk_len..];
        }

        true
    }

    /// Writes the bytes in `bytes` into physical memory at `address`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn write_bytes_at(&self, mut address: PhysicalAddress, mut bytes: &[u8]) -> bool {
        if address.checked_add(usize_to_u64(bytes.len())).is_none() {
            return false;
        }

        while !bytes.is_empty() {
            let Some(virt) = self.map_temporary(address) else {
                return false;
            };

            let page_offset = virt.value() % page_size();
            let remaining_in_page = page_size() - page_offset;
            let chunk_len = min(bytes.len(), remaining_in_page);

            // SAFETY:
            //
            // - The temporary mapping covers the containing page.
            // - `page_offset + chunk_len <= page_size()`.
            // - Destination slice is valid for at least `chunk_len` bytes.
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    ptr::with_exposed_provenance_mut(virt.value()),
                    chunk_len,
                );
            }

            address = address.strict_add(usize_to_u64(chunk_len));
            bytes = &bytes[chunk_len..];
        }

        true
    }

    /// Reads the `u8` located at `address` in physical memory.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn read_u8_at(&self, address: PhysicalAddress) -> Option<u8> {
        let mut bytes = [0u8; 1];

        if !self.read_bytes_at(address, &mut bytes) {
            return None;
        }

        Some(bytes[0])
    }

    /// Reads the `u16` located at `address` in physical memory.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn read_u16_at(&self, address: PhysicalAddress) -> Option<u16> {
        let mut bytes = [0u8; 2];

        if !self.read_bytes_at(address, &mut bytes) {
            return None;
        }

        Some(u16::from_ne_bytes(bytes))
    }

    /// Reads the `u32` located at `address` in physical memory.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn read_u32_at(&self, address: PhysicalAddress) -> Option<u32> {
        let mut bytes = [0u8; 4];

        if !self.read_bytes_at(address, &mut bytes) {
            return None;
        }

        Some(u32::from_ne_bytes(bytes))
    }

    /// Reads the `u64` located at `address` in physical memory.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn read_u64_at(&self, address: PhysicalAddress) -> Option<u64> {
        let mut bytes = [0u8; 8];

        if !self.read_bytes_at(address, &mut bytes) {
            return None;
        }

        Some(u64::from_ne_bytes(bytes))
    }

    /// Writes the `u8` into physical memory at `address`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn write_u8_at(&self, address: PhysicalAddress, value: u8) -> bool {
        self.write_bytes_at(address, &[value])
    }

    /// Writes the `u16` into physical memory at `address`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn write_u16_at(&self, address: PhysicalAddress, value: u16) -> bool {
        self.write_bytes_at(address, &value.to_ne_bytes())
    }

    /// Writes the `u32` into physical memory at `address`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn write_u32_at(&self, address: PhysicalAddress, value: u32) -> bool {
        self.write_bytes_at(address, &value.to_ne_bytes())
    }

    /// Writes the `u64` into physical memory at `address`.
    ///
    /// This function calls [`VirtualMemoryManager::map_temporary()`].
    fn write_u64_at(&self, address: PhysicalAddress, value: u64) -> bool {
        self.write_bytes_at(address, &value.to_ne_bytes())
    }
}

/// Various errors that can occur while mapping a [`FrameRange`] into memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    /// The virtual address space of `revm-stub` does not cotain any region large enough to
    /// fulfill the requested mapping.
    FindFreeRegionError,
    /// An error occurred while allocating physical memory required to map a [`FrameRange`] into
    /// `revm-stub`'s virtual address space.
    FrameAllocation(OutOfMemory),
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FindFreeRegionError => {
                write!(f, "error while searching for region: not found")
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
    /// The [`PageRange`] should be readable, writable, and executable.
    ReadWriteExecute,
}

impl Permissions {
    /// Returns `true` if the [`Permissions`] indicates the [`PageRange`] should be writable.
    pub fn writable(self) -> bool {
        match self {
            Self::ReadWrite | Self::ReadWriteExecute => true,
            Self::Read | Self::ReadExecute => false,
        }
    }

    /// Returns `true` if the [`Permissions`] indicates the [`PageRange`] should be executable.
    pub fn executable(self) -> bool {
        match self {
            Self::ReadExecute | Self::ReadWriteExecute => true,
            Self::Read | Self::ReadWrite => false,
        }
    }
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

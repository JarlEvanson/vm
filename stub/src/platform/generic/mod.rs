//! Definitions providing the interface platform implementors provide and an abstraction utilizing
//! that interface to implement services for use by the rest of the executable.

use core::{error, fmt, ptr::NonNull};

use stub_api::{MemoryDescriptor, Status, TakeoverFlags};
use sync::ControlledModificationCell;

use crate::platform::memory_structs::{FrameRange, PhysicalAddress, VirtualAddress};

/// The [`Platform`] implementation that is currently in use.
static PLATFORM: ControlledModificationCell<Option<&'static dyn Platform>> =
    ControlledModificationCell::new(None);

/// Initializes the [`Platform`] implementation to be used for its services.
///
/// # Safety
///
/// This function must be called before any function that requires [`Platform`] services is called.
pub(in crate::platform) unsafe fn platform_initialize(platform: &'static dyn Platform) {
    // SAFETY:
    //
    // The invariants of this function ensure that the mutable access begins and ends before any
    // immutable or mutable access takes place.
    unsafe { *PLATFORM.get_mut() = Some(platform) }
}

/// Cleans up the [`Platform`] implementation and any associated data.
///
/// # Safety
///
/// Any function that requires [`Platform`] services must be called before this function.
pub(in crate::platform) unsafe fn platform_teardown() {
    // SAFETY:
    //
    // The invariants of this function ensure that the mutable access begins and ends after any
    // immutable or mutable access takes place.
    unsafe { *PLATFORM.get_mut() = None }
}

/// Returns the [`Platform`] implementation that is currently in use.
pub(in crate::platform) fn platform() -> &'static dyn Platform {
    PLATFORM
        .get()
        .expect("platform implementation not initialized")
}

/// Collection of various services provided by a platform-specific implementation.
pub(in crate::platform) trait Platform: Send + Sync {
    /// Allocates a region of memory of `size` bytes aligned to `alignment`.
    fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<u8>>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The `size` and `alignment` parameters must match the parameters utilized when this region
    /// of memory was allocated. `ptr` must describe a block of memory currently allocated via a
    /// call to [`Platform::allocate()`].
    unsafe fn deallocate(&self, ptr: NonNull<u8>, size: usize, alignment: usize);

    /// Returns the size, in bytes, of a frame.
    fn frame_size(&self) -> u64;

    /// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`].
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
    /// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
    fn allocate_frames(
        &self,
        count: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory>;

    /// Allocates a region of `count` frames with a starting physical address being a multiple of
    /// `alignment` and the entire region in accordance with the provided [`AllocationPolicy`].
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
    /// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
    fn allocate_frames_aligned(
        &self,
        count: u64,
        alignment: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory> {
        assert!(alignment.is_power_of_two());

        if alignment <= self.frame_size() {
            self.allocate_frames(count, policy)
        } else {
            let total_count = alignment
                .div_ceil(self.frame_size())
                .checked_add(count)
                .ok_or(OutOfMemory)?;

            let range = self.allocate_frames(total_count, policy)?;

            let requested_range_start = range.start().align_up(alignment);
            let requested_range = FrameRange::new(requested_range_start, count);

            let lower = range.split_at(requested_range.start()).unwrap().0;
            let upper = range.split_at(requested_range.end()).unwrap().1;

            if !lower.is_empty() {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(lower) };
            }

            if !upper.is_empty() {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(upper) };
            }

            Ok(requested_range)
        }
    }

    /// Deallocates a region of `count` frames with the starting `physical_address`.
    ///
    /// # Safety
    ///
    /// These frames must have been allocated by a call to [`Platform::allocate_frames()`].
    unsafe fn deallocate_frames(&self, range: FrameRange);

    /// Returns the current physical [`MemoryMap`].
    ///
    /// # Errors
    ///
    /// [`BufferTooSmall`] is returned when the provided `buffer` is too small, and contains the
    /// required size of the buffer. Any allocations or deallocations may change the memory map.
    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [MemoryDescriptor],
    ) -> Result<MemoryMap<'buffer>, BufferTooSmall>;

    /// Returns the size, in bytes, of a page.
    fn page_size(&self) -> usize;

    /// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
    /// contained into the stub's virtual address space. Any call to this function will invalidate
    /// all previous temporary mappings produced by [`Platform::map_temporary()`].
    ///
    /// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
    /// chunk, only 1 byte may be accessible.
    fn map_temporary(&self, address: PhysicalAddress) -> *mut u8;

    /// Maps the physical memory region of `size` bytes into `revm-stub`'s virtual address space.
    fn map_identity(&self, address: PhysicalAddress, size: u64) -> *mut u8;

    /// Returns the physical address associated with the provided `virtual_address` if the
    /// translation is valid. Otherwise, return [`None`].
    fn translate_virtual(&self, address: VirtualAddress) -> Option<u64>;

    /// Executes a takover on behalf of the loaded executable. `key` is utilized to ensure that the
    /// memory map that the loaded executable has is accurate.
    ///
    /// On success, the executable becomes the sole controller of the system. This means that the
    /// executable is free to directly manipulate the hardware in whatever manner it desires.
    fn takeover(&self, key: u64, flags: TakeoverFlags) -> Status;

    /// Prints the provided [`Arguments`][fmt::Arguments].
    fn print(&self, args: fmt::Arguments);

    /// Returns the physical address of the UEFI system table, if present.
    fn uefi_system_table(&self) -> Option<PhysicalAddress>;

    /// Returns the physical address of the RSDP structure, if present.
    fn rsdp(&self) -> Option<PhysicalAddress>;

    /// Returns the physical address of the XSDP structure, if present.
    fn xsdp(&self) -> Option<PhysicalAddress>;

    /// Returns the physical address of the device tree structure, if present.
    fn device_tree(&self) -> Option<PhysicalAddress>;

    /// Returns the physical address of the SMBIO 32 Entry Point, if present.
    fn smbios_32(&self) -> Option<PhysicalAddress>;

    /// Returns the physical address of the SMBIO 64 Entry Point, if present.
    fn smbios_64(&self) -> Option<PhysicalAddress>;
}

/// Structure controlling the behavior of [`Platform::allocate_frames()`].
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocationPolicy {
    /// Any frame region is suitable for allocation.
    #[default]
    Any,
    /// Only the frame region with the starting physical address that matches the associated
    /// physical address is valid for allocated.
    At(u64),
    /// The entire frame region must be below the associated physical address.
    Below(u64),
}

impl fmt::Debug for AllocationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.pad("Any"),
            Self::At(value) => write!(f, "At({value:#x})"),
            Self::Below(value) => write!(f, "Below({value:#x})"),
        }
    }
}

/// A physical [`MemoryMap`], containing the current layout and designation of physical memory.
///
/// This may be invalided by any allocation or deallocation.
pub struct MemoryMap<'buffer> {
    /// The buffer containing the [`MemoryDescriptor`]s.
    buffer: &'buffer mut [MemoryDescriptor],
    /// The key associated with said descriptors.
    key: u64,
}

impl<'buffer> MemoryMap<'buffer> {
    /// Returns a newly created [`MemoryMap`] with the provided [`MemoryDescriptor`]s.
    pub const fn new(buffer: &'buffer mut [MemoryDescriptor], key: u64) -> Self {
        Self { buffer, key }
    }

    /// Returns the [`MemoryDescriptor`]s associated with this [`MemoryMap`].
    pub const fn descriptors(&self) -> &[MemoryDescriptor] {
        self.buffer
    }

    /// Returns the unique value that identifies this [`MemoryMap`].
    pub const fn key(&self) -> u64 {
        self.key
    }
}

/// Indicates that there were no frame regions that were free and complied with the provided flags.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("out of memory")
    }
}

impl error::Error for OutOfMemory {}

/// Indicates that the provided buffer was too small to contain the memory map.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferTooSmall {
    /// The required number of entries in the buffer.
    pub required_count: usize,
}

impl fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("buffer too small")
    }
}

impl error::Error for BufferTooSmall {}

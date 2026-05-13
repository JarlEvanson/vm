//! Definitions and interfaces that platforms utilize to provide physical memory management services
//! for use by the rest of the executable.

use core::{error, fmt};

use sync::ControlledModificationCell;

use crate::platform::{Frame, FrameRange, PhysicalAddress, PhysicalAddressRange, frame_size};

/// The current [`PhysicalMemoryManager`].
static PHYSICAL_MEMORY_MANAGER: ControlledModificationCell<
    Option<&'static dyn PhysicalMemoryManager>,
> = ControlledModificationCell::new(None);

/// Initializes the physical memory management subsystem.
///
/// # Safety
///
/// This function must not be called when any other physical memory management function is active.
pub(in crate::platform) unsafe fn initialize_physical_memory_manager(
    manager: &'static dyn PhysicalMemoryManager,
) {
    // SAFETY:
    //
    // The invariants of [`initialize_physical_memory_manager()`] ensure that this operation is
    // safe.
    unsafe { *PHYSICAL_MEMORY_MANAGER.get_mut() = Some(manager) }
}

/// Returns the currently active [`PhysicalMemoryManager`].
fn physical_memory_manager() -> &'static dyn PhysicalMemoryManager {
    PHYSICAL_MEMORY_MANAGER
        .get()
        .expect("physical memory management subsystem is uninitialized")
}

/// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`].
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
/// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
pub fn allocate_frames(
    count: u64,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, OutOfMemory> {
    physical_memory_manager()
        .allocate_frames(count, policy)
        .map(FrameAllocation)
}

/// Allocates a region of `count` frames with a starting physical address being a multiple of
/// `alignment` and the entire region in accordance with the provided [`AllocationPolicy`].
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
/// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
pub fn allocate_frames_aligned(
    count: u64,
    alignment: u64,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, OutOfMemory> {
    physical_memory_manager()
        .allocate_frames_aligned(count, alignment, policy)
        .map(FrameAllocation)
}

/// Allocates at least `byte_count` bytes with an alignment of `alignment` in accordance with the
/// provided [`AllocationPolicy`].
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested physical memory. This may
/// not indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
pub fn allocate_physical(
    byte_count: u64,
    alignment: u64,
    policy: AllocationPolicy,
) -> Result<FrameAllocation, OutOfMemory> {
    let frame_count = byte_count.div_ceil(frame_size());
    allocate_frames_aligned(frame_count, alignment, policy)
}

/// Deallocates the provided physical [`FrameRange`].
///
/// # Safety
///
/// - The provided [`FrameRange`] must not be used after this call.
pub unsafe fn deallocate_frames(range: FrameRange) {
    // SAFETY:
    //
    // The invariants of [`deallocate_frames()`] ensure that the invariants of
    // `physical_memory_manager().deallocate_frames()` are fulfilled.
    unsafe { physical_memory_manager().deallocate_frames(range) }
}

/// Deallocates the physical memory region that starts at `base` and extends for at least
/// `byte_count` bytes with an alignment of `alignment`.
///
/// # Safety
///
/// - The provided physical memory region must not be used after this call.
pub unsafe fn deallocate_physical(base: PhysicalAddress, byte_count: u64) {
    let frame_count = byte_count.div_ceil(frame_size());
    let range = FrameRange::new(Frame::containing_address(base), frame_count);
    // SAFETY:
    //
    // The invariants of [`deallocate_physical()`] ensure that the invariants of
    // [`deallocate_frames()`]s are fulfilled.
    unsafe { deallocate_frames(range) }
}

/// Returns the current physical [`MemoryMap`].
///
/// # Errors
///
/// [`BufferTooSmall`] is returned when the provided `buffer` is too small, and contains the
/// required size of the buffer. Any allocations or deallocations may change the memory map.
pub fn memory_map<'buffer>(
    buffer: &'buffer mut [MemoryDescriptor],
) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
    physical_memory_manager().memory_map(buffer)
}

/// Wrapper around a region of frames allocated with [`allocate_frames()`] or
/// [`allocate_frames_aligned()`].
///
/// This structure automatically frees the region of frames when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameAllocation(FrameRange);

impl FrameAllocation {
    /// Returns the [`FrameRange`] that this [`FrameAllocation`] owns.
    pub const fn range(&self) -> FrameRange {
        self.0
    }
}

impl Drop for FrameAllocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of frames indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.0) }
    }
}

/// Trait representing a platform-independent mechanism for physical memory management.
pub(in crate::platform) trait PhysicalMemoryManager: Send + Sync {
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

        if frame_size() >= alignment {
            self.allocate_frames(count, policy)
        } else {
            let total_count = alignment
                .div_ceil(frame_size())
                .checked_add(count)
                .ok_or(OutOfMemory)?;

            let chunk_range = self.allocate_frames(total_count, policy)?;

            let returned_chunk = chunk_range
                .start()
                .checked_align_up(alignment)
                .ok_or(OutOfMemory)?;
            let requested_range = FrameRange::new(returned_chunk, count);

            let (lower, middle, upper) = chunk_range.partition(requested_range);

            if let Some(lower) = lower {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(lower) };
            }

            if let Some(upper) = upper {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(upper) };
            }

            middle.ok_or_else(|| unreachable!())
        }
    }

    /// Deallocates the provided physical [`FrameRange`].
    ///
    /// # Safety
    ///
    /// - The provided [`FrameRange`] must not be used after this call.
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
}

/// Structure controlling the behavior of [`PhysicalMemoryManager::allocate_frames()`] and
/// [`PhysicalMemoryManager::allocate_frames_aligned()`].
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocationPolicy {
    /// Any frame region is suitable for allocation.
    #[default]
    Any,
    /// Only the frame region with the starting physical address that matches the associated
    /// physical address is valid for allocated.
    At(u64),
    /// The entire frame region must be within the region bounded by the provided inclusive maximum
    /// address.
    InclusiveMax(u64),
}

impl fmt::Debug for AllocationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.pad("Any"),
            Self::At(value) => write!(f, "At({value:#x})"),
            Self::InclusiveMax(value) => write!(f, "InclusiveMax({value:#x})"),
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

/// Description of a single memory region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryDescriptor {
    /// The [`PhysicalAddressRange`] described by this [`MemoryDescriptor`].
    pub range: PhysicalAddressRange,
    /// The type of the memory region.
    pub region_type: MemoryType,
}

impl MemoryDescriptor {
    /// The version of the [`MemoryDescriptor`] with which this [`MemoryDescriptor`] is associated.
    pub const VERSION: u64 = 0;
}

/// Various types of memory regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryType {
    /// Memory that is usable RAM (i.e., that doesn't contain any active code or relevant
    /// information).
    Free,
    /// Memory that is used to store parts of the bootloader, firmware, or the executable.
    ///
    /// This memory can be reclaimed as soon as the executable is no longer utilizing the memory.
    BootloaderReclaimable,
    /// Memory in which errors have been detected.
    Bad,
    /// Memory that is utilzed for unknown purposes by the hardware, firmware, or other entities and
    /// thus should not be utilized by the executable.
    Reserved,
    /// Memory that holds ACPI tables.
    AcpiReclaimable,
    /// Memory that holds non-volatile ACPI data.
    AcpiNonVolatile,
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

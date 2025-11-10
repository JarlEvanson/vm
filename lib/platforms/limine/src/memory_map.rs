//! Definitions of [`MemoryMapRequest`] and [`MemoryMapResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`MemoryMapRequest`].
pub const MEMORY_MAP_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x67cf3d9d378a806f,
    0xe304acdfc50c3c62,
];

/// Request for the machine's physical memory map.
#[repr(C)]
#[derive(Debug)]
pub struct MemoryMapRequest {
    /// Location storing [`MEMORY_MAP_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`MemoryMapRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`MemoryMapResponse`] structure for this [`MemoryMapRequest`].
    pub response: *mut MemoryMapResponse,
}

// SAFETY:
//
// [`MemoryMapRequest`] does not interact with threads in any manner.
unsafe impl Send for MemoryMapRequest {}
// SAFETY:
//
// [`MemoryMapRequest`] does not interact with threads in any manner.
unsafe impl Sync for MemoryMapRequest {}

/// Response to a [`MemoryMapRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct MemoryMapResponse {
    /// The revision of the [`MemoryMapResponse`] structure.
    pub revision: u64,
    /// How many [`MemoryMapEntry`]s are present.
    pub entry_count: u64,
    /// A pointer to an array of [`MemoryMapResponse::entry_count`] pointers to
    /// memory map entry structures.
    pub entries: *mut *mut MemoryMapEntry,
}

// SAFETY:
//
// [`MemoryMapResponse`] does not interact with threads in any manner.
unsafe impl Send for MemoryMapResponse {}
// SAFETY:
//
// [`MemoryMapResponse`] does not interact with threads in any manner.
unsafe impl Sync for MemoryMapResponse {}

/// Description of a region of physical address space.
#[repr(C)]
#[derive(Debug)]
pub struct MemoryMapEntry {
    /// The physical address at the start of the region.
    pub base: u64,
    /// The size of the region in bytes.
    pub length: u64,
    /// The type/usage of the region.
    pub mem_type: MemoryType,
}

/// Various memory types.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryType(pub u64);

impl MemoryType {
    /// Memory available for general purpose usage.
    pub const USABLE: Self = Self(0);
    /// Memory that was reserved through some mechanism.
    pub const RESERVED: Self = Self(1);
    /// Memory that can be reclaimed after ACPI is initialized.
    pub const ACPI_RECLAIMABLE: Self = Self(2);
    /// Memory that stores non-volatile data for ACPI.
    pub const ACPI_NVS: Self = Self(3);
    /// Memory in which error have been detected.
    pub const BAD_MEMORY: Self = Self(4);
    /// Memory in which bootloader structures have been allocated.
    ///
    /// This memory can be recovered after all bootloader structures have finished being used.
    pub const BOOTLOADER_RECLAIMABLE: Self = Self(5);
    /// Memory that contains the loaded executable and modules.
    pub const EXECUTABLE_AND_MODULES: Self = Self(6);
    /// Memory that contains the framebuffer.
    pub const FRAMEBUFFER: Self = Self(7);
}

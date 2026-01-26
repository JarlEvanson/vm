//! Definitions of [`EfiMemoryMapRequest`] and [`EfiMemoryMapResponse`].

use core::ffi::c_void;

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`EfiMemoryMapRequest`].
pub const EFI_MEMORY_MAP_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0xc5e77b6b397e7b43,
    0x27637845accdcf3c,
];

/// Request for the address of the EFI memory map.
#[repr(C)]
#[derive(Debug)]
pub struct EfiMemoryMapRequest {
    /// Location storing [`EFI_MEMORY_MAP_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`EfiMemoryMapRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`EfiMemoryMapResponse`] structure for this [`EfiMemoryMapRequest`].
    pub response: *mut EfiMemoryMapResponse,
}

// SAFETY:
//
// [`EfiMemoryMapRequest`] does not interact with threads in any manner.
unsafe impl Send for EfiMemoryMapRequest {}
// SAFETY:
//
// [`EfiMemoryMapRequest`] does not interact with threads in any manner.
unsafe impl Sync for EfiMemoryMapRequest {}

/// Response to an [`EfiMemoryMapRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct EfiMemoryMapResponse {
    /// The revision of the [`EfiMemoryMapResponse`] structure.
    pub revision: u64,
    /// The address of the EFI memory map.
    ///
    /// The address is in the HHDM in bootloader reclaimable memory.
    pub mem_map: *mut c_void,

    /// The size, in bytes, of the EFI memory map.
    pub mem_map_size: u64,
    /// The size, in bytes, of a single EFI memory map descriptor.
    pub desc_size: u64,
    /// The version of the EFI memory map descriptors.
    pub desc_version: u64,
}

// SAFETY:
//
// [`EfiMemoryMapResponse`] does not interact with threads in any manner.
unsafe impl Send for EfiMemoryMapResponse {}
// SAFETY:
//
// [`EfiMemoryMapResponse`] does not interact with threads in any manner.
unsafe impl Sync for EfiMemoryMapResponse {}

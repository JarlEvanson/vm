//! Definitions of [`DeviceTreeRequest`] and [`DeviceTreeResponse`].

use core::ffi::c_void;

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`DeviceTreeRequest`].
pub const DEVICE_TREE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x502746e184c088aa,
    0xfbc5ec83e6327893,
];

/// Request for the device tree blob.
#[repr(C)]
#[derive(Debug)]
pub struct DeviceTreeRequest {
    /// Location storing [`DEVICE_TREE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`DeviceTreeRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`DeviceTreeResponse`] structure for this [`DeviceTreeRequest`].
    pub response: *mut DeviceTreeResponse,
}

// SAFETY:
//
// [`DeviceTreeRequest`] does not interact with threads in any manner.
unsafe impl Send for DeviceTreeRequest {}
// SAFETY:
//
// [`DeviceTreeRequest`] does not interact with threads in any manner.
unsafe impl Sync for DeviceTreeRequest {}

/// Response to an [`DeviceTreeRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct DeviceTreeResponse {
    /// The revision of the [`DeviceTreeResponse`] structure.
    pub revision: u64,
    /// A pointer to the device tree blob, in bootloader reclaimable memory.
    pub dtb_ptr: *mut c_void,
}

// SAFETY:
//
// [`DeviceTreeResponse`] does not interact with threads in any manner.
unsafe impl Send for DeviceTreeResponse {}
// SAFETY:
//
// [`DeviceTreeResponse`] does not interact with threads in any manner.
unsafe impl Sync for DeviceTreeResponse {}

//! Definitions of the [`BootloaderInfoRequest`] and [`BootloaderInfoResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`BootloaderInfoRequest`].
pub const BOOTLOADER_INFO_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0xf55038d8e2a1202f,
    0x279426fcf5f59740,
];

/// Request for the bootloader's name and version.
#[repr(C)]
#[derive(Debug)]
pub struct BootloaderInfoRequest {
    /// Location storing [`BOOTLOADER_INFO_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`BootloaderInfoRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`BootloaderInfoResponse`] structure for this [`BootloaderInfoRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`BootloaderInfoRequest`] does not interact with threads in any manner.
unsafe impl Send for BootloaderInfoRequest {}
// SAFETY:
//
// [`BootloaderInfoRequest`] does not interact with threads in any manner.
unsafe impl Sync for BootloaderInfoRequest {}

/// Response to a [`BootloaderInfoRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct BootloaderInfoResponse {
    /// The revision of the [`BootloaderInfoResponse`] structure.
    pub revision: u64,
    /// A 0-terminated ASCII string containing the name of the loading bootloader.
    pub name: u64,
    /// A 0-terminated ASCII string containing the version of the loading bootloader.
    pub version: u64,
}

// SAFETY:
//
// [`BootloaderInfoResponse`] does not interact with threads in any manner.
unsafe impl Send for BootloaderInfoResponse {}
// SAFETY:
//
// [`BootloaderInfoResponse`] does not interact with threads in any manner.
unsafe impl Sync for BootloaderInfoResponse {}

//! Definitions of [`SmbiosRequest`] and [`SmbiosResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`SmbiosRequest`].
pub const SMBIOS_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x9e9046f11e095391,
    0xaa4a520fefbde5ee,
];

/// Request for the location of the address of the SMBIOS table.
#[repr(C)]
#[derive(Debug)]
pub struct SmbiosRequest {
    /// Location storing [`SMBIOS_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`SmbiosRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`SmbiosResponse`] structure for this [`SmbiosRequest`].
    pub response: *mut SmbiosResponse,
}

// SAFETY:
//
// [`SmbiosRequest`] does not interact with threads in any manner.
unsafe impl Send for SmbiosRequest {}
// SAFETY:
//
// [`SmbiosRequest`] does not interact with threads in any manner.
unsafe impl Sync for SmbiosRequest {}

/// Response to an [`SmbiosRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct SmbiosResponse {
    /// The revision of the [`SmbiosResponse`] structure.
    pub revision: u64,
    /// The address of the 32-bit SMBIOS entry point.
    ///
    /// Physical for base revision >= 3.
    pub entry_32: u64,
    /// The address of the 64-bit SMBIOS entry point.
    ///
    /// Physical for base revision >= 3.
    pub entry_64: u64,
}

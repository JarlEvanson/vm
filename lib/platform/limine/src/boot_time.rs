//! Definitions of [`BootTimeRequest`] and [`BootTimeResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`BootTimeRequest`].
pub const BOOT_TIME_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x502746e184c088aa,
    0xfbc5ec83e6327893,
];

/// Request for the UNIX time on boot, in seconds.
#[repr(C)]
#[derive(Debug)]
pub struct BootTimeRequest {
    /// Location storing [`BOOT_TIME_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`BootTimeRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`BootTimeResponse`] structure for this [`BootTimeRequest`].
    pub response: *mut BootTimeResponse,
}

// SAFETY:
//
// [`BootTimeRequest`] does not interact with threads in any manner.
unsafe impl Send for BootTimeRequest {}
// SAFETY:
//
// [`BootTimeRequest`] does not interact with threads in any manner.
unsafe impl Sync for BootTimeRequest {}

/// Response to an [`BootTimeRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct BootTimeResponse {
    /// The revision of the [`BootTimeResponse`] structure.
    pub revision: u64,
    /// The UNIX time on boot, in seconds, as taken from the system RTC.
    pub boot_time: i64,
}

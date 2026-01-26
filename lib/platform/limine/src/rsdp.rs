//! Definitions of [`RsdpRequest`] and [`RsdpResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`RsdpRequest`].
pub const RSDP_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0xc5e77b6b397e7b43,
    0x27637845accdcf3c,
];

/// Request for the address of the RSDP table.
#[repr(C)]
#[derive(Debug)]
pub struct RsdpRequest {
    /// Location storing [`RSDP_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`RsdpRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`RsdpResponse`] structure for this [`RsdpRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`RsdpRequest`] does not interact with threads in any manner.
unsafe impl Send for RsdpRequest {}
// SAFETY:
//
// [`RsdpRequest`] does not interact with threads in any manner.
unsafe impl Sync for RsdpRequest {}

/// Response to an [`RsdpRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct RsdpResponse {
    /// The revision of the [`RsdpResponse`] structure.
    pub revision: u64,
    /// The address of the RSDP table.
    ///
    /// Physical for base revision == 3, virtual for all other revisions.
    pub address: u64,
}

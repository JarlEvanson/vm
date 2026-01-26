//! Definitions of [`HhdmRequest`] and [`HhdmResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`HhdmRequest`].
pub const HHDM_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x48dcf1cb8ad2b852,
    0x63984e959a98244b,
];

/// Request the offset of the higher half memory map.
#[repr(C)]
#[derive(Debug)]
pub struct HhdmRequest {
    /// Location storing [`HHDM_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`HhdmRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`HhdmResponse`] structure for this [`HhdmRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`HhdmRequest`] does not interact with threads in any manner.
unsafe impl Send for HhdmRequest {}
// SAFETY:
//
// [`HhdmRequest`] does not interact with threads in any manner.
unsafe impl Sync for HhdmRequest {}

/// Response to a [`HhdmRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct HhdmResponse {
    /// The revision of the [`HhdmResponse`] structure.
    pub revision: u64,
    /// The virtual address offset of the beginning of the higher half direct map.
    pub offset: u64,
}

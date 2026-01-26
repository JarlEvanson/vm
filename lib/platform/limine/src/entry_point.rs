//! Definitions of [`EntryPointRequest`] and [`EntryPointResponse`].

use core::ffi::c_void;

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`EntryPointRequest`].
pub const ENTRY_POINT_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x13d86c035a1cd3e1,
    0x2b0caa89d8f3026a,
];

/// Request specifying the intended entry point when the executable is loaded according to the
/// Limine boot protocol.
#[repr(C)]
#[derive(Debug)]
pub struct EntryPointRequest {
    /// Location storing [`ENTRY_POINT_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`EntryPointRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`EntryPointResponse`] structure for this [`EntryPointRequest`].
    pub response: *mut EntryPointResponse,
    /// The requested entry point.
    pub entry_point: *mut c_void,
}

// SAFETY:
//
// [`EntryPointRequest`] does not interact with threads in any manner.
unsafe impl Send for EntryPointRequest {}
// SAFETY:
//
// [`EntryPointRequest`] does not interact with threads in any manner.
unsafe impl Sync for EntryPointRequest {}

/// Response to an [`EntryPointRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct EntryPointResponse {
    /// The revision of the [`EntryPointResponse`] structure.
    pub revision: u64,
}

//! Definitions of [`ExecutableCmdLineRequest`] and [`ExecutableCmdLineResponse`].

use core::ffi::c_char;

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`ExecutableCmdLineRequest`].
pub const EXECUTABLE_CMD_LINE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x4b161536e598651e,
    0xb390ad4a2f1f303a,
];

/// Request for the ASCII 0-terminated string containing the command line associated with the loaded
/// executable file.
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableCmdLineRequest {
    /// Location storing [`EXECUTABLE_CMD_LINE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`ExecutableCmdLineRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`ExecutableCmdLineResponse`] structure for this [`ExecutableCmdLineRequest`].
    pub response: *mut ExecutableCmdLineResponse,
}

// SAFETY:
//
// [`ExecutableCmdLineRequest`] does not interact with threads in any manner.
unsafe impl Send for ExecutableCmdLineRequest {}
// SAFETY:
//
// [`ExecutableCmdLineRequest`] does not interact with threads in any manner.
unsafe impl Sync for ExecutableCmdLineRequest {}

/// Response to an [`ExecutableCmdLineRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableCmdLineResponse {
    /// The revision of the [`ExecutableCmdLineRequest`] structure.
    pub revision: u64,
    /// The ASCII 0-terminated string containing the command line associated with the loaded
    /// executable file.
    pub cmd_line: *mut c_char,
}

// SAFETY:
//
// [`ExecutableCmdLineResponse`] does not interact with threads in any manner.
unsafe impl Send for ExecutableCmdLineResponse {}
// SAFETY:
//
// [`ExecutableCmdLineResponse`] does not interact with threads in any manner.
unsafe impl Sync for ExecutableCmdLineResponse {}

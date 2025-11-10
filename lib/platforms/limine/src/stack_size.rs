//! Definitions of [`StackSizeRequest`] and [`StackSizeResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`StackSizeRequest`].
pub const STACK_SIZE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x224ef0460a8e8926,
    0xe1cb0fc25f46ea3d,
];

/// Request specifying the desired stack size in bytes.
#[repr(C)]
#[derive(Debug)]
pub struct StackSizeRequest {
    /// Location storing [`STACK_SIZE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`StackSizeRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`StackSizeResponse`] structure for this [`StackSizeRequest`].
    pub response: *mut StackSizeResponse,
    /// The requested stack size in bytes.
    ///
    /// This size is also used for other processors if enabled.
    pub stack_size: u64,
}

// SAFETY:
//
// [`StackSizeRequest`] does not interact with threads in any manner.
unsafe impl Send for StackSizeRequest {}
// SAFETY:
//
// [`StackSizeRequest`] does not interact with threads in any manner.
unsafe impl Sync for StackSizeRequest {}

/// Response to a [`StackSizeRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct StackSizeResponse {
    /// The revision of the [`StackSizeResponse`] structure.
    pub revision: u64,
}

//! Definitions of [`ExecutableAddressRequest`] and [`ExecutableAddressResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`ExecutableAddressRequest`].
pub const EXECUTABLE_ADDRESS_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x71ba76863cc55f63,
    0xb2644a48c516a487,
];

/// Request for the physical and virtual address of the executable.
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableAddressRequest {
    /// Location storing [`EXECUTABLE_ADDRESS_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`ExecutableAddressRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`ExecutableAddressResponse`] structure for this [`ExecutableAddressRequest`].
    pub response: *mut ExecutableAddressResponse,
}

// SAFETY:
//
// [`ExecutableAddressRequest`] does not interact with threads in any manner.
unsafe impl Send for ExecutableAddressRequest {}
// SAFETY:
//
// [`ExecutableAddressRequest`] does not interact with threads in any manner.
unsafe impl Sync for ExecutableAddressRequest {}

/// Response to an [`ExecutableAddressRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableAddressResponse {
    /// The revision of the [`ExecutableAddressResponse`] structure.
    pub revision: u64,
    /// The physical base address of the executable.
    pub physical_base: u64,
    /// The virtual base address of the executable.
    pub virtual_base: u64,
}

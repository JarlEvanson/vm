//! Definitions of [`EfiSystemTableRequest`] and [`EfiSystemTableResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`EfiSystemTableRequest`].
pub const EFI_SYSTEM_TABLE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x5ceba5163eaaf6d6,
    0x0a6981610cf65fcc,
];

/// Request for the address of the EFI system table.
#[repr(C)]
#[derive(Debug)]
pub struct EfiSystemTableRequest {
    /// Location storing [`EFI_SYSTEM_TABLE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`EfiSystemTableRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`EfiSystemTableResponse`] structure for this [`EfiSystemTableRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`EfiSystemTableRequest`] does not interact with threads in any manner.
unsafe impl Send for EfiSystemTableRequest {}
// SAFETY:
//
// [`EfiSystemTableRequest`] does not interact with threads in any manner.
unsafe impl Sync for EfiSystemTableRequest {}

/// Response to an [`EfiSystemTableRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct EfiSystemTableResponse {
    /// The revision of the [`EfiSystemTableResponse`] structure.
    pub revision: u64,
    /// The address of the EFI system table.
    ///
    /// Physical for base revision >= 3.
    pub address: u64,
}

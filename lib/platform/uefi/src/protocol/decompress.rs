//! Definitions of the UEFI Device Path Protocol and associated items.

use core::ffi;

use crate::{
    data_type::{Guid, Status},
    guid,
};

/// Provides a decompression service.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct DecompressProtocol {
    /// Gets the size of the uncompressed and scratch buffers.
    pub get_info: GetInfo,
    /// Decompresses a compressed source buffer.
    pub decompress: Decompress,
}

impl DecompressProtocol {
    /// The [`Guid`] associated with the [`DecompressProtocol`].
    pub const GUID: Guid = guid!("d8117cfe-94a6-11d4-9a3a-0090273fc14d");
}

/// Returns the size of the uncompressed buffer and the size of the scratch buffer required to
/// decompress the compressed source buffer.
pub type GetInfo = unsafe extern "efiapi" fn(
    this: *mut DecompressProtocol,
    source: *const ffi::c_void,
    source_size: u32,
    destination_size: *mut u32,
    scratch_size: *mut u32,
) -> Status;

/// Decompresses a compressed source buffer.
pub type Decompress = unsafe extern "efiapi" fn(
    this: *mut DecompressProtocol,
    source: *const ffi::c_void,
    source_size: u32,
    destination: *mut ffi::c_void,
    destination_size: u32,
    scratch: *mut ffi::c_void,
    scratch_size: u32,
) -> Status;

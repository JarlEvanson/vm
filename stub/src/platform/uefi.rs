//! Support for booting from an UEFI platform implementation.

use uefi::{
    data_type::{Handle, Status},
    table::system::SystemTable,
};

/// Rust entrypoint for the UEFI environment.
pub extern "efiapi" fn uefi_main(
    _image_handle: Handle,
    system_table_ptr: *mut SystemTable,
) -> Status {
    Status::SUCCESS
}

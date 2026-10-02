//! Support for booting from an UEFI platform implementation.

use core::{
    ffi, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use uefi::{
    data_type::{Handle, Status},
    table::system::SystemTable,
};

mod console;

/// The [`Handle`] representing the image.
static IMAGE_HANDLE: AtomicPtr<ffi::c_void> = AtomicPtr::new(ptr::null_mut());
/// The program's UEFI table.
static UEFI_SYSTEM_TABLE: AtomicPtr<SystemTable> = AtomicPtr::new(ptr::null_mut());

/// Rust entrypoint for the UEFI environment.
pub extern "efiapi" fn main(image_handle: Handle, system_table_ptr: *mut SystemTable) -> Status {
    IMAGE_HANDLE.store(image_handle.0, Ordering::Release);
    UEFI_SYSTEM_TABLE.store(system_table_ptr, Ordering::Release);
    console::register();

    Status::SUCCESS
}

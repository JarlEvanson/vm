//! Support for booting from an UEFI platform implementation.

use core::{
    ffi, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use uefi::{
    data_type::{Handle, Status},
    table::system::SystemTable,
};

use crate::platform::uefi::cmd_line::acquire_cmdline;

mod cmd_line;
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

    // SAFETY:
    //
    // `system_table_ptr` was provided by the `efi_main` entry point and so according to the UEFI
    // specification, the pointer must be valid.
    let boot_services_ptr = unsafe { (*system_table_ptr).boot_services };

    // SAFETY:
    //
    // `system_table_ptr` was provided by the `efi_main` entry point and so according to the UEFI
    // specification, it must contain a valid UEFI [`BootServices`] table.
    let boot_services_revision = unsafe { (*boot_services_ptr).header.revision };

    // SAFETY:
    //
    // - `image_handle` was passed to this function by UEFI firmware and thus fulfills the
    //   invariants of [`acquire_cmdline()`].
    // - `boot_services_ptr` was derived from the system_table_ptr passed to this function by UEFI
    //   firmware and thus fulfills the invariants of [`acquire_cmdline`].
    let cmdline = unsafe { acquire_cmdline(image_handle, boot_services_ptr) };
    let cmdline = match cmdline {
        Ok(cmdline) => cmdline,
        Err(error) => {
            crate::error!("error while acquiring image command line: {error}");
            return Status::UNSUPPORTED;
        }
    };

    crate::debug!("{}", cmdline.as_str());

    Status::SUCCESS
}

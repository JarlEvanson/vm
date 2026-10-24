//! Support for booting from an UEFI platform implementation.

use core::{
    ffi, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use uefi::{
    data_type::{Handle, Status},
    table::system::SystemTable,
};

use crate::{
    define_node,
    graph::ActivationSuccess,
    platform::{self, BootProtocol},
};

/// The [`Handle`] representing the image.
static IMAGE_HANDLE: AtomicPtr<ffi::c_void> = AtomicPtr::new(ptr::null_mut());
/// The program's UEFI table.
static UEFI_SYSTEM_TABLE: AtomicPtr<SystemTable> = AtomicPtr::new(ptr::null_mut());

/// Rust entrypoint for the UEFI environment.
pub extern "efiapi" fn main(image_handle: Handle, system_table_ptr: *mut SystemTable) -> Status {
    IMAGE_HANDLE.store(image_handle.0, Ordering::Release);
    UEFI_SYSTEM_TABLE.store(system_table_ptr, Ordering::Release);

    // SAFETY:
    //
    // This executable was booted from UEFI.
    unsafe { platform::set_protocol(BootProtocol::Uefi) }

    Status::SUCCESS
}

define_node!(
    CHECK_UEFI,
    "Check UEFI",
    type = (),
    activation = check_uefi,
    enabled = true,
    extended_lifetime = false,
    required_enabled = [],
    required_disabled = [],
    required_by_enabled = [],
    required_by_disabled = [],
);

/// Acts as a general safeguard against UEFI-specific [`GraphNode`][crate::graph::GraphNode`]s being
/// activated when the [`BootProtocol`] does not match [`BootProtocol::Uefi`].
fn check_uefi() -> Result<ActivationSuccess<()>, ()> {
    if platform::get_protocol() != BootProtocol::Uefi {
        return Ok(ActivationSuccess::Disable);
    }

    Ok(ActivationSuccess::Activate(()))
}

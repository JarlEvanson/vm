//! Definitions of the UEFI Loaded Image Protocol and associated items.

use core::ffi;

use crate::{
    data_type::{Guid, Handle, Status},
    guid,
    memory::MemoryType,
    protocol::device_path::DevicePathProtocol,
    table::system::SystemTable,
};

/// Protocol providing information about the loaded image.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct LoadedImageProtocol {
    /// The revision of the [`LoadedImageProtocol`] structure.
    ///
    /// All revision will be backward compatible to the current revision.
    pub revision: u32,
    /// The parent image's image [`Handle`].
    pub parent_handle: Handle,
    /// The image's UEFI [`SystemTable`] pointer.
    pub system_table: *mut SystemTable,

    /// The device [`Handle`] that the UEFI image was loaded from.
    pub device_handle: Handle,
    /// A pointer to the file path portion specific to the [`LoadedImageProtocol::device_handle`]
    /// that the UEFI image was loaded from.
    pub file_path: *mut DevicePathProtocol,
    /// Reserved.
    pub _reserved: *mut ffi::c_void,

    /// The size, in bytes, of [`LoadedImageProtocol::load_options`].
    pub load_options_size: u32,
    /// A pointer to the image's binary load options.
    pub load_options: *mut ffi::c_void,

    /// The base address at which the image was loaded.
    pub image_base: *mut ffi::c_void,
    /// The size, in bytes, of the loaded image.
    pub image_size: u64,
    /// The [`MemoryType`] that the code sections were loaded as.
    pub image_code_type: MemoryType,
    /// The [`MemoryType`] that the data sections were loaded as.
    pub image_data_type: MemoryType,
    /// Function that unloads the image.
    pub unload: ImageUnload,
}

impl LoadedImageProtocol {
    /// The [`Guid`] identifying this protocol.
    pub const GUID: Guid = guid!("5b1b31a1-9562-11d2-8e3f-00a0c969723b");
}

/// A callback that a driver registers to do cleanup when the [`BootServices::unload_image`][bsui]
/// function is called.
///
/// [bsui]: crate::table::boot::BootServices1_0::unload_image
pub type ImageUnload = unsafe extern "efiapi" fn(image_handle: Handle) -> Status;

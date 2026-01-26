//! Definitions of the UEFI Device Path Utilities Protocol and associated items.

use crate::{
    data_type::{Boolean, Guid},
    guid,
    protocol::device_path::DevicePathProtocol,
};

/// Provides various utility functions that aid in creating and manipulating device paths.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DevicePathUtilitiesProtocol {
    /// Returns the size of the specified device path in bytes.
    get_device_path_size: GetDevicePathSize,
    /// Duplicates a device path structure.
    duplicate_device_path: DuplicateDevicePath,
    /// Appends a device path to the specified device path.
    append_device_path: AppendDevicePath,
    /// Appends a device path to the specified device path node.
    append_device_path_node: AppendDevicePath,
    /// Appends a device path instance to the specified device path.
    append_device_path_instance: AppendDevicePathInstance,
    /// Retrieves the next device path instance from the device path data structure.
    get_next_device_path_instance: GetNextDevicePathInstance,
    /// Returns [`Boolean::TRUE`] if this is a multi-instance device path.
    is_device_path_multi_instance: IsDevicePathMultiInstance,
    /// Allocates memory for a node with the specified type and subtype.
    create_device_path_node: CreateDevicePathNode,
}

impl DevicePathUtilitiesProtocol {
    /// The [`Guid`] associated with the [`DevicePathUtilitiesProtocol`].
    pub const GUID: Guid = guid!("0379be4e-d706-437d-0b37-edb82fb772a4");
}

/// Returns the size of the specified device path in bytes, including the end-of-path tag.
///
/// If `this` is NULL, then zero is returned.
pub type GetDevicePathSize =
    unsafe extern "efiapi" fn(device_path: *const DevicePathProtocol) -> usize;

/// Creates a duplicate of the specified device path.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// Returns NULL if `device_path` is NULL or if there was insufficient memory.
pub type DuplicateDevicePath =
    unsafe extern "efiapi" fn(device_path: *const DevicePathProtocol) -> *mut DevicePathProtocol;

/// Creates a new path by appending the second device path to the first.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// If both of the sources are NULL, an end-of-path node is returned. If one of the sources is
/// NULL, the other is returned. If there was insufficient memory, NULL is returned.
pub type AppendDevicePath = unsafe extern "efiapi" fn(
    src_1: *const DevicePathProtocol,
    src_2: *const DevicePathProtocol,
) -> *mut DevicePathProtocol;

/// Creates a new path by appending the device node to the device path.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// If one of the inputs is NULL, then it is simply not added to the new device path.
pub type AppendDevicePathNode = unsafe extern "efiapi" fn(
    device_path: *const DevicePathProtocol,
    device_node: *const DevicePathProtocol,
) -> *mut DevicePathProtocol;

/// Creates a new path by appending the device path instance to the device path.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// If one of the inputs is NULL, then it is simply not added to the new device path.
pub type AppendDevicePathInstance = unsafe extern "efiapi" fn(
    device_path: *const DevicePathProtocol,
    device_path_instance: *const DevicePathProtocol,
) -> *mut DevicePathProtocol;

/// Creates a copy of the current device path instance and returns a pointer to the next device
/// path instance.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// Updates `device_path_instance_size` with the size of the returned [`DevicePathProtocol`]
/// instance and `device_path_instance` with the next device path instance or NULL if there are no
/// more.
pub type GetNextDevicePathInstance = unsafe extern "efiapi" fn(
    device_path_instance: *mut *const DevicePathProtocol,
    device_path_instance_size: *mut usize,
) -> *mut DevicePathProtocol;

/// Returns whether the device path is multi-instance.
///
/// If `device_path` is NULL, then [`Boolean::FALSE`].
pub type IsDevicePathMultiInstance =
    unsafe extern "efiapi" fn(device_path: *const DevicePathProtocol) -> Boolean;

/// Creates a device node.
///
/// The memory is allocated from EFI Boot Services and is the responsibility of the caller to free.
///
/// Returns NULL if `node_length` is less than the size of the header or if there was insufficient
/// memory.
pub type CreateDevicePathNode = unsafe extern "efiapi" fn(
    node_type: u8,
    node_subtype: u8,
    node_length: u16,
) -> *mut DevicePathProtocol;

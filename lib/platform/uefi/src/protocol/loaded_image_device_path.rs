//! Definitions of the UEFI Device Path Protocol and associated items.

use crate::{data_type::Guid, guid, protocol::device_path::DevicePathProtocol};

/// A programmatic path to the device path used when a PE/COFF file is loaded.
#[repr(transparent)]
pub struct LoadedImageDevicePathProtocol(DevicePathProtocol);

impl LoadedImageDevicePathProtocol {
    /// The [`Guid`] identifying this protocol.
    pub const GUID: Guid = guid!("bc62157e-3e33-4fec-9920-2d3b36d750df");
}

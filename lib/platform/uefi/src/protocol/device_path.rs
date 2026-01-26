//! Definitions of the UEFI Device Path Protocol and associated items.

use crate::{data_type::Guid, guid};

/// A programmatic path to a device.
#[repr(C)]
pub struct DevicePathProtocol {
    /// The major type of a device path node.
    pub node_type: u8,
    /// The subtype of a device path node.
    pub node_subtype: u8,
    /// The length of the extended device path node.
    pub length: [u8; 2],
}

impl DevicePathProtocol {
    /// The [`Guid`] identifying this protocol.
    pub const GUID: Guid = guid!("09576e91-6d3f-11d2-8e39-00a0c969723b");
}

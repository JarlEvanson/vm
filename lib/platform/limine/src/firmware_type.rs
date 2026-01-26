//! Definitions of the [`FirmwareTypeRequest`] and [`FirmwareTypeResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`FirmwareTypeRequest`].
pub const FIRMWARE_TYPE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x8c2f75d90bef28a8,
    0x7045a4688eac00c3,
];

/// Request for the firmware the bootloader loaded from.
#[repr(C)]
#[derive(Debug)]
pub struct FirmwareTypeRequest {
    /// Location storing [`FIRMWARE_TYPE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`FirmwareTypeRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`FirmwareTypeResponse`] structure for this [`FirmwareTypeRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`FirmwareTypeRequest`] does not interact with threads in any manner.
unsafe impl Send for FirmwareTypeRequest {}
// SAFETY:
//
// [`FirmwareTypeRequest`] does not interact with threads in any manner.
unsafe impl Sync for FirmwareTypeRequest {}

/// Response to a [`FirmwareTypeRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct FirmwareTypeResponse {
    /// The revision of the [`FirmwareTypeResponse`] structure.
    pub revision: u64,
    /// The type of the firmware.
    pub firmware_type: FirmwareType,
}

/// Various types of a firmware interface.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FirmwareType(pub u64);

impl FirmwareType {
    /// The firmware is an `x86` BIOS.
    pub const X86_BIOS: Self = Self(0);
    /// The firmware is a 32-bit UEFI implementation.
    pub const UEFI_32: Self = Self(1);
    /// The firmware is a 64-bit UEFI implementation.
    pub const UEFI_64: Self = Self(2);
    /// The firmware is a SBI implementation.
    pub const SBI: Self = Self(3);
}

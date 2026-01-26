//! Definitions related to UEFI configuration tables.

use core::ffi::c_void;

use crate::{data_type::Guid, guid};

/// [`ConfigurationTable::vendor_guid`] of the ACPI table.
pub const ACPI: Guid = guid!("eb9d2d30-2d88-11d3-9a16-0090273fc14d");

/// [`ConfigurationTable::vendor_guid`] of the ACPI 2.0 table.
pub const ACPI_2: Guid = guid!("8868e871-e4f1-11d3-bc22-0080c73c8881");

/// [`ConfigurationTable::vendor_guid`] of the device tree blob table.
pub const DEVICE_TREE: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

/// [`ConfigurationTable::vendor_guid`] of the SMBIOS table.
pub const SMBIOS: Guid = guid!("eb9d2d31-2d88-11d3-9a16-0090273fc14d");
/// [`ConfigurationTable::vendor_guid`] of the SMBIOS 3 table.
pub const SMBIOS_3: Guid = guid!("f2fd1544-9794-4a2c-992e-e5bbcf20e394");

/// Contains a [`Guid`] and pointer pair that identifies the table and its location.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigurationTable {
    /// The [`Guid`] that uniquely identifies the system [`ConfigurationTable`].
    pub vendor_guid: Guid,
    /// A pointer to the table associated with [`ConfigurationTable::vendor_guid`].
    ///
    /// May be either a physical address or a virtual address, depending on the
    /// [`ConfigurationTable::vendor_guid`].
    pub vendor_table: *mut c_void,
}

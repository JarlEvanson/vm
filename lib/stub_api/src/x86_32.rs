//! The specification of the `x86_32` specific table.

pub use X86_32TableV0 as X86_32Table;

/// Table providing information and functionality that is specific to `x86_32`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct X86_32TableV0 {
    /// The version of the [`X86_32Table`] with which this table identifies.
    pub version: u64,

    /// The physical address of the UEFI system table.
    pub uefi_system_table: u64,

    /// The physical address of the RSDP structure.
    pub rsdp: u64,
    /// The physical address of the XSDP structure.
    pub xsdp: u64,
    /// The physical address of the start of the device tree.
    pub device_tree: u64,
    /// The physical address of the 32-bit SMBIOS entry point.
    pub smbios_32: u64,
    /// The physical address of the 64-bit SMBIOS entry point.
    pub smbios_64: u64,
}

impl X86_32TableV0 {
    /// The version of the [`X86_32Table`] with which this [`X86_32Table`] is associated.
    pub const VERSION: u64 = 0;
}

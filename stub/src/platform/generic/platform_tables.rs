//! Definitions and interfaces that platforms use to provide information related to
//! platform/firmware tables in a platform agnostic manner.

use sync::ControlledModificationCell;

use crate::platform::PhysicalAddress;

/// Centralized representation of all platform/firmware tables.
static PLATFORM_TABLES: PlatformTables = PlatformTables::new();

/// Sets the platform's UEFI System Table location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_uefi_system_table()`] or [`uefi_system_table()`].
pub unsafe fn set_uefi_system_table(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_uefi_system_table()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.uefi_system_table.get_mut() = Some(address) }
}

/// Returns the platform's UEFI System Table location parameter.
pub fn uefi_system_table() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.uefi_system_table.get()
}

/// Sets the platform's ACPI RSDP location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_rsdp()`] or [`rsdp()`].
pub unsafe fn set_rsdp(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_rsdp()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.rsdp.get_mut() = Some(address) }
}

/// Returns the platform's ACPI RSDP location parameter.
pub fn rsdp() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.rsdp.get()
}

/// Sets the platform's ACPI XSDP location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_xsdp()`] or [`xsdp()`].
pub unsafe fn set_xsdp(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_xsdp()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.xsdp.get_mut() = Some(address) }
}

/// Returns the platform's ACPI XSDP location parameter.
pub fn xsdp() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.xsdp.get()
}

/// Sets the platform's Flattened Device Tree location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_device_tree()`] or [`device_tree()`].
pub unsafe fn set_device_tree(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_device_tree()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.device_tree.get_mut() = Some(address) }
}

/// Returns the platform's Flattened Device Tree location parameter.
pub fn device_tree() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.device_tree.get()
}

/// Sets the platform's SMBIOS 32 location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_smbios_32()`] or [`smbios_32()`].
pub unsafe fn set_smbios_32(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_smbios_32()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.smbios_32.get_mut() = Some(address) }
}

/// Returns the platform's SMBIOS 32 location parameter.
pub fn smbios_32() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.smbios_32.get()
}

/// Sets the platform's SMBIOS 64 location parameter.
///
/// # Safety
///
/// There must be zero overlapping calls to [`set_smbios_64()`] or [`smbios_64()`].
pub unsafe fn set_smbios_64(address: PhysicalAddress) {
    // SAFETY:
    //
    // The invariants of `set_smbios_64()` ensure that this operation is safe.
    unsafe { *PLATFORM_TABLES.smbios_64.get_mut() = Some(address) }
}

/// Returns the platform's SMBIOS 64 location parameter.
pub fn smbios_64() -> Option<PhysicalAddress> {
    *PLATFORM_TABLES.smbios_64.get()
}

/// Collection of various platform/firmware tables.
struct PlatformTables {
    /// The platform's UEFI System Table location paramter.
    uefi_system_table: ControlledModificationCell<Option<PhysicalAddress>>,
    /// The platform's ACPI RSDP location paramter.
    rsdp: ControlledModificationCell<Option<PhysicalAddress>>,
    /// The platform's ACPI XSDP location paramter.
    xsdp: ControlledModificationCell<Option<PhysicalAddress>>,
    /// The platform's Flattened Device Tree location paramter.
    device_tree: ControlledModificationCell<Option<PhysicalAddress>>,
    /// The platform's SMBIOS 32 location paramter.
    smbios_32: ControlledModificationCell<Option<PhysicalAddress>>,
    /// The platform's SMBIOS 64 location paramter.
    smbios_64: ControlledModificationCell<Option<PhysicalAddress>>,
}

impl PlatformTables {
    /// Constructs an empty [`PlatformTables`] instance.
    const fn new() -> Self {
        Self {
            uefi_system_table: ControlledModificationCell::new(None),
            rsdp: ControlledModificationCell::new(None),
            xsdp: ControlledModificationCell::new(None),
            device_tree: ControlledModificationCell::new(None),
            smbios_32: ControlledModificationCell::new(None),
            smbios_64: ControlledModificationCell::new(None),
        }
    }
}

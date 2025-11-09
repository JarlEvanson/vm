//! UEFI memory-related items.

use core::fmt;

use crate::FmtLowerHex;

/// The current version of the [`MemoryDescriptor`].
pub const CURRENT_MEMORY_DESCRIPTOR_VERSION: u32 = 1;

/// The type of a memory region.
///
/// The type of memory region can affect how the memory region is interpreted by the firmware and
/// OS.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryType(pub u32);

impl MemoryType {
    /// Memory that is not usable.
    pub const RESERVED: Self = Self(0);
    /// Memory that holds the code portions of a loaded UEFI application.
    pub const LOADER_CODE: Self = Self(1);
    /// Memory that holds the data portions of a loaded UEFI application.
    pub const LOEADER_DATA: Self = Self(2);
    /// Memory that holds the code portions of a loaded UEFI Boot Services Driver.
    pub const BOOT_SERVICES_CODE: Self = Self(3);
    /// Memory that holds the data portions of a loaded UEFI Boot Services Driver.
    pub const BOOT_SERVICES_DATA: Self = Self(4);
    /// Memory that holds the code portions of a loaded UEFI Runtime Services Driver.
    ///
    /// Must be preserved by the UEFI OS loader and OS in the working and ACPI S1-S3 states.
    pub const RUNTIME_SERVICES_CODE: Self = Self(5);
    /// Memory that holds the data portions of a loaded UEFI Runtime Services Driver.
    ///
    /// Must be preserved by the UEFI OS loader and OS in the working and ACPI S1-S3 states.
    pub const RUNTIME_SERVICES_DATA: Self = Self(6);
    /// Memory that is free (unallocated).
    pub const CONVENTIONAL: Self = Self(7);
    /// Memory that has had errors detected.
    pub const UNUSABLE: Self = Self(8);
    /// Memory that holds the ACPI tables.
    ///
    /// Once ACPI is enabled, the memory in this range is available for general use.
    pub const ACPI_RECLAIM: Self = Self(9);
    /// Memory that is reserved for use by the firmware.
    ///
    /// Must be preserved by the UEFI OS loader and OS in the working and ACPI S1-S3 states.
    pub const ACPI_NVS: Self = Self(10);
    /// Memory that holds a region of memory-mapped IO.
    ///
    /// Used to request that a memory-mapped IO region is mapped by the OS to a virtual address so
    /// it can be accessed by UEFI Runtime Services.
    pub const MMIO: Self = Self(11);
    /// Memory that holds a region of memory-mapped IO used to translate memory cycles to IO cycles
    /// by the processor.
    pub const MMIO_PORT_SPACE: Self = Self(12);
    /// Memory that holds code that is part of the processor.
    ///
    /// Must be preserved by the UEFI OS loader and OS in the working and ACPI S1-S4 states.
    pub const PAL_CODE: Self = Self(13);
    /// Memory that operates as [`MemoryType::CONVENTIONAL`] but also supports byte-addressable
    /// non-volatility.
    pub const PERSISTENT: Self = Self(14);
    /// Memory that holds unaccepted memory, which must be accepted by the boot target before it
    /// can be used.
    pub const UNACCEPTED: Self = Self(15);
}

/// A description of a region of physical memory.
#[repr(C)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryDescriptor {
    /// The [`MemoryType`] of the region.
    pub region_type: MemoryType,
    /// The physical address at the start of the region.
    pub physical_start: u64,
    /// The virtual address at the start of the region.
    pub virtual_start: u64,
    /// The number of 4KiB pages in the memory region.
    pub number_of_pages: u64,
    /// Attributes of the memory region that describe the bit mask of capabilities for that memory
    /// region and not necessarily the current settings for that memory region.
    pub attribute: u64,
}

impl fmt::Debug for MemoryDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("MemoryDescriptor");

        debug_struct.field("region_type", &self.region_type);
        debug_struct.field("physical_start", &FmtLowerHex(self.physical_start));
        debug_struct.field("virtual_start", &FmtLowerHex(self.virtual_start));
        debug_struct.field("number_of_pages", &FmtLowerHex(self.number_of_pages));
        debug_struct.field("attributes", &FmtLowerHex(self.attribute));

        debug_struct.finish()
    }
}

/// Describes the bit mask of the capabilities for that memory region.
///
/// This is not current settings for that memory region.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryAttribute(pub u64);

impl MemoryAttribute {
    /// The memory region can be configured as uncacheable.
    pub const UNCACHEABLE: Self = Self(0x0000000000000001);
    /// The memory region can be configured as write combining.
    pub const WRITE_COMBINING: Self = Self(0x0000000000000002);
    /// The memory region can be configured as write through.
    pub const WRITE_THROUGH: Self = Self(0x0000000000000004);
    /// The memory region can be configured as write back.
    pub const WRITE_BACK: Self = Self(0x0000000000000008);
    /// The memory region can be configured as uncacheable, exported, and supports the "fetch and
    /// add" semaphore mechanisms.
    pub const UNCACHEABLE_EXPORTED: Self = Self(0x0000000000000010);
    /// The memory region can be configured as write-protected.
    pub const WRITE_PROTECTED: Self = Self(0x0000000000001000);
    /// The memory region can be configured as read-protected.
    pub const READ_PROTECTED: Self = Self(0x0000000000002000);
    /// The memory region can be configured as not executable.
    pub const EXECUTE_PROTECTED: Self = Self(0x0000000000004000);
    /// The memory region can be configured as persistent (non-volatile).
    pub const PERSISTENT: Self = Self(0x0000000000008000);
    /// The memory region is higher reliability relative to other memory in the system.
    pub const MORE_RELIABLE: Self = Self(0x0000000000010000);
    /// The memory region can be configured as read-only.
    pub const READ_ONLY: Self = Self(0x0000000000020000);
    /// The memory region is designated for specific purpose such as for specific device drivers or
    /// applications.
    ///
    /// Prolonged use of this memory may result in suboptimal platform performance.
    pub const SPECIFIC_PURPOSE: Self = Self(0x0000000000040000);
    /// The memory region can be protected by the CPU's memory cryptographic capabilities.
    pub const CPU_CRYPTO: Self = Self(0x0000000000080000);
    /// The memory region is present and capable of having memory dynamically removed from the
    /// platform.
    pub const HOT_PLUGGABLE: Self = Self(0x0000000000100000);
    /// The memory region must be given a virtual mapping by the operating system when UEFI Runtime
    /// Services are relocated to a virtual address mapping.
    pub const RUNTIME: Self = Self(0x8000000000000000);

    /// The memory region is described by additional ISA-specific [`MemoryAttribute`]s as specified
    /// in [`MemoryAttribute::ISA_MASK`].
    pub const ISA_VALID: Self = Self(0x4000000000000000);
    /// Bits reserved for describing optional ISA-specific cacheablility attributes that are not
    /// covered [`MemoryAttribute::UNCACHEABLE`], [`MemoryAttribute::WRITE_COMBINING`],
    /// [`MemoryAttribute::WRITE_THROUGH`], [`MemoryAttribute::WRITE_BACK`],
    /// [`MemoryAttribute::UNCACHEABLE_EXPORTED`].
    pub const ISA_MASK: Self = Self(0x0FFFF00000000000);
}

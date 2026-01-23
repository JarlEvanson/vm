//! # The REVM Boot Protocol
//!
//! This file serves as the protocol's specification.
//!
//! ## General Notes
//!
//! The `executable` is a kernel or other freestanding application being loaded by the REVM boot
//! protocol compliant bootloader.
//!
//! The REVM boot protocol does not enforce any specific executable binary format to use.
//!
//! The ABIs the REVM protocol uses and expects the executable to comply with as as follows:
//! - **x86_64**: System V ABI without FP/SIMD
//! - **aarch64**: AAPCS64 without FP/SIMD
//!
//! The executable can internally use FP/SIMD, but when interfacing with the REVM boot protocol,
//! the above are the expected ABIs.
#![no_std]

use core::fmt;

pub mod aarch64;
pub mod x86_64;

pub use GenericTableV0 as GenericTable;
pub use HeaderV0 as Header;

/// The header for the REVM protocol table.
///
/// This structure will always be backward compatible.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderV0 {
    /// The base version of the REVM protocol that was used to produce this table.
    pub version: u64,
    /// The last base version that was a major version (namely, it broke API/ABI compatibility).
    pub last_major_version: u64,
    /// The total size, in bytes, of the REVM protocol table.
    pub length: u64,
    /// The offset, in bytes, from the start of the [`Header`] to the [`GenericTable`].
    pub generic_table_offset: u64,
    /// The offset, in bytes, from the start of the [`Header`] to the architecture-specific table.
    pub arch_table_offset: u64,
}

impl HeaderV0 {
    /// The base version of the REVM protocol with which this [`Header`] is associated.
    pub const VERSION: u64 = 0;
    /// The last major version of the REVM protocol with which this [`Header`] is associated.
    pub const LAST_MAJOR_VERSION: u64 = 0;
}

/// Table providing information and functionality that is cross-architectural in nature.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GenericTableV0 {
    /// The version of the [`GenericTable`] with which this table identifies.
    pub version: u64,

    /// The smallest unit for allocating frames and mapping pages available. This is also the
    /// minimum alignment for allocation and mapping.
    ///
    /// This will always be greater than or equal to 256.
    pub page_frame_size: u64,

    /// Writes the UTF-8 string described by `string` and `length` to some logging mechanism
    /// provided by the bootloader.
    pub write: unsafe extern "C" fn(string: *const u8, length: u64) -> Status,

    /// Allocates a region of `count` frames aligned to `alignment`. The physical address at the
    /// start of the allocated region is written to `physical_address` on success.
    pub allocate_frames: unsafe extern "C" fn(
        count: u64,
        alignment: u64,
        flags: AllocationFlags,
        physical_address: *mut u64,
    ) -> Status,

    /// Deallocates a region of `count` frames with a base at `physical_address`.
    pub deallocate_frames: unsafe extern "C" fn(physical_address: u64, count: u64) -> Status,

    /// Returns the current physical memory map.
    ///
    /// - `size`: On input, the size of the buffer pointed to by `map`. On output, the size of the
    ///   buffer returned by the bootloader or the size of the required buffer if the provided
    ///   buffer was too small.
    /// - `map`: A pointer to the buffer into which the memory map should be written.
    /// - `key`: The location to which the key identifying version of the physical memory map that
    ///   was written should be written.
    /// - `descriptor_size`: The size, in bytes, of each [`MemoryDescriptor`].
    /// - `descriptor_version`: The version associated with the layout of [`MemoryDescriptor`].
    pub get_memory_map: unsafe extern "C" fn(
        size: *mut u64,
        map: *mut MemoryDescriptor,
        key: *mut u64,
        descriptor_size: *mut u64,
        descriptor_version: *mut u64,
    ) -> Status,

    /// Maps the physical region beginning at `physical_address` into the executable's address
    /// space starting at `virtual_address`. This mapping extends for `count`
    /// [`GenericTable::page_frame_size`] blocks.
    ///
    /// When [`GenericTable::map`] is called for overlapping mappings, this call must succeed.
    pub map: unsafe extern "C" fn(
        physical_address: u64,
        virtual_address: u64,
        count: u64,
        flags: MapFlags,
    ) -> Status,

    /// Unmaps the virtual region starting at `virtual_address` and extending for `count`
    /// [`GenericTable::page_frame_size`] blocks.
    pub unmap: unsafe extern "C" fn(virtual_address: u64, count: u64) -> Status,

    /// Signals to the bootloader that the application wishes to take over control of the computer.
    /// `key` is utilized to ensure that the memory map that the application has is current.
    ///
    /// On success, the application becomes the sole controller of the system. This means that the
    /// executable can directly manipulate the hardware and has sole control over the computer.
    ///
    /// After this function has been called, all services/functions provided by this protocol are
    /// invalid and must not be called.
    pub takeover: unsafe extern "C" fn(key: u64, flags: TakeoverFlags) -> Status,
}

impl GenericTableV0 {
    /// The version of the [`GenericTable`] with which this [`GenericTable`] is associated.
    pub const VERSION: u64 = 0;
}

impl fmt::Write for GenericTableV0 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // SAFETY:
        //
        // The [`str`] is a valid UTF-8 string with the proper length.
        #[expect(clippy::as_conversions)]
        let result = unsafe { (self.write)(s.as_ptr(), s.len() as u64) };
        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

/// Various flags affecting the behavior of [`GenericTable::allocate_frames`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationFlags(pub u64);

impl AllocationFlags {
    /// Any available range of frames that satisfies the request may be returned.
    pub const ANY: Self = Self(0);
    /// The only available range of frames that starts at the provided address may be returned.
    pub const AT: Self = Self(1);
    /// Any available range of frames that is entirely below `physical_address` may be returned.
    pub const BELOW: Self = Self(2);
    /// Bitmask of bits determining the type of the allocation.
    pub const TYPE: Self = Self(0b11);

    /// Bitmask of the valid flags.
    pub const VALID: Self = Self(Self::TYPE.0);
}

/// Various flags affecting the behavior of [`GenericTable::takeover`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TakeoverFlags(pub u64);

impl TakeoverFlags {
    /// Indicates to the firmware that the application wish to transparently take over control of
    /// the computer. This means that the bootloader needs to prepare the environment such that the
    /// application can virtualize the boot environment.
    ///
    /// [`GenericTable::takeover`] and only [`GenericTable::takeover`] may be called twice if this
    /// flag is set for both calls.
    ///
    /// The first call must prepare prepare the computer for the application virtualizing the boot
    /// environment (this means it must quiescent any VMs or other activities that might interfere
    /// with an easy transition).
    ///
    /// The second call should be done within the application's virtual machine and indicates to
    /// the bootloader that it can restart anything that the bootloader put into quiescent and may
    /// underload itself it so desired. The second call must not return, may unload the
    /// bootloader, and may unload or change the protocol table.
    pub const IN_PLACE: Self = Self(1);

    /// Mask over all valid flags.
    pub const VALID: Self = Self::IN_PLACE;
}

/// Description of a single memory region.
///
/// This will be backwards compatible within a major version.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryDescriptor {
    /// The physical address of the first byte in the memory region.
    ///
    /// This will always be aligned to [`GenericTable::page_frame_size`].
    pub start: u64,
    /// The number of [`GenericTable::page_frame_size`] frames in the memory region.
    pub count: u64,
    /// The type of the memory region.
    pub region_type: MemoryType,
}

impl MemoryDescriptor {
    /// The version of the [`MemoryDescriptor`] with wich this [`MemoryDescriptor`] is associated
    /// (this is the value returned in `descriptor_version`).
    pub const VERSION: u64 = 0;
}

/// Various types of memory regions.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemoryType(pub u32);

impl MemoryType {
    /// Memory that is not usable.
    pub const RESERVED: Self = Self(0);
    /// Memory that is unallocated and free for general usage.
    pub const FREE: Self = Self(1);
    /// Memory that is used to store parts of the bootloader, firmware, or the executable.
    ///
    /// This memory can be reclaimed as soon as the executable is no longer utilizing the memory.
    pub const BOOTLOADER_RECLAIMABLE: Self = Self(2);
    /// Memory in which errors have been detected.
    pub const BAD: Self = Self(3);
    /// Memory that holds ACPI tables.
    pub const ACPI_RECLAIMABLE: Self = Self(4);
    /// Memory that holds non-volatile ACPI data.
    pub const ACPI_NON_VOLATILE: Self = Self(5);
}

impl fmt::Debug for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::RESERVED => f.pad("RESERVED"),
            Self::FREE => f.pad("FREE"),
            Self::BOOTLOADER_RECLAIMABLE => f.pad("BOOTLOADER_RECLAIMABLE"),
            Self::BAD => f.pad("BAD"),
            Self::ACPI_RECLAIMABLE => f.pad("ACPI_RECLAIMABLE"),
            Self::ACPI_NON_VOLATILE => f.pad("ACPI_NON_VOLATILE"),

            unknown => f.debug_tuple("MemoryType").field(&unknown.0).finish(),
        }
    }
}

/// Various flags affecting the behavior of [`GenericTable::map`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapFlags(pub u64);

impl MapFlags {
    /// The mapped virtual memory region should be wriable.
    pub const WRITE: Self = Self(1 << 1);
    /// The mapped virtual memory region should be executable.
    pub const EXECUTE: Self = Self(1 << 2);

    /// Bitmask of the valid flags.
    pub const VALID: Self = Self(Self::WRITE.0 | Self::EXECUTE.0);
}

/// Various status codes.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Status(pub u64);

impl Status {
    /// The operation was successful.
    pub const SUCCESS: Self = Self(0);

    /// The bit, that if set, indicates the [`Status`] indicates an error.
    pub const ERROR_BIT: u64 = 1 << 63;

    /// The function or data was provided or utilized in an improper manner.
    #[expect(clippy::identity_op)]
    pub const INVALID_USAGE: Self = Self(Self::ERROR_BIT | 0);
    /// The system cannot allocate the required amount of memory.
    pub const OUT_OF_MEMORY: Self = Self(Self::ERROR_BIT | 1);
    /// The item could not be found.
    pub const NOT_FOUND: Self = Self(Self::ERROR_BIT | 2);
    /// The attempted usage is not supported.
    pub const NOT_SUPPORTED: Self = Self(Self::ERROR_BIT | 3);
    /// The provided memory map key is not current.
    pub const INVALID_KEY: Self = Self(Self::ERROR_BIT | 4);
    /// The provided buffer is too small.
    pub const BUFFER_TOO_SMALL: Self = Self(Self::ERROR_BIT | 5);
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SUCCESS => f.pad("SUCCESS"),

            Self::INVALID_USAGE => f.pad("INVALID_USAGE"),
            Self::OUT_OF_MEMORY => f.pad("OUT_OF_MEMORY"),
            Self::NOT_FOUND => f.pad("NOT_FOUND"),
            Self::NOT_SUPPORTED => f.pad("NOT_SUPPORTED"),
            Self::INVALID_KEY => f.pad("INVALID_KEY"),
            Self::BUFFER_TOO_SMALL => f.pad("BUFFER_TOO_SMALL"),

            unknown => f.debug_tuple("Status").field(&unknown.0).finish(),
        }
    }
}

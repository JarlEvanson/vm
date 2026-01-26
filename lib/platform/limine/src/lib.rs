//! Definitions and implementations of the Limine boot protocol.

#![no_std]

pub mod boot_time;
pub mod bootloader_info;
pub mod device_tree;
pub mod efi_mem_map;
pub mod efi_sys_table;
pub mod entry_point;
pub mod executable;
pub mod executable_addr;
pub mod firmware_type;
pub mod framebuffer;
pub mod hhdm;
pub mod memory_map;
pub mod module;
pub mod mp;
pub mod paging_mode;
pub mod rsdp;
pub mod smbios;
pub mod stack_size;

/// Marks the start of the Limine boot protocol feature requests section.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct RequestsStartMarker([u64; 4]);

impl RequestsStartMarker {
    /// Constructs a new [`RequestsStartMarker`].
    pub const fn new() -> Self {
        Self([
            0xf6b8f4b39de7d1ae,
            0xfab91a6940fcb9cf,
            0x785c6ed015d3e316,
            0x181e920a7852b9d9,
        ])
    }
}

impl Default for RequestsStartMarker {
    fn default() -> Self {
        Self::new()
    }
}

/// Marks the end of the Limine boot protocol feature requests section.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct RequestsEndMarker([u64; 2]);

impl RequestsEndMarker {
    /// Constructs a new [`RequestsEndMarker`].
    pub const fn new() -> Self {
        Self([0xadc0e0531bb10d03, 0x9572709f31764c62])
    }
}

impl Default for RequestsEndMarker {
    fn default() -> Self {
        Self::new()
    }
}

/// The first of the two common magic numbers used to identify a Limine boot protocol feature
/// request.
pub const REQUEST_MAGIC_0: u64 = 0xc7b1dd30df4c8b88;
/// The first of the two common magic numbers used to identify a Limine boot protocol feature
/// request.
pub const REQUEST_MAGIC_1: u64 = 0x0a82e883a194f07b;

/// The base revision of the Limine boot protocol this crate provides.
pub const BASE_REVISION: u64 = 4;

/// A tag setting the minimum base revision of the Limine boot protocol supported by the
/// executable and containing the base revision of the Limine boot protocol to load the
/// executable.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseRevisionTag {
    /// Magic number.
    pub magic: u64,

    /// The base revision of the Limine boot protocol used to load the executable.
    ///
    /// This field may not contain the correct revision if the bootloader does not support base
    /// revision 3 or greater.
    pub loaded_revision: u64,

    /// The application's requested base revision.
    ///
    /// This must be set to 0 if the executable's base revision is supported by the bootloader.
    pub requested_revision: u64,
}

impl BaseRevisionTag {
    /// The first magic number indicating the base revision tag.
    const MAGIC_0: u64 = 0xf9562b2d5c95a6c8;
    /// The second magic number indicating the base revision tag.
    const MAGIC_1: u64 = 0x6a7b384944536bdc;

    /// Returns a newly constructed [`BaseRevisionTag`] that requests the `requested_revision`.
    pub const fn new(requested_revision: u64) -> Self {
        Self {
            magic: Self::MAGIC_0,
            loaded_revision: Self::MAGIC_1,
            requested_revision,
        }
    }

    /// Returns a newly constructed [`BaseRevisionTag`] that requests the base revision of the
    /// Limine boot protocol that this crate provides.
    pub const fn new_current() -> Self {
        Self::new(BASE_REVISION)
    }

    /// Returns `true` if the [`BaseRevisionTag::requested_revision`] was set to zero.
    pub const fn is_supported(&self) -> bool {
        self.requested_revision == 0
    }

    /// Returns the base revision of the Limine protocol that this executable was loaded with, if
    /// the [`BaseRevisionTag::loaded_revision`] field changed.
    pub const fn loaded_revision(&self) -> Option<u64> {
        if self.loaded_revision != Self::MAGIC_1 {
            Some(self.loaded_revision)
        } else {
            None
        }
    }
}

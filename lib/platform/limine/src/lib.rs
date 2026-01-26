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
pub const REQUESTS_START_MARKER: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

/// Marks the end of the Limine boot protocol feature requests section.
pub const REQUESTS_END_MARKER: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

/// The first of the two common magic numbers used to identify a Limine boot protocol feature
/// request.
pub const REQUEST_MAGIC_0: u64 = 0xc7b1dd30df4c8b88;
/// The first of the two common magic numbers used to identify a Limine boot protocol feature
/// request.
pub const REQUEST_MAGIC_1: u64 = 0x0a82e883a194f07b;

/// The base revision of the Limine boot protocol this crate provides.
pub const BASE_REVISION: u64 = 4;

/// 1st magic number indicating the base revision tag.
pub const BASE_REVISION_MAGIC_0: u64 = 0xf9562b2d5c95a6c8;
/// 2nd magic number indicating the base revision tag.
pub const BASE_REVISION_MAGIC_1: u64 = 0x6a7b384944536bdc;

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
    /// revision 3.
    pub loaded_revision: u64,

    /// The application's expected base revision.
    ///
    /// This must be set to 0 if the executable's base revision is supported by the bootloader.
    pub supported_revision: u64,
}

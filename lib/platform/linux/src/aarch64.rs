//! Definitions for the `aarch64` linux boot protocol.

/// The 64-byte header at the start of a valid `linux` boot protocol image.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// Code bytes responsible for branching to remainder of executable.
    pub code_0: u32,
    /// Code bytes responsible for branching to remainder of executable.
    pub code_1: u32,
    /// Image load offset (little endian).
    pub text_offset: u64,
    /// Effective image size (little endian).
    pub image_size: u64,
    /// Kernel flags (little endian).
    pub flags: u64,
    /// Reserved.
    pub res_2: u64,
    /// Reserved.
    pub res_3: u64,
    /// Reserved.
    pub res_4: u64,
    /// Magic number (little endian).
    pub magic: u32,
    /// Reserved.
    pub res_5: u32,
}

impl Header {
    /// Native endian representation of the magic number.
    pub const MAGIC: u32 = u32::from_le_bytes([0x41, 0x52, 0x4d, 0x64]);
}

/// Flags field.
pub struct Flags(pub u64);

impl Flags {
    /// The executable is little endian.
    pub const LITTLE_ENDIAN: Self = Self(0);
    /// The executable is big endian.
    pub const BIG_ENDIAN: Self = Self(1);

    /// Mask over all endianness possibilities.
    const ENDIAN_MASK: u64 = 1;

    /// The executable is targeted towards an unspecified page size.
    pub const GRANULE_UNSPECIFIED: Self = Self(0b00 << 1);
    /// The executable is targeted at 4 KiB pages.
    pub const GRANULE_4_KIB: Self = Self(0b01 << 1);
    /// The executable is targeted at 16 KiB pages.
    pub const GRANULE_16_KIB: Self = Self(0b10 << 1);
    /// The executable is targeted at 64 KiB pages.
    pub const GRANULE_64_KIB: Self = Self(0b11 << 1);

    /// Mask over all granule possibilities.
    const GRANULE_MASK: u64 = 0b11 << 1;

    /// 2 MiB aligned executable base should be as close as possible to the base of DRAM.
    pub const PLACEMENT_LOW: Self = Self(0 << 3);
    /// 2 MiB aligned executable base can be placed anywhere in physical memory (within 48-bit
    /// range).
    pub const PLACEMENT_ANY: Self = Self(1 << 3);

    /// Mask over all placement possibilities.
    const PLACEMENT_MASK: u64 = 0b1 << 3;

    /// Returns `true` if the executable image is little endian.
    pub const fn is_little_endian(&self) -> bool {
        !self.is_big_endian()
    }

    /// Returns `true` if the executable image is big endian.
    pub const fn is_big_endian(&self) -> bool {
        (self.0 & Self::ENDIAN_MASK) == Self::BIG_ENDIAN.0
    }

    /// Returns the required memory page granule size descriptor.
    pub const fn granule_size(&self) -> Self {
        Self(self.0 & Self::GRANULE_MASK)
    }

    /// Returns `true` if the executable can be placed anywhere in physical memory.
    pub const fn can_place_anywhere(&self) -> bool {
        (self.0 & Self::PLACEMENT_MASK) == Self::PLACEMENT_ANY.0
    }
}

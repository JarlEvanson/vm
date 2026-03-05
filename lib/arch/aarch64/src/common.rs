//! Common structures that occur in different modules.

/// An exception level.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum EL {
    /// The lowest exception level.
    ///
    /// Typically used for general applications.
    #[default]
    EL0,
    /// The second lowest exception level.
    ///
    /// Typically used for OS kernels and other privileged applications.
    EL1,
    /// The second highest exception level.
    ///
    /// Typically used for hypervisors.
    EL2,
    /// The highest exception level.
    ///
    /// Typically used for monitors.
    EL3,
}

/// The minimum size of a translation region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Granule {
    /// The minimum size of a translation region is 4 KiB.
    Page4KiB,
    /// The minimum size of a translation region is 16 KiB.
    Page16KiB,
    /// The minimum size of a translation region is 64 KiB.
    Page64KiB,
}

impl Granule {
    /// Returns the size, in bytes, of a page according to this [`Granule`].
    pub const fn size(self) -> u32 {
        match self {
            Self::Page4KiB => 4 * 1024,
            Self::Page16KiB => 16 * 1024,
            Self::Page64KiB => 64 * 1024,
        }
    }
}

/// The physical address space size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhysicalAddressSpaceSize {
    /// The physical address is 32 bits.
    Bits32,
    /// The physical address is 36 bits.
    Bits36,
    /// The physical address is 40 bits.
    Bits40,
    /// The physical address is 42 bits.
    Bits42,
    /// The physical address is 44 bits.
    Bits44,
    /// The physical address is 48 bits.
    Bits48,
    /// The physical address is 52 bits.
    Bits52,
    /// The physical address is 56 bits.
    Bits56,
}

impl PhysicalAddressSpaceSize {
    /// Constructs a new [`PhysicalAddressSpaceSize`] from the given bits.
    pub const fn from_bits(val: u8) -> Self {
        match val {
            0b0000 => Self::Bits32,
            0b0001 => Self::Bits36,
            0b0010 => Self::Bits40,
            0b0011 => Self::Bits42,
            0b0100 => Self::Bits44,
            0b0101 => Self::Bits48,
            0b0110 => Self::Bits52,
            0b0111 => Self::Bits56,
            _ => unreachable!(),
        }
    }

    /// Returns the bit representation of this [`PhysicalAddressSpaceSize`].
    pub const fn to_bits(self) -> u8 {
        match self {
            Self::Bits32 => 0b0000,
            Self::Bits36 => 0b0001,
            Self::Bits40 => 0b0010,
            Self::Bits42 => 0b0011,
            Self::Bits44 => 0b0100,
            Self::Bits48 => 0b0101,
            Self::Bits52 => 0b0110,
            Self::Bits56 => 0b0111,
        }
    }

    /// Returns the number of bits in the physical address space.
    pub const fn to_val(self) -> u8 {
        match self {
            Self::Bits32 => 32,
            Self::Bits36 => 36,
            Self::Bits40 => 40,
            Self::Bits42 => 42,
            Self::Bits44 => 44,
            Self::Bits48 => 48,
            Self::Bits52 => 52,
            Self::Bits56 => 56,
        }
    }
}

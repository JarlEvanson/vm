//! Definitions of standard UEFI table types.

use core::fmt;

pub mod boot;
pub mod config;
pub mod runtime;
pub mod system;

/// Data structure that comes before all of the standard UEFI table types.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableHeader {
    /// A 64-bit signature that identifies the type of table that follows.
    pub signature: u64,
    /// The revision of the UEFI specification to which this table conforms.
    pub revision: Revision,
    /// The size, in bytes, of the entire table including this [`TableHeader`].
    pub header_size: u32,
    /// The 32-bit CRC for the entire table.
    ///
    /// Computed by setting this field to 0 and computing the 32-bit CRC for
    /// [`TableHeader::header_size`] bytes.
    pub crc_32: u32,
    /// Reserved field that must be set to 0.
    pub reserved: u32,
}

/// A revision of the UEFI specification.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(pub u32);

impl Revision {
    /// Returns the major revision of the UEFI specification to which the table conforms.
    #[must_use]
    pub const fn major(self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Returns the minor revision of the UEFI specification to which the table conforms.
    #[must_use]
    pub const fn minor(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

impl fmt::Debug for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (major, minor) = (self.major(), self.minor());
        if self.major() == 1 {
            write!(f, "{major}.{minor:02}")
        } else {
            let (minor, patch) = (self.minor() / 10, self.minor() % 10);
            if patch == 0 {
                write!(f, "{major}.{minor}")
            } else {
                write!(f, "{major}.{minor}.{patch}")
            }
        }
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

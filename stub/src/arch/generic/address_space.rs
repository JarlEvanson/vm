//! Implementations of paging structures.

use core::{error, fmt};

/// Interface to manipulate an address space.
///
/// # Safety
///
/// The [`AddressSpace`] trait is an unsafe trait because implementors of this trait must correctly
/// implement the semantics of each method in order to prevent memory corruption.
pub trait AddressSpace {
    /// Returns the size of a page in bytes.
    fn page_size(&self) -> u64;

    /// Returns the maximum virtual address.
    ///
    /// This is an inclusive range.
    fn max_virtual_address(&self) -> u64;

    /// Returns the maximum supported virtual address.
    ///
    /// This is an inclusive range.
    fn max_physical_address(&self) -> u64;

    /// Maps the physical region with a base of `physical_address` and a size of `count *
    /// Self::page_size()` into the [`AddressSpace`] at `virtual_address` with the specified
    /// [`ProtectionFlags`].
    ///
    /// # Errors
    ///
    /// - [`MapError::AlignmentError`]: Returned when the `physical_address` or the
    ///   `virtual_address` is not aligned to the [`AddressSpace::page_size()`].
    /// - [`MapError::AllocationError`]: Returned when an error allocating memory required to map
    ///   the region occurs.
    /// - [`MapError::GeneralError`]: Returned when [`AddressSpace::map()`] fails in a manner
    ///   that does not belong to any other [`MapError`] value.
    /// - [`MapError::InvalidAddress`]: Returned when `physical_address` or `virtual_address`
    ///   is not a valid address.
    /// - [`MapError::InvalidSize`]: Returned when the size of the region is too large.
    /// - [`MapError::WrapAroundError`]: Returned when the region described by `physical_address`
    ///   or `virtual_address` overflows.
    fn map(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        count: u64,
        protection: ProtectionFlags,
    ) -> Result<(), MapError>;

    /// Unmaps `count` pages at starting at `virtual_address`.
    ///
    /// # Safety
    ///
    /// The unmapped region must not be accessed while unmapped.
    unsafe fn unmap(&mut self, virtual_address: u64, count: u64);

    /// Returns the virtual address corresponding to the base of a free region of `count` pages.
    ///
    /// # Errors
    ///
    /// Returns [`NotFound`] if a region of `count` pages cannot be found.
    fn find_region(&self, count: u64) -> Result<u64, NotFound>;

    /// Translates the given `virtual_address` to its `physical_address`.
    ///
    /// # Errors
    ///
    /// Returns [`NoMapping`] if there exists no mapping from `virtual_address` to a physical
    /// address.
    fn translate_virt(&self, virtual_address: u64) -> Result<u64, NoMapping>;
}

/// Protection settings for a page in an address space.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProtectionFlags(u32);

impl ProtectionFlags {
    /// The page should be readable.
    pub const READ: Self = Self(0x1);
    /// The page should be writable.
    pub const WRITE: Self = Self(0x2);
    /// The page should be executable.
    pub const EXECUTE: Self = Self(0x4);

    /// Returns `true` if the page should be readable.
    pub fn readable(self) -> bool {
        self & Self::READ == Self::READ
    }

    /// Returns `true` if the page should be writable.
    pub fn writable(self) -> bool {
        self & Self::WRITE == Self::WRITE
    }

    /// Returns `true` if the page should be executable.
    pub fn executable(self) -> bool {
        self & Self::EXECUTE == Self::EXECUTE
    }
}

impl core::ops::BitOr for ProtectionFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for ProtectionFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl core::ops::BitAnd for ProtectionFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitAndAssign for ProtectionFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl core::ops::BitXor for ProtectionFlags {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl core::ops::BitXorAssign for ProtectionFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl core::ops::Not for ProtectionFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Various errors that can occur when mapping a physical region into an address space.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapError {
    /// The requested mapping starts at an invalid address.
    AlignmentError,
    /// An error occurred when allocating memory required to fulfill the requested mapping.
    AllocationError,
    /// An unspecified error occurred.
    GeneralError,
    /// A provided address is invalid.
    InvalidAddress,
    /// The size of the requested mapping is invalid.
    InvalidSize,
    /// The requested mapping would wrap around an address space.
    ///
    /// This address space may be the physical or virtual address space.
    WrapAroundError,
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlignmentError => "requested mapping starts at an invalid alignment".fmt(f),
            Self::AllocationError => "requested mapping had a required allocation fail".fmt(f),
            Self::GeneralError => "an unspecified error occurred".fmt(f),
            Self::InvalidAddress => "requested mapping involves an invalid address".fmt(f),
            Self::InvalidSize => "requested mapping is too large".fmt(f),
            Self::WrapAroundError => "requested mapping would wrap around an address space".fmt(f),
        }
    }
}

impl error::Error for MapError {}

/// An error obtained when attempting to unmap a virtual region that is already not mapped.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotMapped;

impl fmt::Display for NotMapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "requested unmapping region was not mapped".fmt(f)
    }
}

impl error::Error for NotMapped {}

/// An error obtained when a region cannot be found.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotFound;

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "region of requested size could not be found".fmt(f)
    }
}

impl error::Error for NotFound {}

/// An error obtained when attempting to translate a physical or virtual address using an address
/// space.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoMapping;

impl fmt::Display for NoMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "no mapping exists to facilitate the translation".fmt(f)
    }
}

impl error::Error for NoMapping {}

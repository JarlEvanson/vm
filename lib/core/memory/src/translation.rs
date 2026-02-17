//! Abstraction over address translation schemes.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use crate::address::{Address, AddressChunkRange, AddressSpaceDescriptor};

/// A trait representing low-level [`Address`] translation.
///
/// The [`TranslationScheme`] trait provides an abstraction for mapping, translating, and unmapping
/// [`AddressChunkRange`]s from an address space. It supports variable-sized [`AddressChunk`][ac]s
/// and is capable of support both virtual memory translation and memory virtualization techniques
/// like EPT on `x86_64`.
///
/// # Safety
///
/// Manipulating [`AddressChunk`][ac]s can have side effects and may violate memory safety if not
/// carefully managed. Implementors must ensure that proper address validation occurrs, the APIs
/// are implemented as specified, and any implementations adhere to the architectural specification
/// if necessary.
///
/// [ac]: crate::address::AddressChunk
pub unsafe trait TranslationScheme {
    /// Returns the [`AddressSpaceDescriptor`] that describes the address space that the
    /// [`TranslationScheme`] receives as input.
    fn input_descriptor(&self) -> AddressSpaceDescriptor;

    /// Returns the [`AddressSpaceDescriptor`] that describes the address space that the
    /// [`TranslationScheme`] outputs into.
    fn output_descriptor(&self) -> AddressSpaceDescriptor;

    /// Returns the size, in bytes, of the smallest translation granule.
    ///
    /// This value can be used with [`AddressChunk`][ac] and [`AddressChunkRange`] APIs.
    ///
    /// [ac]: crate::address::AddressChunk
    fn chunk_size(&self) -> u64;

    /// Maps the [`AddressChunkRange`] in the input address space to the [`AddressChunkRange`] in
    /// the output address space with the provided [`MapFlags`].
    ///
    /// # Errors
    ///
    /// - [`MapError::MappingMismatch`]: Returned if `input` and `output` are not the same size.
    /// - [`MapError::InvalidRange`]: Returned if the requested [`AddressChunkRange`]s are not
    ///   valid in their respective address spaces.
    /// - [`MapError::OverlapError`]: Returned if the `input` [`AddressChunkRange`] contains
    ///   pre-existing mappings and [`MapFlags::MAY_OVERWRITE`] was not set.
    /// - [`MapError::OutOfMemory`]: Returned if an error occurred while allocating memory required
    ///   to complete the map request.
    ///
    /// # Safety
    ///
    /// Any existing mappings that are overwritten as a result of this call must have not been in
    /// use and the [`AddressChunkRange`] in the output space must be valid for the intended use
    /// case.
    unsafe fn map(
        &mut self,
        input: AddressChunkRange,
        output: AddressChunkRange,
        flags: MapFlags,
    ) -> Result<(), MapError>;

    /// Unmaps the [`AddressChunkRange`] in the input address space from the [`TranslationScheme`].
    ///
    /// # Safety
    ///
    /// The [`AddressChunkRange`] in the input address space must not be in use.
    unsafe fn unmap(&mut self, input: AddressChunkRange);

    /// Translates the provided [`Address`] in the input address space into its corresponding
    /// [`Address`] in the output address space and returns its associated [`MapFlags`]
    /// configuration.
    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)>;
}

/// Configuration for a mapped [`AddressChunkRange`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapFlags(pub u64);

impl MapFlags {
    /// The mapped input [`AddressChunkRange`] should be readable.
    pub const READ: Self = Self(1 << 0);
    /// The mapped input [`AddressChunkRange`] should be wriable.
    pub const WRITE: Self = Self(1 << 1);
    /// The mapped input [`AddressChunkRange`] should be executable.
    pub const EXEC: Self = Self(1 << 2);

    /// The requested mapping represents normal memory (this should be utilized for normal RAM that
    /// is not being utilized for DMA).
    pub const CACHE_NORMAL: Self = Self(0b00 << 3);
    /// The requested mapping represents uncacheable normal memory (typically DMA memory).
    pub const CACHE_NORMAL_UNCACHEABLE: Self = Self(0b01 << 3);
    /// The requested mapping represents device memory (typically memory mapped registers).
    pub const CACHE_DEVICE: Self = Self(0b10 << 3);
    /// The requested mapping represents device memory on which it is safe to perform
    /// write-combining (typically framebuffers).
    pub const CACHE_WRITE_COMBINING: Self = Self(0b11 << 3);

    /// Bitmask over the cache options.
    pub const VALID_CACHE: Self = Self(0b11 << 3);

    /// The requested mapping may overwrite existing mappings.
    pub const MAY_OVERWRITE: Self = Self(1 << 5);

    /// Bitmask of the valid flags.
    pub const VALID: Self = Self(
        Self::READ.0 | Self::WRITE.0 | Self::EXEC.0 | Self::VALID_CACHE.0 | Self::MAY_OVERWRITE.0,
    );

    /// Returns `true` if the provided [`MapFlags`] does not have any invalid bits set.
    pub const fn is_valid(&self) -> bool {
        (self.0 & Self::VALID.0) == self.0
    }

    /// Returns `true` if the flags in `other` are set in `self`.
    pub const fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for MapFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MapFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitAnd for MapFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for MapFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

/// Various errors that can occur while mapping an input [`AddressChunk`][ac] to an output
/// [`AddressChunk`][ac].
///
/// [ac]: crate::address::AddressChunk
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    /// The sizes of the provided [`AddressChunkRange`]s are not the same.
    MappingMismatch,
    /// The requested [`AddressChunkRange`]s are not valid.
    InvalidRange,
    /// The requested [`AddressChunkRange`] contained pre-existing mappings and
    /// [`MapFlags::MAY_OVERWRITE`] was not set.
    OverlapError,
    /// An error occurred while allocating memory for the [`TranslationScheme`].
    OutOfMemory,
}

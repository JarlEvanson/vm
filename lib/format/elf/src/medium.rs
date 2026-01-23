//! Generic API over immutable and contiguous byte sources.

use core::{error, fmt};

use crate::{u64_to_usize, usize_to_u64};

/// Generic API over various immutable and contiguous byte sources.
///
/// The medium must be immutable. This means that byte values never change between reads and the
/// value of [`Medium::size()`] also never changes.
///
/// # Implementors
///
/// Implementations must treat any overflow in `offset + length` as a bounds error (helpful utility
/// functions are provided as [`check_bounds()`] and [`check_bounds_usize()`]).
pub trait Medium {
    /// Any errors that might need to be propagated up through the [`Medium`] abstraction.
    type Error;

    /// The number of bytes available to be retrieved.
    ///
    /// This value must not change but may be zero.
    ///
    /// # Implementors
    ///
    /// This function should be cheap.
    fn size(&self) -> u64;

    /// Read a single byte from `offset`.
    ///
    /// # Errors
    ///
    /// - [`MediumError::BoundsError`]: Requested region is outside of the bounds of [`Medium`].
    /// - [`MediumError::UnderlyingError`]: The underlying region returned an error when accessing
    ///   it.
    fn read_byte(&self, offset: u64) -> Result<u8, MediumError<Self::Error>> {
        let mut val = 0;

        self.read_slice(offset, core::array::from_mut(&mut val))?;
        Ok(val)
    }

    /// Read `slice.len()` bytes into `slice` from `offset`.
    ///
    /// # Errors
    ///
    /// - [`MediumError::BoundsError`]: Requested region is outside of the bounds of [`Medium`].
    /// - [`MediumError::UnderlyingError`]: The underlying region returned an error when accessing
    ///   it.
    fn read_slice(&self, offset: u64, slice: &mut [u8]) -> Result<(), MediumError<Self::Error>>;
}

/// A [`BackedMedium`] provides an API to provide access to contiguous addressable bytes that can
/// safely be borrowed as slices.
///
/// # Implementors
///
/// Implementations must treat any overflow in `offset + length` as a bounds error (helpful utility
/// functions are provided as [`check_bounds()`] and [`check_bounds_usize()`]). The backing
/// storage must be stable.
pub trait BackedMedium: Medium {
    /// Accesses a slice of `length` bytes at `offset` into the [`BackedMedium`].
    ///
    /// # Errors
    ///
    /// - [`MediumError::BoundsError`]: Requested region is outside of the bounds of [`Medium`].
    /// - [`MediumError::UnderlyingError`]: The underlying medium returned an error when accessing
    ///   it.
    fn access_slice(&self, offset: u64, length: u64) -> Result<&[u8], MediumError<Self::Error>>;
}

/// Various errors that can occur when interacting with a [`Medium`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediumError<E> {
    /// Requested region is outside of the bounds of [`Medium`].
    BoundsError {
        /// The offset, in bytes, of the start of the requested region in the [`Medium`].
        offset: u64,
        /// The size, in bytes, of the requested region.
        length: u64,
        /// The actual size of the [`Medium`].
        size: u64,
    },
    /// An error that might occur when accessing the medium.
    UnderlyingError(E),
}

impl<E> From<E> for MediumError<E> {
    fn from(value: E) -> Self {
        Self::UnderlyingError(value)
    }
}

impl<E: fmt::Display> fmt::Display for MediumError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoundsError {
                offset,
                length,
                size,
            } => write!(
                f,
                "requested region at {offset} bytes with a length of {length} \
                does not fit inside medium of {size} bytes"
            ),
            Self::UnderlyingError(error) => write!(f, "error accessing underlying medium: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for MediumError<E> {}

/// Utility function to centralize [`Medium`] bounds checking.
#[expect(clippy::missing_errors_doc)]
pub fn check_bounds<E>(size: u64, offset: u64, length: u64) -> Result<(), MediumError<E>> {
    let max_offset = offset.checked_add(length).ok_or(MediumError::BoundsError {
        offset,
        length,
        size,
    })?;
    if max_offset > size {
        return Err(MediumError::BoundsError {
            offset,
            length,
            size,
        });
    }

    Ok(())
}

/// Utility function to centralize [`Medium`] bounds checking.
#[expect(clippy::missing_errors_doc)]
pub fn check_bounds_usize<E>(size: u64, offset: u64, length: usize) -> Result<(), MediumError<E>> {
    check_bounds(size, offset, usize_to_u64(length))
}

impl Medium for [u8] {
    type Error = core::convert::Infallible;

    fn size(&self) -> u64 {
        usize_to_u64(self.len())
    }

    fn read_slice(&self, offset: u64, slice: &mut [u8]) -> Result<(), MediumError<Self::Error>> {
        check_bounds_usize(self.size(), offset, slice.len())?;

        // The requested read region fits within a `usize`, since the bounds checking succeeded
        // and the upper bound is a `usize`.
        slice.copy_from_slice(&self[u64_to_usize(offset)..][..slice.len()]);
        Ok(())
    }
}

impl BackedMedium for [u8] {
    fn access_slice(&self, offset: u64, length: u64) -> Result<&[u8], MediumError<Self::Error>> {
        check_bounds(self.size(), offset, length)?;

        // The requested read region fits within a `usize`, since the bounds checking succeeded
        // and the upper bound is a `usize`.
        Ok(&self[u64_to_usize(offset)..][..u64_to_usize(length)])
    }
}

impl<M: Medium> Medium for &M {
    type Error = M::Error;

    fn size(&self) -> u64 {
        M::size(*self)
    }

    fn read_byte(&self, offset: u64) -> Result<u8, MediumError<Self::Error>> {
        M::read_byte(*self, offset)
    }

    fn read_slice(&self, offset: u64, slice: &mut [u8]) -> Result<(), MediumError<Self::Error>> {
        M::read_slice(*self, offset, slice)
    }
}

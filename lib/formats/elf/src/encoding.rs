//! Encoding aware reading.

use core::{error, fmt};

use crate::ident;

/// An [`Encoding`] provides an API for retreiving decoded values from a [`Medium`].
pub trait Encoding: Clone + Copy {
    /// Returns the [`Encoding`] instance that corresponds with the given [`ident::Encoding`].
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedEncodingError`] if the [`ident::Encoding`] is not supported by this
    /// [`Encoding`] type.
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError>;

    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_u8<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u8;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_u16<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u16;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_u32<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u32;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_u64<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u64;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_i32<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> i32;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    ///
    /// # Panics
    ///
    /// Panics if any error ocurrs, whether that be an arithmetic error or an error retreiving
    /// bytes.
    fn parse_i64<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> i64;
}

/// An error that occurs when the code does not support a particular [`Encoding`]
/// object.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnsupportedEncodingError(ident::Encoding);

impl fmt::Display for UnsupportedEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ident::Encoding::NONE => write!(f, "no data encoding ELF parsing not supported"),
            ident::Encoding::LSB2 => {
                write!(f, "two's complement little-endian parsing not supported")
            }
            ident::Encoding::MSB2 => write!(f, "two's complement big-endian parsing not supported"),
            ident::Encoding(encoding) => {
                write!(f, "unknown data encoding({encoding}) not supported")
            }
        }
    }
}

impl error::Error for UnsupportedEncodingError {}

/// Generates parsing functions for various encodings.
macro_rules! setup_func {
    ($kind:ident, $func:ident, $convert:ident) => {
        fn $func<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> $kind {
            let converted_size = TryInto::<u64>::try_into(core::mem::size_of::<$kind>())
                .expect("`size` was too large");
            let byte_after = offset
                .checked_add(converted_size.saturating_sub(1))
                .expect("`offset + size` overflowed");
            if byte_after >= medium.size() {
                if core::mem::size_of::<$kind>() != 1 {
                    panic!(
                        "attempted read of {} bytes at an offset of {} bytes from medium of {} \
                        bytes",
                        core::mem::size_of::<$kind>(),
                        offset,
                        medium.size(),
                    )
                } else {
                    panic!(
                        "attempted read of 1 byte at an offset of {} bytes from medium of {} \
                        bytes",
                        offset,
                        medium.size(),
                    )
                }
            }

            let mut data = [0; core::mem::size_of::<$kind>()];
            medium
                .read_array(offset, &mut data)
                .expect("in-bounds read from medium failed");
            $kind::$convert(data)
        }
    };
}

/// An object offering methods for safe parsing of unaligned big or little endian integers.
pub type AnyEndian = Merge<LittleEndian, BigEndian>;

/// A zero-sized object offering methods for safe parsing of unaligned little-endian integer
/// parsing.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LittleEndian;

impl Encoding for LittleEndian {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        if encoding != ident::Encoding::LSB2 {
            return Err(UnsupportedEncodingError(encoding));
        }

        Ok(Self)
    }

    setup_func!(u8, parse_u8, from_le_bytes);
    setup_func!(u16, parse_u16, from_le_bytes);
    setup_func!(u32, parse_u32, from_le_bytes);
    setup_func!(u64, parse_u64, from_le_bytes);
    setup_func!(i32, parse_i32, from_le_bytes);
    setup_func!(i64, parse_i64, from_le_bytes);
}

/// A zero-sized object offering methods for safe parsing of unaligned big-endian integer
/// parsing.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigEndian;

impl Encoding for BigEndian {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        if encoding != ident::Encoding::MSB2 {
            return Err(UnsupportedEncodingError(encoding));
        }

        Ok(Self)
    }

    setup_func!(u8, parse_u8, from_be_bytes);
    setup_func!(u16, parse_u16, from_be_bytes);
    setup_func!(u32, parse_u32, from_be_bytes);
    setup_func!(u64, parse_u64, from_be_bytes);
    setup_func!(i32, parse_i32, from_be_bytes);
    setup_func!(i64, parse_i64, from_be_bytes);
}

/// An object used to dispatch the [`Encoding`] to the two underlying [`Encoding`]
/// implementations.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Merge<A: Encoding, B: Encoding> {
    /// The first [`Encoding`] implementation.
    A(A),
    /// The second [`Encoding`] implementation.
    B(B),
}

impl<A: Encoding, B: Encoding> Encoding for Merge<A, B> {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        if let Ok(a) = A::from_elf_encoding(encoding) {
            return Ok(Self::A(a));
        }

        B::from_elf_encoding(encoding).map(Self::B)
    }

    fn parse_u8<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u8 {
        match self {
            Self::A(a) => a.parse_u8(offset, medium),
            Self::B(b) => b.parse_u8(offset, medium),
        }
    }

    fn parse_u16<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u16 {
        match self {
            Self::A(a) => a.parse_u16(offset, medium),
            Self::B(b) => b.parse_u16(offset, medium),
        }
    }

    fn parse_u32<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u32 {
        match self {
            Self::A(a) => a.parse_u32(offset, medium),
            Self::B(b) => b.parse_u32(offset, medium),
        }
    }

    fn parse_u64<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> u64 {
        match self {
            Self::A(a) => a.parse_u64(offset, medium),
            Self::B(b) => b.parse_u64(offset, medium),
        }
    }

    fn parse_i32<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> i32 {
        match self {
            Self::A(a) => a.parse_i32(offset, medium),
            Self::B(b) => b.parse_i32(offset, medium),
        }
    }

    fn parse_i64<M: Medium + ?Sized>(self, offset: u64, medium: &M) -> i64 {
        match self {
            Self::A(a) => a.parse_i64(offset, medium),
            Self::B(b) => b.parse_i64(offset, medium),
        }
    }
}

/// A [`Medium`] provides an API for retreiving bytes.
pub trait Medium {
    /// The number of bytes available to be retreived.
    fn size(&self) -> u64;
    /// Read a single byte from `offset`.
    fn read_byte(&self, offset: u64) -> Option<u8>;
    /// Read `slice.len()` bytes into `slice` from `offset`.
    fn read_slice(&self, offset: u64, slice: &mut [u8]) -> Option<()>;
    /// Read `array.len()` bytes into `array` from `offset`.
    fn read_array<const N: usize>(&self, offset: u64, array: &mut [u8; N]) -> Option<()>;
}

impl Medium for [u8] {
    fn size(&self) -> u64 {
        TryInto::try_into(self.len()).expect("slice is larger than medium allows")
    }

    fn read_byte(&self, offset: u64) -> Option<u8> {
        let offset = TryInto::<usize>::try_into(offset).ok()?;
        self.get(offset).copied()
    }

    fn read_slice(&self, offset: u64, slice: &mut [u8]) -> Option<()> {
        let offset = TryInto::<usize>::try_into(offset).ok()?;
        let bytes = self.get(offset..).and_then(|arr| arr.get(..slice.len()))?;
        slice.copy_from_slice(bytes);
        Some(())
    }

    fn read_array<const N: usize>(&self, offset: u64, array: &mut [u8; N]) -> Option<()> {
        let offset = TryInto::<usize>::try_into(offset).ok()?;
        let bytes = self.get(offset..).and_then(|arr| arr.first_chunk::<N>())?;
        array.copy_from_slice(bytes);
        Some(())
    }
}

/// A [`BackedMedium`] provides an API to provide access to slices of the [`Medium`]'s data.
pub trait BackedMedium: Medium {
    /// Accesses a slice of `length` bytes at `offset` into the [`BackedMedium`].
    fn access_slice(&self, offset: u64, length: u64) -> Option<&[u8]>;
}

impl BackedMedium for [u8] {
    fn access_slice(&self, offset: u64, length: u64) -> Option<&[u8]> {
        let offset = TryInto::<usize>::try_into(offset).ok()?;
        let length = TryInto::<usize>::try_into(length).ok()?;
        self.get(offset..).and_then(|arr| arr.get(..length))
    }
}

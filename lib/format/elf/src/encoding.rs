//! Encoding aware reading.

use core::{error, fmt};

use crate::{
    ident,
    medium::{Medium, MediumError},
};

/// An [`Encoding`] provides an API for retrieving decoded values from a [`Medium`].
///
/// This defines how values are decoded from raw bytes (e.g., endianness and signedness).
///
/// # Errors
///
/// All read methods return a [`MediumError`] when the underlying [`Medium`] cannot provide the
/// requested data.
///
/// # Implementors
///
/// - [`Encoding`] must be stateless.
/// - Unaligned reads must be allowed.
#[expect(
    clippy::missing_errors_doc,
    reason = "errors are documented in trait implementation"
)]
pub trait Encoding: Copy {
    /// Returns the [`Encoding`] instance that corresponds with the given [`ident::Encoding`].
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedEncodingError`] if the [`ident::Encoding`] is not supported by this
    /// [`Encoding`] type.
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError>;

    /// Reads the `u8` at `offset` bytes into the `medium`.
    fn read_u8<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u8, MediumError<M::Error>>;
    /// Reads the `u16` at `offset` bytes into the `medium`.
    fn read_u16<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u16, MediumError<M::Error>>;
    /// Reads the `u32` at `offset` bytes into the `medium`.
    fn read_u32<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u32, MediumError<M::Error>>;
    /// Reads the `u64` at `offset` bytes into the `medium`.
    fn read_u64<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u64, MediumError<M::Error>>;
    /// Reads the `i8` at `offset` bytes into the `medium`.
    fn read_i8<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i8, MediumError<M::Error>>;
    /// Reads the `i16` at `offset` bytes into the `medium`.
    fn read_i16<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i16, MediumError<M::Error>>;
    /// Reads the `i32` at `offset` bytes into the `medium`.
    fn read_i32<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i32, MediumError<M::Error>>;
    /// Reads the `i64` at `offset` bytes into the `medium`.
    fn read_i64<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i64, MediumError<M::Error>>;
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
    ($func:ident, $kind:ident, $convert:ident) => {
        fn $func<M: Medium + ?Sized>(
            self,
            offset: u64,
            medium: &M,
        ) -> Result<$kind, MediumError<M::Error>> {
            // Size of the generic array is interpreted from the `convert` function.
            read_array(medium, offset).map(|arr| $kind::$convert(arr))
        }
    };
}

/// A zero-sized object offering methods for safe parsing of unaligned little-endian integers.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LittleEndian;

impl Encoding for LittleEndian {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        if encoding != ident::Encoding::LSB2 {
            return Err(UnsupportedEncodingError(encoding));
        }

        Ok(Self)
    }

    setup_func!(read_u8, u8, from_le_bytes);
    setup_func!(read_u16, u16, from_le_bytes);
    setup_func!(read_u32, u32, from_le_bytes);
    setup_func!(read_u64, u64, from_le_bytes);

    setup_func!(read_i8, i8, from_le_bytes);
    setup_func!(read_i16, i16, from_le_bytes);
    setup_func!(read_i32, i32, from_le_bytes);
    setup_func!(read_i64, i64, from_le_bytes);
}

/// A zero-sized object offering methods for safe parsing of unaligned big-endian integers.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigEndian;

impl Encoding for BigEndian {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        if encoding != ident::Encoding::MSB2 {
            return Err(UnsupportedEncodingError(encoding));
        }

        Ok(Self)
    }

    setup_func!(read_u8, u8, from_be_bytes);
    setup_func!(read_u16, u16, from_be_bytes);
    setup_func!(read_u32, u32, from_be_bytes);
    setup_func!(read_u64, u64, from_be_bytes);

    setup_func!(read_i8, i8, from_be_bytes);
    setup_func!(read_i16, i16, from_be_bytes);
    setup_func!(read_i32, i32, from_be_bytes);
    setup_func!(read_i64, i64, from_be_bytes);
}

/// A zero-sized object offering methods for safe parsing of unaligned little-endian and big-endian
/// integers.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnyEndian {
    /// Integers are in little-endian.
    LittleEndian,
    /// Integers are in big-endian.
    BigEndian,
}

impl Encoding for AnyEndian {
    fn from_elf_encoding(encoding: ident::Encoding) -> Result<Self, UnsupportedEncodingError> {
        match encoding {
            ident::Encoding::LSB2 => Ok(AnyEndian::LittleEndian),
            ident::Encoding::MSB2 => Ok(AnyEndian::BigEndian),
            encoding => Err(UnsupportedEncodingError(encoding)),
        }
    }

    fn read_u8<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u8, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_u8(offset, medium),
            Self::BigEndian => BigEndian.read_u8(offset, medium),
        }
    }

    fn read_u16<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u16, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_u16(offset, medium),
            Self::BigEndian => BigEndian.read_u16(offset, medium),
        }
    }

    fn read_u32<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u32, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_u32(offset, medium),
            Self::BigEndian => BigEndian.read_u32(offset, medium),
        }
    }

    fn read_u64<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<u64, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_u64(offset, medium),
            Self::BigEndian => BigEndian.read_u64(offset, medium),
        }
    }

    fn read_i8<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i8, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_i8(offset, medium),
            Self::BigEndian => BigEndian.read_i8(offset, medium),
        }
    }

    fn read_i16<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i16, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_i16(offset, medium),
            Self::BigEndian => BigEndian.read_i16(offset, medium),
        }
    }

    fn read_i32<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i32, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_i32(offset, medium),
            Self::BigEndian => BigEndian.read_i32(offset, medium),
        }
    }

    fn read_i64<M: Medium + ?Sized>(
        self,
        offset: u64,
        medium: &M,
    ) -> Result<i64, MediumError<M::Error>> {
        match self {
            Self::LittleEndian => LittleEndian.read_i64(offset, medium),
            Self::BigEndian => BigEndian.read_i64(offset, medium),
        }
    }
}

/// Performs an exact-length read.
fn read_array<M: Medium<Error = E> + ?Sized, E, const N: usize>(
    medium: &M,
    offset: u64,
) -> Result<[u8; N], MediumError<E>> {
    let mut arr = [0; N];
    medium.read_slice(offset, &mut arr)?;
    Ok(arr)
}

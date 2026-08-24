//! Ergonomic wrapper over [`ElfIdent`][raw::ElfIdent`].

use core::{error, fmt, mem};

use conversion::usize_to_u64;

use crate::{
    extract_format,
    medium::{Medium, MediumError},
    raw,
};

/// Contains basic information about an ELF file that can be obtained in an architecture
/// independent manner.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ElfIdent<'slice, M: ?Sized>(&'slice M);

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized> ElfIdent<'slice, M> {
    /// The magic bytes that identify the start of an ELf file.
    pub const MAGIC_BYTES: [u8; 4] = [0x7F, b'E', b'L', b'F'];
    /// The current version of the ELF file header.
    pub const CURRENT_HEADER_VERSION: u8 = 1;

    /// Creates a new [`ElfIdent`] from the given [`Medium`].
    ///
    /// # Errors
    ///
    /// Returns [`MediumError`] if the [`Medium`] is not large enough to contain the [`ElfIdent`].
    pub fn new(medium: &'slice M) -> Result<Self, MediumError<M::Error>> {
        if medium.size() < usize_to_u64(mem::size_of::<raw::ElfIdent>()) {
            // If `medium` isn't large enough to contain an [`ElfHeader`], then return a
            // [`MediumError::BoundsError`].
            return Err(MediumError::BoundsError {
                offset: 0,
                length: usize_to_u64(mem::size_of::<raw::ElfIdent>()),
                size: medium.size(),
            });
        }

        Ok(Self(medium))
    }

    /// Validates that the [`ElfIdent`] matches the ELF specification.
    ///
    /// # Errors
    ///
    /// - [`ElfIdentValidationError::InvalidMagicBytes`]: Returned when the magic bytes are not
    ///   correct.
    /// - [`ElfIdentValidationError::UnsupportedElfHeaderVersion`]: Returned when the ELF header
    ///   version is not supported.
    /// - [`ElfIdentValidationError::MediumError`]: Returned when an error occurs while interacting
    ///   with the underlying [`Medium`].
    pub fn validate(&self) -> Result<(), ElfIdentValidationError<M::Error>> {
        let magic = self.magic()?;
        if magic != Self::MAGIC_BYTES {
            return Err(ElfIdentValidationError::InvalidMagicBytes(magic));
        }

        let version = self.version()?;
        if version != Self::CURRENT_HEADER_VERSION {
            return Err(ElfIdentValidationError::UnsupportedElfHeaderVersion(
                version,
            ));
        }

        Ok(())
    }

    /// Returns the magic bytes that identify this file as an ELF file.
    pub fn magic(&self) -> Result<[u8; 4], MediumError<M::Error>> {
        let mut arr = [0; 4];
        self.0
            .read_slice(
                usize_to_u64(mem::offset_of!(raw::ElfIdent, magic)),
                &mut arr,
            )
            .map(|()| arr)
    }

    /// Returns the [`Class`] of this ELF file.
    pub fn class(&self) -> Result<Class, MediumError<M::Error>> {
        self.0
            .read_byte(usize_to_u64(mem::offset_of!(raw::ElfIdent, class)))
            .map(Class)
    }

    /// Returns the [`Encoding`] of this ELF file.
    pub fn encoding(&self) -> Result<Encoding, MediumError<M::Error>> {
        self.0
            .read_byte(usize_to_u64(mem::offset_of!(raw::ElfIdent, encoding)))
            .map(Encoding)
    }

    /// Returns the version of the ELF file identifier.
    pub fn version(&self) -> Result<u8, MediumError<M::Error>> {
        self.0
            .read_byte(usize_to_u64(mem::offset_of!(raw::ElfIdent, version)))
    }

    /// Returns the [`OsAbi`] of the ELF file.
    pub fn os_abi(&self) -> Result<OsAbi, MediumError<M::Error>> {
        self.0
            .read_byte(usize_to_u64(mem::offset_of!(raw::ElfIdent, os_abi)))
            .map(OsAbi)
    }

    /// Returns the version of the ABI to which the object is targeted.
    pub fn abi_version(&self) -> Result<u8, MediumError<M::Error>> {
        self.0
            .read_byte(usize_to_u64(mem::offset_of!(raw::ElfIdent, abi_version)))
    }

    /// Returns the padding bytes of this [`ElfIdent`].
    pub fn padding(&self) -> Result<[u8; 7], MediumError<M::Error>> {
        let mut arr = [0; 7];
        self.0
            .read_slice(usize_to_u64(mem::offset_of!(raw::ElfIdent, pad)), &mut arr)
            .map(|()| arr)
    }

    /// Returns the underlying [`Medium`].
    pub fn medium(&self) -> &M {
        self.0
    }
}

impl<'slice, M: Medium + ?Sized> core::fmt::Debug for ElfIdent<'slice, M>
where
    <M as Medium>::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let magic = self.magic();
        let class = self.class();
        let encoding = self.encoding();
        let version = self.version();
        let os_abi = self.os_abi();
        let abi_version = self.abi_version();

        f.debug_struct("ElfIdent")
            .field("magic", extract_format(&magic))
            .field("class", extract_format(&class))
            .field("encoding", extract_format(&encoding))
            .field("version", extract_format(&version))
            .field("os_abi", extract_format(&os_abi))
            .field("abi_version", extract_format(&abi_version))
            .finish()
    }
}

/// Various errors that can occur when validating an [`ElfIdent`] follows the ELF specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfIdentValidationError<E> {
    /// The given slice has invalid magic bytes.
    InvalidMagicBytes([u8; 4]),
    /// The ELF header version is not supported.
    UnsupportedElfHeaderVersion(u8),
    /// Various errors that can occur when accessing the underlying [`Medium`].
    MediumError(MediumError<E>),
}

impl<E> From<MediumError<E>> for ElfIdentValidationError<E> {
    fn from(value: MediumError<E>) -> Self {
        Self::MediumError(value)
    }
}

impl<E: fmt::Display> fmt::Display for ElfIdentValidationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagicBytes(bytes) => write!(f, "invalid magic bytes: {bytes:x?}"),
            Self::UnsupportedElfHeaderVersion(version) => {
                write!(f, "unsupported ELf header version: {version}")
            }
            Self::MediumError(error) => write!(f, "error accessing ELF ident bytes: {error}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for ElfIdentValidationError<E> {}

/// Specifier of the ELF file class, which determines the sizing
/// of various items in the ELF file format.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Class(pub u8);

impl Class {
    /// Invalid [`Class`] specifier.
    pub const NONE: Self = Self(0);
    /// ELF file is formatted in its 32-bit format.
    pub const CLASS32: Self = Self(1);
    /// ELF file is formatted in its 64-bit format.
    pub const CLASS64: Self = Self(2);
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("Invalid"),
            Self::CLASS32 => f.pad("Class32"),
            Self::CLASS64 => f.pad("Class64"),
            class => f.debug_tuple("Class").field(&class.0).finish(),
        }
    }
}

/// Specifier of the ELF file data encoding, which determines the encoding
/// of both the data structures used by the ELF file format and data contained
/// in the object file sections.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Encoding(pub u8);

impl Encoding {
    /// Invalid [`Encoding`] specifier.
    pub const NONE: Self = Self(0);
    /// The encoding of the ELF file format uses little endian
    /// two's complement integers.
    pub const LSB2: Self = Self(1);
    /// The encoding of the ELF file format uses big endian
    /// two's complement integers.
    pub const MSB2: Self = Self(2);
}

impl fmt::Debug for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("NoEncoding"),
            Self::LSB2 => f.pad("LittleEndian"),
            Self::MSB2 => f.pad("BigEndian"),
            encoding => f.debug_tuple("Encoding").field(&encoding.0).finish(),
        }
    }
}

/// Specifier of the OS or ABI specific ELF extensions used by this file.
///
/// This field determines the interpretation of various OS or ABI specific values.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OsAbi(pub u8);

impl OsAbi {
    /// No extensions or unspecified extensions.
    pub const NONE: Self = Self(0);
}

impl fmt::Debug for OsAbi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NONE => f.pad("None"),
            os_abi => f.debug_tuple("OsAbi").field(&os_abi.0).finish(),
        }
    }
}

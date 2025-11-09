//! Ergonomic wrapper over [`raw::ElfIdent`].

use core::{error, fmt, mem};

use crate::{LittleEndian, Medium, raw};

/// Contains basic information about an ELF file that can be obtained in an architecture
/// independent manner.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct FileIdent<'slice, M: Medium + ?Sized>(pub(crate) &'slice M);

impl<'slice, M: Medium + ?Sized> FileIdent<'slice, M> {
    /// Creates a new [`FileIdent`] from the given [`Medium`].
    ///
    /// Returns [`None`] if `medium.size()` is too small to contain an [`FileIdent`].
    pub fn new(medium: &'slice M) -> Option<Self> {
        if medium.size() >= mem::size_of::<raw::ElfIdent>() as u64 {
            Some(Self(medium))
        } else {
            None
        }
    }

    /// Validates that this [`FileIdent`] matches the ELF specification and is supported by this
    /// crate.
    ///
    /// # Errors
    /// - [`ValidateFileIdentSpecError::InvalidMagicBytes`]: Returned when this [`FileIdent`]'s magic
    ///   bytes are invalid.
    /// - [`ValidateFileIdentSpecError::UnsupportedElfHeaderVersion`]: Returns when the ELF header
    ///   version is not supported.
    /// - [`ValidateFileIdentSpecError::NonZeroPadding`]: Returned when the padding of the
    ///   [`FileIdent`] is non-zero.
    pub fn validate_spec(&self) -> Result<(), ValidateFileIdentSpecError> {
        if self.magic() != Self::MAGIC_BYTES {
            return Err(ValidateFileIdentSpecError::InvalidMagicBytes(self.magic()));
        }

        if self.version() != Self::CURRENT_HEADER_VERSION {
            return Err(ValidateFileIdentSpecError::UnsupportedElfHeaderVersion(
                self.version(),
            ));
        }

        if self.padding().into_iter().any(|val| val != 0) {
            return Err(ValidateFileIdentSpecError::NonZeroPadding(self.padding()));
        }

        Ok(())
    }

    /// The magic bytes that identify the start of an ELf file.
    pub const MAGIC_BYTES: [u8; 4] = [0x7F, b'E', b'L', b'F'];

    /// The current version of the ELF file header.
    pub const CURRENT_HEADER_VERSION: u8 = 1;

    /// Returns the magic bytes that identify this file as an ELF file.
    pub fn magic(&self) -> [u8; 4] {
        let mut arr = [0; 4];
        self.0
            .read_array(mem::offset_of!(raw::ElfIdent, magic) as u64, &mut arr);
        arr
    }

    /// Returns the [`Class`] of this ELF file.
    pub fn class(&self) -> Class {
        Class(crate::Encoding::parse_u8(
            LittleEndian,
            mem::offset_of!(raw::ElfIdent, class) as u64,
            self.0,
        ))
    }

    /// Returns the [`Encoding`] of this ELF file.
    pub fn encoding(&self) -> Encoding {
        Encoding(crate::Encoding::parse_u8(
            LittleEndian,
            mem::offset_of!(raw::ElfIdent, encoding) as u64,
            self.0,
        ))
    }

    /// Returns the version of the ELF file identifier.
    pub fn version(&self) -> u8 {
        crate::Encoding::parse_u8(
            LittleEndian,
            mem::offset_of!(raw::ElfIdent, version) as u64,
            self.0,
        )
    }

    /// Returns the [`OsAbi`] of the ELF file.
    pub fn os_abi(&self) -> OsAbi {
        OsAbi(crate::Encoding::parse_u8(
            LittleEndian,
            mem::offset_of!(raw::ElfIdent, os_abi) as u64,
            self.0,
        ))
    }

    /// Returns the version of the ABI to which the object is targeted.
    pub fn abi_version(&self) -> u8 {
        crate::Encoding::parse_u8(
            LittleEndian,
            mem::offset_of!(raw::ElfIdent, abi_version) as u64,
            self.0,
        )
    }

    /// Returns the padding bytes of this [`FileIdent`].
    pub fn padding(&self) -> [u8; 7] {
        let mut arr = [0; 7];
        self.0
            .read_array(mem::offset_of!(raw::ElfIdent, pad) as u64, &mut arr);
        arr
    }
}

impl<M: Medium + ?Sized> core::fmt::Debug for FileIdent<'_, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug_struct = f.debug_struct("FileIdent");

        debug_struct.field("magic", &self.magic());
        debug_struct.field("class", &self.class());
        debug_struct.field("encoding", &self.encoding());
        debug_struct.field("version", &self.version());
        debug_struct.field("os_abi", &self.os_abi());
        debug_struct.field("abi_version", &self.abi_version());

        debug_struct.finish()
    }
}

/// Various errors that can occur when validating an [`FileIdent`] follows the ELF specification and
/// is supported by this crate.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidateFileIdentSpecError {
    /// The given slice has invalid magic bytes.
    InvalidMagicBytes([u8; 4]),
    /// The ELF header verison is not unsupported.
    UnsupportedElfHeaderVersion(u8),
    /// The padding of the [`FileIdent`] is non-zero.
    NonZeroPadding([u8; 7]),
}

impl fmt::Display for ValidateFileIdentSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagicBytes(bytes) => write!(f, "invalid magic bytes: {bytes:X?}",),
            Self::UnsupportedElfHeaderVersion(version) => {
                write!(f, "invalid ELF header version: {version}")
            }
            Self::NonZeroPadding(padding) => write!(f, "non-zero padding: {padding:X?}"),
        }
    }
}

impl error::Error for ValidateFileIdentSpecError {}

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

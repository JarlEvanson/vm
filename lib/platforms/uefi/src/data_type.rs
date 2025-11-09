//! Definitions of common data type.
//!
//! This module defines the common data types that are used throughout the UEFI specification.

use core::{
    error, ffi,
    fmt::{self, Debug},
};

/// Logical boolean.
///
/// Should be either [`Boolean::FALSE`] or [`Boolean::TRUE`]. Other values are undefined.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Boolean(pub u8);

impl Boolean {
    /// Corresponds to [`false`].
    pub const FALSE: Self = Self(0);
    /// Corresponds to [`true`].
    pub const TRUE: Self = Self(1);
}

/// An ISO-Latin-1 character.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Char8(pub u8);

/// An UCS-2 character.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Char16(pub u16);

impl Char16 {
    /// Returns `true` if the [`Char16`] is a valid character according to the UCS-2 specification.
    pub const fn valid(self) -> bool {
        self.0 <= 0xD7FF || self.0 >= 0xE000
    }
}

/// 128-bit buffer containing a unique identifier value. Unless otherwise specified,
/// aligned on a 64-bit boundary.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Guid {
    /// The little-endian low field of the timestamp.
    pub time_low: u32,
    /// The little-endian middle field of the timestamp.
    pub time_mid: [u8; 2],
    /// The little-endian middle field of the timestamp and the version number.
    pub time_high_version: [u8; 2],
    /// The high field of the clock sequence and reserved.
    pub clock_seq_high_reserved: u8,
    /// The low field of the clock sequence.
    pub clock_seq_low: u8,
    /// The spatially unique node identifier.
    pub node: [u8; 6],
}

impl Guid {
    /// Parses a [`Guid`] from the given [`str`].
    ///
    /// # Errors
    /// - [`ParseGuidError::InvalidLength`] if the given [`str`] is not exactly 36 bytes long.
    /// - [`ParseGuidError::InvalidSeperator`] if the given [`str`] has invalid seperator
    ///   characters.
    /// - [`ParseGuidError::InvalidHexCharacter`] if the given [`str`] has invalid hex characters.
    pub const fn try_parse(s: &str) -> Result<Self, ParseGuidError> {
        let bytes = s.as_bytes();

        if bytes.len() != 36 {
            return Err(ParseGuidError::InvalidLength);
        }

        let seperator_positions = [8, 13, 18, 23];
        let mut index = 0;
        while index < seperator_positions.len() {
            let position = seperator_positions[index];
            if bytes[position] != b'-' {
                return Err(ParseGuidError::InvalidSeperator {
                    c: bytes[position],
                    position,
                });
            }

            index += 1;
        }

        let time_low = {
            let byte_3 = match Self::parse_from_ascii_bytes(bytes, 0) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_2 = match Self::parse_from_ascii_bytes(bytes, 2) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_1 = match Self::parse_from_ascii_bytes(bytes, 4) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_0 = match Self::parse_from_ascii_bytes(bytes, 6) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };

            u32::from_ne_bytes([byte_0, byte_1, byte_2, byte_3])
        };

        let time_mid = {
            let byte_1 = match Self::parse_from_ascii_bytes(bytes, 9) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_0 = match Self::parse_from_ascii_bytes(bytes, 11) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };

            [byte_0, byte_1]
        };

        let time_high_version = {
            let byte_1 = match Self::parse_from_ascii_bytes(bytes, 14) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_0 = match Self::parse_from_ascii_bytes(bytes, 16) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };

            [byte_0, byte_1]
        };

        let clock_seq_high_reserved = match Self::parse_from_ascii_bytes(bytes, 19) {
            Ok(byte) => byte,
            Err(error) => return Err(error),
        };

        let clock_seq_low = match Self::parse_from_ascii_bytes(bytes, 21) {
            Ok(byte) => byte,
            Err(error) => return Err(error),
        };

        let node = {
            let byte_0 = match Self::parse_from_ascii_bytes(bytes, 24) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_1 = match Self::parse_from_ascii_bytes(bytes, 26) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_2 = match Self::parse_from_ascii_bytes(bytes, 28) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_3 = match Self::parse_from_ascii_bytes(bytes, 30) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_4 = match Self::parse_from_ascii_bytes(bytes, 32) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };
            let byte_5 = match Self::parse_from_ascii_bytes(bytes, 34) {
                Ok(byte) => byte,
                Err(error) => return Err(error),
            };

            [byte_0, byte_1, byte_2, byte_3, byte_4, byte_5]
        };

        let guid = Self {
            time_low,
            time_mid,
            time_high_version,
            clock_seq_high_reserved,
            clock_seq_low,
            node,
        };

        Ok(guid)
    }

    /// Converts this [`Guid`] to its string representation with lowercase hex characters.
    pub const fn to_str_lower(&self) -> [u8; 36] {
        let mut str = [b'-'; 36];

        (str[0], str[1]) = Self::to_hex_lower(self.time_low.to_ne_bytes()[3]);
        (str[2], str[3]) = Self::to_hex_lower(self.time_low.to_ne_bytes()[2]);
        (str[4], str[5]) = Self::to_hex_lower(self.time_low.to_ne_bytes()[1]);
        (str[6], str[7]) = Self::to_hex_lower(self.time_low.to_ne_bytes()[0]);

        (str[9], str[10]) = Self::to_hex_lower(self.time_mid[1]);
        (str[11], str[12]) = Self::to_hex_lower(self.time_mid[0]);

        (str[14], str[15]) = Self::to_hex_lower(self.time_high_version[1]);
        (str[16], str[17]) = Self::to_hex_lower(self.time_high_version[0]);

        (str[19], str[20]) = Self::to_hex_lower(self.clock_seq_high_reserved);
        (str[21], str[22]) = Self::to_hex_lower(self.clock_seq_low);

        (str[24], str[25]) = Self::to_hex_lower(self.node[0]);
        (str[26], str[27]) = Self::to_hex_lower(self.node[1]);
        (str[28], str[29]) = Self::to_hex_lower(self.node[2]);
        (str[30], str[31]) = Self::to_hex_lower(self.node[3]);
        (str[32], str[33]) = Self::to_hex_lower(self.node[4]);
        (str[34], str[35]) = Self::to_hex_lower(self.node[5]);

        str
    }

    /// Converts this [`Guid`] to its string representation with uppercase hex characters.
    pub const fn to_str_upper(&self) -> [u8; 36] {
        let mut str = [b'-'; 36];

        (str[0], str[1]) = Self::to_hex_upper(self.time_low.to_ne_bytes()[3]);
        (str[2], str[3]) = Self::to_hex_upper(self.time_low.to_ne_bytes()[2]);
        (str[4], str[5]) = Self::to_hex_upper(self.time_low.to_ne_bytes()[1]);
        (str[6], str[7]) = Self::to_hex_upper(self.time_low.to_ne_bytes()[0]);

        (str[9], str[10]) = Self::to_hex_upper(self.time_mid[1]);
        (str[11], str[12]) = Self::to_hex_upper(self.time_mid[0]);

        (str[14], str[15]) = Self::to_hex_upper(self.time_high_version[1]);
        (str[16], str[17]) = Self::to_hex_upper(self.time_high_version[0]);

        (str[19], str[20]) = Self::to_hex_upper(self.clock_seq_high_reserved);
        (str[21], str[22]) = Self::to_hex_upper(self.clock_seq_low);

        (str[24], str[25]) = Self::to_hex_upper(self.node[0]);
        (str[26], str[27]) = Self::to_hex_upper(self.node[1]);
        (str[28], str[29]) = Self::to_hex_upper(self.node[2]);
        (str[30], str[31]) = Self::to_hex_upper(self.node[3]);
        (str[32], str[33]) = Self::to_hex_upper(self.node[4]);
        (str[34], str[35]) = Self::to_hex_upper(self.node[5]);

        str
    }

    /// Retrieves the byte represented by the two hex characters located at index `position`.
    const fn parse_from_ascii_bytes(bytes: &[u8], position: usize) -> Result<u8, ParseGuidError> {
        let low_nibble = bytes[position + 1];
        let low_nibble = match Self::parse_nibble_from_ascii(low_nibble) {
            Ok(nibble) => nibble,
            Err(c) => return Err(ParseGuidError::InvalidHexCharacter { c, position }),
        };

        let high_nibble = bytes[position];
        let high_nibble = match Self::parse_nibble_from_ascii(high_nibble) {
            Ok(nibble) => nibble,
            Err(c) => {
                return Err(ParseGuidError::InvalidHexCharacter {
                    c,
                    position: position + 1,
                });
            }
        };

        Ok((high_nibble << 4) | low_nibble)
    }

    /// Retrives the nibble that a hex character represents if successful. Otherwise returns the
    /// failing character.
    const fn parse_nibble_from_ascii(byte: u8) -> Result<u8, u8> {
        let nibble = match byte {
            b'0' => 0x0,
            b'1' => 0x1,
            b'2' => 0x2,
            b'3' => 0x3,
            b'4' => 0x4,
            b'5' => 0x5,
            b'6' => 0x6,
            b'7' => 0x7,
            b'8' => 0x8,
            b'9' => 0x9,
            b'a' | b'A' => 0xA,
            b'b' | b'B' => 0xB,
            b'c' | b'C' => 0xC,
            b'd' | b'D' => 0xD,
            b'e' | b'E' => 0xE,
            b'f' | b'F' => 0xF,
            c => return Err(c),
        };

        Ok(nibble)
    }

    /// Converts a byte into its lowercase hex characters.
    const fn to_hex_lower(byte: u8) -> (u8, u8) {
        (
            Self::nibble_to_hex_lower(byte >> 4),
            Self::nibble_to_hex_lower(byte & 0xF),
        )
    }

    /// Converts a byte into its lowercase hex character.
    const fn nibble_to_hex_lower(nibble: u8) -> u8 {
        match nibble {
            0..=9 => b'0' + nibble,
            0xa..=0xf => b'a' - 10 + nibble,
            _ => unreachable!(),
        }
    }

    /// Converts a byte into its uppercase hex characters.
    const fn to_hex_upper(byte: u8) -> (u8, u8) {
        (
            Self::nibble_to_hex_upper(byte >> 4),
            Self::nibble_to_hex_upper(byte & 0xF),
        )
    }

    /// Converts a byte into its uppercase hex character.
    const fn nibble_to_hex_upper(nibble: u8) -> u8 {
        match nibble {
            0..=9 => b'0' + nibble,
            0xA..=0xF => (b'A' - 10) + nibble,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = self.to_str_lower();
        let str = core::str::from_utf8(&str).unwrap();

        fmt::Display::fmt(&str, f)
    }
}

/// Various errors that could occur while parsing a [`Guid`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ParseGuidError {
    /// The given string has the wrong length, expected 36 bytes.
    InvalidLength,
    /// The input is missing a seperator character (`-`) at index `position`.
    InvalidSeperator {
        /// The invalid seperator character.
        c: u8,
        /// The index at which the invalid seperator character is located.
        position: usize,
    },
    /// The input contains an invalid hex character at index `position`.
    InvalidHexCharacter {
        /// The invalid hex character.
        c: u8,
        /// The index at which the invalid hex character is located.
        position: usize,
    },
}

impl fmt::Display for ParseGuidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidLength => {
                write!(f, "given GUID string has wrong length: expected 36 bytes")
            }
            Self::InvalidSeperator { c, position } => write!(
                f,
                "given GUID string has invalid seperator at {position}: '{}'",
                c as char
            ),
            Self::InvalidHexCharacter { c, position } => write!(
                f,
                "given GUID string has invalid hex character at {position}: '{}'",
                c as char
            ),
        }
    }
}

impl error::Error for ParseGuidError {}

/// Parses a [`Guid`] at panics if an error occurs.
///
/// Intended for use at compile time.
#[macro_export]
macro_rules! guid {
    ($expr:tt) => {{
        let guid = $expr;
        match $crate::data_type::Guid::try_parse(&guid) {
            Ok(guid) => guid,
            Err($crate::data_type::ParseGuidError::InvalidLength) => {
                panic!("provided GUID is of an invalid length")
            }
            Err($crate::data_type::ParseGuidError::InvalidSeperator { c: _, position: _ }) => {
                panic!("given GUID has an invalid seperator character")
            }
            Err($crate::data_type::ParseGuidError::InvalidHexCharacter { c: _, position: _ }) => {
                panic!("given GUID has an invalid hex character")
            }
        }
    }};
}

/// A UEFI status code. UEFI status codes are utilized to report sucesses, warnings, and errors.
#[repr(transparent)]
#[derive(Clone, Copy, Hash, Default, PartialEq, Eq)]
pub struct Status(pub usize);

impl Status {
    /// All [`Self`]es with the [`Self::ERROR_BIT`] set are error codes.
    pub const ERROR_BIT: usize = 1 << (usize::BITS - 1);
    /// All [`Self`]es with the [`Self::OEM_BIT`] set are reserved for use of OEMs.
    pub const OEM_BIT: usize = 1 << (usize::BITS - 2);

    // Success codes

    /// The operation completed successfully.
    pub const SUCCESS: Self = Self(0);

    // Warning codes

    /// The string cotained one or more characters that the device could not render and were
    /// skipped.
    pub const WARN_UNKNOWN_GLYPH: Self = Self(1);
    /// The handle was closed, but the file was not deleted.
    pub const WARN_DELETE_FAILURE: Self = Self(2);
    /// The handle was closed, but the data to the file was not flushed properly.
    pub const WARN_WRITE_FAILURE: Self = Self(3);
    /// The resulting buffer was too small, and the data was truncated to the buffer size.
    pub const WARN_BUFFER_TOO_SMALL: Self = Self(4);
    /// The data has not been updated within the timeframe set by local policy for this type of
    /// data.
    pub const WARN_STALE_DATA: Self = Self(5);
    /// The resulting buffer contains a UEFI-compliant file system.
    pub const WARN_FILE_SYSTEM: Self = Self(6);
    /// The operation will be processed across a system reset.
    pub const WARN_RESET_REQUIRED: Self = Self(7);

    // Error codes

    /// The image failed to load.
    pub const LOAD_ERROR: Self = Self(Self::ERROR_BIT | 1);
    /// A parameter was incorrect.
    pub const INVALID_PARAMETER: Self = Self(Self::ERROR_BIT | 2);
    /// The operation is not supported.
    pub const UNSUPPORTED: Self = Self(Self::ERROR_BIT | 3);
    /// The buffer was not the proper size for the request.
    pub const BAD_BUFFER_SIZE: Self = Self(Self::ERROR_BIT | 4);
    /// The buffer is not large enough to hold the requested data. The required buffer size is
    /// returned in the appropriate parameter when this error occurs.
    pub const BUFFER_TOO_SMALL: Self = Self(Self::ERROR_BIT | 5);
    /// There is no pending data upon return.
    pub const NOT_READY: Self = Self(Self::ERROR_BIT | 6);
    /// The physical device reported an error while attempting the operation.
    pub const DEVICE_ERROR: Self = Self(Self::ERROR_BIT | 7);
    /// The device cannot be written to.
    pub const WRITE_PROTECTED: Self = Self(Self::ERROR_BIT | 8);
    /// A resource has run out.
    pub const OUT_OF_RESOURCES: Self = Self(Self::ERROR_BIT | 9);
    /// An inconsistency was detected on the file system causing the operation to fail.
    pub const VOLUME_CORRUPTED: Self = Self(Self::ERROR_BIT | 10);
    /// There is no more space on the file system.
    pub const VOLUME_FULL: Self = Self(Self::ERROR_BIT | 11);
    /// The device does not contain any medium to perform the operation.
    pub const NO_MEDIA: Self = Self(Self::ERROR_BIT | 12);
    /// The medium in the device has changed since the last access.
    pub const MEDIA_CHANGED: Self = Self(Self::ERROR_BIT | 13);
    /// The item was not found.
    pub const NOT_FOUND: Self = Self(Self::ERROR_BIT | 14);
    /// Access was denied.
    pub const ACCESS_DENIED: Self = Self(Self::ERROR_BIT | 15);
    /// The server was not found or did not respond to the request.
    pub const NO_RESPONSE: Self = Self(Self::ERROR_BIT | 16);
    /// A mapping to a device does not exist.
    pub const NO_MAPPING: Self = Self(Self::ERROR_BIT | 17);
    /// The timeout time expired.
    pub const TIMEOUT: Self = Self(Self::ERROR_BIT | 18);
    /// The protocol has not been started.
    pub const NOT_STARTED: Self = Self(Self::ERROR_BIT | 19);
    /// The protocol has already been started.
    pub const ALREADY_STARTED: Self = Self(Self::ERROR_BIT | 20);
    /// The operation was aborted.
    pub const ABORTED: Self = Self(Self::ERROR_BIT | 21);
    /// An ICMP error occurred during the network operation.
    pub const ICMP_ERROR: Self = Self(Self::ERROR_BIT | 22);
    /// A TFTP error occurred during the network operation.
    pub const TFTP_ERROR: Self = Self(Self::ERROR_BIT | 23);
    /// A protocol error occurred during the network operation.
    pub const PROTOCOL_ERROR: Self = Self(Self::ERROR_BIT | 24);
    /// The operation encountered an internal version that was incompatible with the version
    /// requested by the caller.
    pub const INCOMPATIBLE_VERSION: Self = Self(Self::ERROR_BIT | 25);
    /// The operation was not performed due to a security violation.
    pub const SECURITY_VIOLATION: Self = Self(Self::ERROR_BIT | 26);
    /// A CRC error was detected.
    pub const CRC_ERROR: Self = Self(Self::ERROR_BIT | 27);
    /// Beginning or end of media was reached.
    pub const END_OF_MEDIA: Self = Self(Self::ERROR_BIT | 28);
    /// The end of the file was reached.
    pub const END_OF_FILE: Self = Self(Self::ERROR_BIT | 31);
    /// The language specified was invalid.
    pub const INVALID_LANGUAGE: Self = Self(Self::ERROR_BIT | 32);
    /// The security status of the data is unknown or compromised and the data must be updated or
    /// replaced to restore a valid security status.
    pub const COMPROMISED_DATA: Self = Self(Self::ERROR_BIT | 33);
    /// There is an address conflict during address allocation.
    pub const IP_ADDRESS_CONFLICT: Self = Self(Self::ERROR_BIT | 34);
    /// A HTTP error occurred during the network operation.
    pub const HTTP_ERROR: Self = Self(Self::ERROR_BIT | 35);

    /// Returns `true` if `self` is an warning code, otherwise returns `false`.
    pub const fn warning(self) -> bool {
        self.0 & Self::ERROR_BIT == 0 && self.0 != Self::SUCCESS.0
    }

    /// Returns `true` if `self` is an error code, otherwise returns `false`.
    pub const fn error(self) -> bool {
        self.0 & Self::ERROR_BIT == Self::ERROR_BIT
    }

    /// Returns `true` if `self` is reserved for use by OEMs, otherwise returns `false`.
    pub const fn oem(self) -> bool {
        self.0 & Self::OEM_BIT == Self::OEM_BIT
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SUCCESS => "SUCCESS".fmt(f),

            Self::WARN_UNKNOWN_GLYPH => "WARN_UNKNOWN_GLYPH".fmt(f),
            Self::WARN_DELETE_FAILURE => "WARN_DELETE_FAILURE".fmt(f),
            Self::WARN_WRITE_FAILURE => "WARN_WRITE_FAILURE".fmt(f),
            Self::WARN_BUFFER_TOO_SMALL => "WARN_BUFFER_TOO_SMALL".fmt(f),
            Self::WARN_STALE_DATA => "WARN_STALE_DATA".fmt(f),
            Self::WARN_FILE_SYSTEM => "WARN_FILE_SYSTEM".fmt(f),
            Self::WARN_RESET_REQUIRED => "WARN_RESET_REQUIRED".fmt(f),

            Self::LOAD_ERROR => "LOAD_ERROR".fmt(f),
            Self::INVALID_PARAMETER => "INVALID_PARAMETER".fmt(f),
            Self::UNSUPPORTED => "UNSUPPORTED".fmt(f),
            Self::BAD_BUFFER_SIZE => "BAD_BUFFER_SIZE".fmt(f),
            Self::BUFFER_TOO_SMALL => "BUFFER_TOO_SMALL".fmt(f),
            Self::NOT_READY => "NOT_READY".fmt(f),
            Self::DEVICE_ERROR => "DEVICE_ERROR".fmt(f),
            Self::WRITE_PROTECTED => "WRITE_PROTECTED".fmt(f),
            Self::OUT_OF_RESOURCES => "OUT_OF_RESOURCES".fmt(f),
            Self::VOLUME_CORRUPTED => "VOLUME_CORRUPTED".fmt(f),
            Self::VOLUME_FULL => "VOLUME_FULL".fmt(f),
            Self::NO_MEDIA => "NO_MEDIA".fmt(f),
            Self::MEDIA_CHANGED => "MEDIA_CHANGED".fmt(f),
            Self::NOT_FOUND => "NOT_FOUND".fmt(f),
            Self::ACCESS_DENIED => "ACCESS_DENIED".fmt(f),
            Self::NO_RESPONSE => "NO_RESPONSE".fmt(f),
            Self::NO_MAPPING => "NO_MAPPING".fmt(f),
            Self::TIMEOUT => "TIMEOUT".fmt(f),
            Self::NOT_STARTED => "NOT_STARTED".fmt(f),
            Self::ALREADY_STARTED => "ALREADY_STARTED".fmt(f),
            Self::ABORTED => "ABORTED".fmt(f),
            Self::ICMP_ERROR => "ICMP_ERROR".fmt(f),
            Self::TFTP_ERROR => "TFTP_ERROR".fmt(f),
            Self::PROTOCOL_ERROR => "PROTOCOL_ERROR".fmt(f),
            Self::INCOMPATIBLE_VERSION => "INCOMPATIBLE_VERSION".fmt(f),
            Self::SECURITY_VIOLATION => "SECURITY_VIOLATION".fmt(f),
            Self::CRC_ERROR => "CRC_ERROR".fmt(f),
            Self::END_OF_MEDIA => "END_OF_MEDIA".fmt(f),
            Self::END_OF_FILE => "END_OF_FILE".fmt(f),
            Self::INVALID_LANGUAGE => "INVALID_LANGUAGE".fmt(f),
            Self::COMPROMISED_DATA => "COMPROMISED_DATA".fmt(f),
            Self::IP_ADDRESS_CONFLICT => "IP_ADDRESS_CONFLICT".fmt(f),
            Self::HTTP_ERROR => "HTTP_ERROR".fmt(f),

            unknown => f.debug_tuple("Status").field(&unknown).finish(),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// A collection of related interfaces.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle(pub *mut ffi::c_void);

/// A handle to an event structure.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Event(pub *mut ffi::c_void);

/// A logical block address for disks.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalBlockAddress(pub u64);

/// A task priority level signalling indicates interruptability.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskPriorityLevel(pub usize);

impl TaskPriorityLevel {
    /// The [`TaskPriorityLevel`] at which all normal execution occurs.
    ///
    /// This [`TaskPriorityLevel`] is used when no [`Event`] notifications are pending.
    pub const APPLICATION: Self = Self(4);
    /// The [`TaskPriorityLevel`] which is typically used for application level notification
    /// functions.
    ///
    /// Long term operations such as file system operations and disk IO can occur at this level.
    pub const CALLBACK: Self = Self(8);
    /// The [`TaskPriorityLevel`] at which most low level IO occurs.
    ///
    /// Blocking is not allowed at this level.
    pub const NOTIFY: Self = Self(16);
    /// The highest [`TaskPriorityLevel`] at which any operations that must be available from any
    /// priority level occur.
    ///
    /// Interrupts are disabled and this [`TaskPriorityLevel`] should be used as little as
    /// possible.
    pub const HIGH_LEVEL: Self = Self(31);
}

/// A Media Access Control address.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MacAddress(pub [u8; 32]);

/// An IPv4 internet protocol address.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv4Address(pub [u8; 4]);

/// An IPv6 internet protocol address.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv6Address(pub [u8; 16]);

/// An IPv4 or IPv6 internet protocol address.
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub union IpAddress {
    /// An IPv4 internet protocol address.
    pub ipv4: Ipv4Address,
    /// An IPv6 internet protocol address.
    pub ipv6: Ipv6Address,
}

#[cfg(test)]
mod test {
    use crate::data_type::Guid;

    const TEST_GUID: Guid = Guid {
        time_low: 0x09576e91,
        time_mid: [0x3f, 0x6d],
        time_high_version: [0xd2, 0x11],
        clock_seq_high_reserved: 0x8e,
        clock_seq_low: 0x39,
        node: [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    };

    const TEST_GUID_STR: &str = TEST_GUID_STR_LOWER;
    const TEST_GUID_STR_LOWER: &str = "09576e91-6d3f-11d2-8e39-00a0c969723b";
    const TEST_GUID_STR_UPPER: &str = "09576E91-6D3F-11D2-8E39-00A0C969723B";

    #[test]
    fn test_parse() {
        let guid = Guid::try_parse(TEST_GUID_STR).unwrap();

        assert_eq!(guid, TEST_GUID);
    }

    #[test]
    fn roundtrip_lower() {
        let guid = Guid::try_parse(TEST_GUID_STR_LOWER).unwrap();
        let guid_str = guid.to_str_lower();

        assert_eq!(&guid_str, TEST_GUID_STR_LOWER.as_bytes());
    }

    #[test]
    fn roundtrip_upper() {
        let guid = Guid::try_parse(TEST_GUID_STR_UPPER).unwrap();
        let guid_str = guid.to_str_upper();

        assert_eq!(&guid_str, TEST_GUID_STR_UPPER.as_bytes());
    }

    #[test]
    fn lower_to_upper_passthrough() {
        let guid = Guid::try_parse(TEST_GUID_STR_LOWER).unwrap();
        let guid_str = guid.to_str_upper();

        assert_eq!(&guid_str, TEST_GUID_STR_UPPER.as_bytes());
    }

    #[test]
    fn upper_to_lower_passthrough() {
        let guid = Guid::try_parse(TEST_GUID_STR_UPPER).unwrap();
        let guid_str = guid.to_str_lower();

        assert_eq!(&guid_str, TEST_GUID_STR_LOWER.as_bytes());
    }
}

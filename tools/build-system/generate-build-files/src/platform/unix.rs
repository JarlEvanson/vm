//! Unix specific code.

use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
};

/// Converts a [`Vec<u8>`] instance into an [`OsString`].
pub fn bytes_to_os_string<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> OsString {
    use std::os::unix::ffi::OsStringExt;

    OsString::from_vec(bytes.into().into_owned())
}

/// Converts an [`OsString`] into a [`Vec<u8>`] instance.
pub fn os_string_to_bytes<'a>(os_str: impl Into<Cow<'a, OsStr>>) -> Vec<u8> {
    use std::os::unix::ffi::OsStringExt;

    os_str.into().into_owned().into_vec()
}

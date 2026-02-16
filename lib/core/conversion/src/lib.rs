//! A const-friendly integer conversion library with panic and truncation variants.
#![no_std]

/// Safely converts `value` to a `u16` relying on compile time code checking.
#[expect(clippy::cast_possible_truncation)]
#[cfg(target_pointer_width = "16")]
pub const fn usize_to_u16(value: usize) -> u16 {
    value as u16
}

/// Safely converts `value` to a `u32` relying on compile time code checking.
#[expect(clippy::cast_possible_truncation)]
#[cfg(any(target_pointer_width = "16", target_pointer_width = "32",))]
pub const fn usize_to_u32(value: usize) -> u32 {
    value as u32
}

/// Safely converts `value` to a `u64` relying on compile time code checking.
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn usize_to_u64(value: usize) -> u64 {
    value as u64
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn u8_to_usize(value: u8) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn u16_to_usize(value: u16) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub const fn u32_to_usize(value: u32) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::cast_possible_truncation)]
#[cfg(target_pointer_width = "64")]
pub const fn u64_to_usize(value: u64) -> usize {
    value as usize
}

/// Converts `value` to a `u8`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u8`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u8_strict(value: usize) -> u8 {
    assert!(usize_to_u64(value) <= u8::MAX as u64);

    value as u8
}

/// Converts `value` to a `u16`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u16`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u16_strict(value: usize) -> u16 {
    assert!(usize_to_u64(value) <= u16::MAX as u64);

    value as u16
}

/// Converts `value` to a `u32`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u32`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u32_strict(value: usize) -> u32 {
    assert!(usize_to_u64(value) <= u32::MAX as u64);

    value as u32
}

/// Converts `value` to a `u64`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u64`.
pub const fn usize_to_u64_strict(value: usize) -> u64 {
    value as u64
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
pub const fn u8_to_usize_strict(value: u8) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
pub const fn u16_to_usize_strict(value: u16) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
pub const fn u32_to_usize_strict(value: u32) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
#[expect(clippy::cast_possible_truncation)]
pub const fn u64_to_usize_strict(value: u64) -> usize {
    assert!(value <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `u8`, truncating if it exceeds `u8::MAX`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u8_truncating(value: usize) -> u8 {
    value as u8
}

/// Converts `value` to a `u16`, truncating if it exceeds `u16::MAX`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u16_truncating(value: usize) -> u16 {
    value as u16
}

/// Converts `value` to a `u32`, truncating if it exceeds `u32::MAX`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u32_truncating(value: usize) -> u32 {
    value as u32
}

/// Converts `value` to a `u64`, truncating if it exceeds `u64::MAX`.
pub const fn usize_to_u64_truncating(value: usize) -> u64 {
    value as u64
}

/// Converts `value` to a `usize`, truncating if it exceeds `usize::MAX`.
pub const fn u8_to_usize_truncating(value: u8) -> usize {
    value as usize
}

/// Converts `value` to a `usize`, truncating if it exceeds `usize::MAX`.
pub const fn u16_to_usize_truncating(value: u16) -> usize {
    value as usize
}

/// Converts `value` to a `usize`, truncating if it exceeds `usize::MAX`.
pub const fn u32_to_usize_truncating(value: u32) -> usize {
    value as usize
}

/// Converts `value` to a `usize`, truncating if it exceeds `usize::MAX`.
#[expect(clippy::cast_possible_truncation)]
pub const fn u64_to_usize_truncating(value: u64) -> usize {
    value as usize
}

/// Attempts to convert `value` to a `u8`.
///
/// Returns `None` if `value` cannot fit within a `u8`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u8_checked(value: usize) -> Option<u8> {
    if usize_to_u64(value) <= u8::MAX as u64 {
        Some(value as u8)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `u16`.
///
/// Returns `None` if `value` cannot fit within a `u16`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u16_checked(value: usize) -> Option<u16> {
    if usize_to_u64(value) <= u16::MAX as u64 {
        Some(value as u16)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `u32`.
///
/// Returns `None` if `value` cannot fit within a `u32`.
#[expect(clippy::cast_possible_truncation)]
pub const fn usize_to_u32_checked(value: usize) -> Option<u32> {
    if usize_to_u64(value) <= u32::MAX as u64 {
        Some(value as u32)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `u64`.
///
/// This always succeeds.
pub const fn usize_to_u64_checked(value: usize) -> Option<u64> {
    Some(value as u64)
}

/// Attempts to convert `value` to a `usize`.
///
/// Returns `None` if `value` cannot fit within a `usize`.
pub const fn u8_to_usize_checked(value: u8) -> Option<usize> {
    if value as u64 <= usize_to_u64(usize::MAX) {
        Some(value as usize)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `usize`.
///
/// Returns `None` if `value` cannot fit within a `usize`.
pub const fn u16_to_usize_checked(value: u16) -> Option<usize> {
    if value as u64 <= usize_to_u64(usize::MAX) {
        Some(value as usize)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `usize`.
///
/// Returns `None` if `value` cannot fit within a `usize`.
pub const fn u32_to_usize_checked(value: u32) -> Option<usize> {
    if value as u64 <= usize_to_u64(usize::MAX) {
        Some(value as usize)
    } else {
        None
    }
}

/// Attempts to convert `value` to a `usize`.
///
/// Returns `None` if `value` cannot fit within a `usize`.
#[expect(clippy::cast_possible_truncation)]
pub const fn u64_to_usize_checked(value: u64) -> Option<usize> {
    if value <= usize_to_u64(usize::MAX) {
        Some(value as usize)
    } else {
        None
    }
}

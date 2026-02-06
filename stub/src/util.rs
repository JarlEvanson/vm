//! Various utility functions.

/// Safely converts `value` to a `u16` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(target_pointer_width = "16")]
pub const fn usize_to_u16(value: usize) -> u16 {
    value as u16
}

/// Safely converts `value` to a `u32` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(any(target_pointer_width = "16", target_pointer_width = "32",))]
pub const fn usize_to_u32(value: usize) -> u32 {
    value as u32
}

/// Safely converts `value` to a `u64` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn usize_to_u64(value: usize) -> u64 {
    value as u64
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn u8_to_usize(value: u8) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(any(
    target_pointer_width = "16",
    target_pointer_width = "32",
    target_pointer_width = "64"
))]
pub const fn u16_to_usize(value: u16) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub const fn u32_to_usize(value: u32) -> usize {
    value as usize
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions)]
#[cfg(target_pointer_width = "64")]
pub const fn u64_to_usize(value: u64) -> usize {
    value as usize
}

/// Safely converts `value` to a `u8` relying on compile time code checking.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u16`.
#[expect(clippy::as_conversions)]
pub const fn usize_to_u8_panicking(value: usize) -> u8 {
    assert!(usize_to_u64(value) <= u8::MAX as u64);

    value as u8
}

/// Safely converts `value` to a `u16` relying on compile time code checking.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u16`.
#[expect(clippy::as_conversions)]
pub const fn usize_to_u16_panicking(value: usize) -> u16 {
    assert!(usize_to_u64(value) <= u16::MAX as u64);

    value as u16
}

/// Safely converts `value` to a `u32` relying on compile time code checking.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u32`.
#[expect(clippy::as_conversions)]
pub const fn usize_to_u32_panicking(value: usize) -> u32 {
    assert!(usize_to_u64(value) <= u32::MAX as u64);

    value as u32
}

/// Safely converts `value` to a `u64` relying on compile time code checking.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `u64`.
#[expect(clippy::as_conversions)]
pub const fn usize_to_u64_panicking(value: usize) -> u64 {
    value as u64
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
#[expect(clippy::as_conversions)]
pub const fn u8_to_usize_panicking(value: u8) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
#[expect(clippy::as_conversions)]
pub const fn u16_to_usize_panicking(value: u16) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
#[expect(clippy::as_conversions)]
pub const fn u32_to_usize_panicking(value: u32) -> usize {
    assert!(value as u64 <= usize_to_u64(usize::MAX));

    value as usize
}

/// Converts `value` to a `usize`.
///
/// # Panics
///
/// Panics if `value` cannot fit within a `usize`.
#[expect(clippy::as_conversions)]
pub const fn u64_to_usize_panicking(value: u64) -> usize {
    assert!(value <= usize_to_u64(usize::MAX));

    value as usize
}

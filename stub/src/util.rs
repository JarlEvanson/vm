//! Various utility functions.

/// Safely converts `value` to a `u64` relying on compile time code checking.
#[expect(clippy::as_conversions)]
pub const fn usize_to_u64(value: usize) -> u64 {
    #[cfg(not(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    )))]
    compile_error!("revm-stub only supports 16-bit, 32-bit, and 64-bit usize");

    value as u64
}

/// Safety converts `value` to a `usize` relying on compile time code checking.
#[expect(clippy::as_conversions)]
pub const fn u64_to_usize(value: u64) -> usize {
    #[cfg(not(any(target_pointer_width = "64")))]
    compile_error!("revm-stub only supports 64-bit usize");

    value as usize
}

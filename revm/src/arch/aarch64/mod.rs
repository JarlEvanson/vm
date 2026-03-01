//! `aarch64`-specific functionality.

/// Returns the size, in bytes, of pages and frames on `revm-stub`.
pub fn compute_page_size() -> usize {
    64 * 1024
}

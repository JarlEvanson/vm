//! Functionality shared between `x86_32` and `x86_64`.

pub mod paging;

/// Returns the size, in bytes, of pages and frames on `revm-stub`.
pub fn compute_page_size() -> usize {
    4096
}

//! Architectural memory manipulation and introspection functionality for `i686` and `x86_64.

/// Computes the page frame size to be used for this application
pub fn compute_page_frame_size() -> usize {
    4096
}

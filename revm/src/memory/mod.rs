//! Definitions and implementations of physical, virtual, and heap management APIs for `revm`.

use sync::ControlledModificationCell;

use crate::arch;

/// The size, in bytes, of a base page and frame.
static PAGE_FRAME_SIZE: ControlledModificationCell<usize> = ControlledModificationCell::new(0);

/// Initializes the memory management subsytem for `revm`.
///
/// # Safety
///
/// This function must be called before any memory subsystem APIs have been called and may only be
/// called until the first time a memory subsystem API is utilized.
#[expect(
    clippy::missing_panics_doc,
    reason = "architectural invariant checking"
)]
pub unsafe fn initialize_memory_management() {
    let page_size = arch::memory::compute_page_frame_size();
    assert_ne!(page_size, 0, "page size must not be zero");
    assert!(
        page_size.is_power_of_two(),
        "page size must be a power of two"
    );

    // SAFETY:
    //
    // The invariants of this function ensure that [`PAGE_FRAME_SIZE`] will not be accessed for the
    // duration of this function and thus it is safe to modify [`PAGE_FRAME_SIZE`].
    unsafe { *PAGE_FRAME_SIZE.get_mut() = page_size }
}

/// Returns the size, in bytes, of a base page and frame.
#[inline(always)]
pub fn page_frame_size() -> usize {
    *PAGE_FRAME_SIZE.get()
}

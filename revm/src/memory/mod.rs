//! Definitions and implementations of virtual and physical memory management APIs for `revm`.

use sync::ControlledModificationCell;

use crate::arch;

pub mod phys;
pub mod virt;

/// The size, in bytes, of pages and frames.
static PAGE_FRAME_SIZE: ControlledModificationCell<usize> = ControlledModificationCell::new(0);

/// Initializes the memory management subsystem for `revm`.
///
/// # Safety
///
/// This function must be called before any memory APIs may be called and may only be called until
/// the first time a memory API is utilized.
#[expect(clippy::missing_panics_doc, reason = "validation of invariants")]
pub unsafe fn initialize_memory_management() {
    let page_size = arch::compute_page_size();
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

/// Returns the size, in bytes, of a page and frame.
#[inline(always)]
pub fn page_frame_size() -> usize {
    *PAGE_FRAME_SIZE.get()
}

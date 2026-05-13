//! Definitions and interfaces that platforms utilize to provide physical memory management services
//! for use by the rest of the executable.

use core::{alloc::Layout, ptr::NonNull};

use sync::ControlledModificationCell;

/// The current [`Allocator`].
static ALLOCATOR: ControlledModificationCell<Option<&'static dyn Allocator>> =
    ControlledModificationCell::new(None);

/// Initializes the heap allocation subsystem.
///
/// # Safety
///
/// This function must not be called when any other heap allocation function is active.
pub(in crate::platform) unsafe fn initialize_allocator(allocator: &'static dyn Allocator) {
    // SAFETY:
    //
    // The invariants of [`initialize_allocator()`] ensure that this operation is safe.
    unsafe { *ALLOCATOR.get_mut() = Some(allocator) }
}

/// Returns the currently active [`Allocator`].
fn allocator() -> &'static dyn Allocator {
    ALLOCATOR
        .get()
        .expect("heap allocation subsystem is uninitialized")
}

/// Allocates a region of memory that fulfills the requirements provided in [`Layout`].
pub fn allocate(layout: Layout) -> Option<NonNull<u8>> {
    allocator().allocate(layout)
}

/// Deallocates the memory referenced by `ptr`.
///
/// # Safety
///
/// - `ptr` must describe a block of memory currently allocated via a call to [`allocate()`].
/// - `layout` must have the same size and alignment as the [`Layout`] passed to the call that
///   allocated `ptr`.
pub unsafe fn deallocate(ptr: NonNull<u8>, layout: Layout) {
    // SAFETY:
    //
    // The invariants of [`deallocate()`] ensure that the invariants of
    // `allocator().deallocate()` are fulfilled.
    unsafe { allocator().deallocate(ptr, layout) }
}

/// Trait representing a platform-independent mechanism for heap allocation.
pub(in crate::platform) trait Allocator: Send + Sync {
    /// Allocates a region of memory that fulfills the requirements provided in [`Layout`].
    fn allocate(&self, layout: Layout) -> Option<NonNull<u8>>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    //
    // - `ptr` must describe a block of memory currently allocated via a call to [`allocate()`].
    // - `layout` must have the same size and alignment as the [`Layout`] passed to the call that
    //   allocated `ptr`.
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);
}

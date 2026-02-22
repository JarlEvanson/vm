//! Implementation of general memory allocation APIs.

use core::ptr::NonNull;

use crate::memory::general::slab::MAX_SIZE_CLASS;

pub mod page;
pub mod slab;

/// Initializes the general purpose memory allocator for `revm`.
///
/// # Safety
///
/// This function must be called before any general purpose APIs may be called and may only be
/// called until the first time a general purpose memory allocation API is utilized.
pub unsafe fn initialize_allocator() {
    // SAFETY:
    //
    // This function was called before any general purpose memory APIs were called and thus
    // [`slab::initialize()`] will be called before any slab APIs are called.
    unsafe { slab::initialize() }
}

/// Allocates a region of memory of `size` bytes aligned to `alignment`.
pub fn allocate(size: usize, alignment: usize) -> Option<NonNull<u8>> {
    if size <= *MAX_SIZE_CLASS.get() && alignment <= *MAX_SIZE_CLASS.get() {
        slab::allocate(size.max(alignment))
    } else {
        page::allocate(size, alignment)
    }
}

/// Deallocates the memory referenced by `ptr`.
///
/// # Safety
///
/// The `size` and `alignment` parameters must match the parameters utilized when the memory
/// referenced by `ptr` was allocated by a call to [`allocate()`]. `ptr` must describe a currently
/// allocated block of memory.
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize, alignment: usize) {
    if size <= *MAX_SIZE_CLASS.get() && alignment <= *MAX_SIZE_CLASS.get() {
        // SAFETY:
        //
        // The invariants of [`deallocate()`] ensure that it is safe to call
        // [`slab::deallocate()`].
        unsafe { slab::deallocate(ptr, size.max(alignment)) }
    } else {
        crate::debug!("implement page-sized deallocation")
    }
}

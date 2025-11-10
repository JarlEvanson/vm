//! Code for controlled modification, placing unsafety on the initialization/changing function.
//!
//! This produces better code at the cost of safety.

use core::cell::UnsafeCell;

/// Wrapper struct for variables that are modified in a thread safe manner that is not visible to
/// Rust code.
#[derive(Debug)]
pub struct ControlledModificationCell<T: ?Sized> {
    /// The variable that is modified.
    value: UnsafeCell<T>,
}

// SAFETY:
//
// Since all mutations are thread-safe, and [`T`] is [`Send`], this is safe.
unsafe impl<T: ?Sized + Send> Sync for ControlledModificationCell<T> {}
// SAFETY:
//
// It is safe to send this across thread boundaries when [`T`] is safe to send across
// because all mutations are thread-safe, and [`T`] is safe to send.
unsafe impl<T: ?Sized + Send> Send for ControlledModificationCell<T> {}

impl<T> ControlledModificationCell<T> {
    /// Constructs a new instance of a [`ControlledModificationCell`].
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Returns a immutable reference to the contained value.
    pub fn get(&self) -> &T {
        // SAFETY:
        // This item is only modified in a thread-safe manner.
        unsafe { &*self.value.get() }
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Safety
    /// - The lifetime of the mutable reference produced by this function does not overlap with the
    ///   lifetime of any other reference, mutable or immutable, that points to this value.
    /// - All synchronization necessary to soundly mutate this value must be performed outside of
    ///   this function.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        // SAFETY:
        // According to the invariants of [`Self::get_mut()`], creating a mutable reference is
        // safe.
        unsafe { &mut *self.value.get() }
    }
}

impl<T: Copy> ControlledModificationCell<T> {
    /// Copies the stored value.
    pub fn copy(&self) -> T {
        // SAFETY:
        // This item is only modified in a thread-safe manner.
        unsafe { self.value.get().read() }
    }
}

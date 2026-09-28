//! Code for controlled modification, placing unsafety on the initialization/changing function.
//!
//! This produces better code at the cost of safety.

use core::{cell::UnsafeCell, fmt, ptr};

/// Wrapper structure for variables that are modified in a manner compliant with the UB model but
/// without such safety being exposed to the Rust codebase.
#[repr(transparent)]
pub struct ControlledModificationCell<T: ?Sized> {
    /// The variable that is modified.
    value: UnsafeCell<T>,
}

impl<T> ControlledModificationCell<T> {
    /// Constructs a new instance of a [`ControlledModificationCell`].
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Returns a immutable reference to the contained value.
    pub const fn get(&self) -> &T {
        // SAFETY:
        // This item is only accessed by immutable reference for the entire lifetime of the
        // returned reference.
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
    pub const unsafe fn get_mut(&self) -> &mut T {
        // SAFETY:
        // According to the invariants of [`Self::get_mut()`], creating a mutable reference is
        // safe.
        unsafe { &mut *self.value.get() }
    }

    /// Returns a mutable reference to the wrapped value.
    pub const fn as_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Unwraps the value, consuming the cell.
    pub const fn into_inner(self) -> T {
        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY:
        //
        // This call owns `this` and both `ManuallyDrop` and `UnsafeCell` guaranteed that they have
        // the same layout and bit validity as the wrapped value.
        unsafe { ptr::read(ptr::from_ref(&this).cast::<T>()) }
    }
}

impl<T: Copy> ControlledModificationCell<T> {
    /// Copies the stored value.
    pub fn copy(&self) -> T {
        *self.get()
    }
}

impl<T: fmt::Debug> fmt::Debug for ControlledModificationCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Debug>::fmt(self.get(), f)
    }
}

impl<T: fmt::Display> fmt::Display for ControlledModificationCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Display>::fmt(self.get(), f)
    }
}

// SAFETY:
//
// [`T`] must be [`Send`] for a [`ControlledModificationCell`] be be [`Send`] because it is possible
// to acquire the owned [`T`] from the [`ControlledModificationCell`] via
// [`ControlledModificationCell::into_inner()`].
unsafe impl<T: ?Sized + Send> Send for ControlledModificationCell<T> {}

// SAFETY:
//
// [`T`] must be [`Send`] for a [`ControlledModificationCell`] be be [`Sync`] because it is possible
// to acquire the owned [`T`] from the [`ControlledModificationCell`] via
// [`ControlledModificationCell::get_mut()`] and [`core::mem::replace()`].
//
// [`T`] must be [`Sync`] for a [`ControlledModificationCell`] to be [`Sync`] because it is possible
// to obtain an immutable reference to the [`T`] from an immutable reference to a
// [`ControlledModificationCell`]  via [`ControlledModificationCell::get()`].
unsafe impl<T: ?Sized + Send + Sync> Sync for ControlledModificationCell<T> {}

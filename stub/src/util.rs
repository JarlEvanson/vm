//! Miscellaneous utilities.

use core::{mem, ptr};

/// Wrapper around running a function on data when dropped.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropWrapper<T, F: FnMut(&mut T)> {
    /// The value to run the function.
    pub val: T,
    /// The function to run.
    pub drop_func: F,
}

impl<T, F: FnMut(&mut T)> DropWrapper<T, F> {
    /// Returns the `val` inside of [`DropWrapper`] without running the provided
    /// [`DropWrapper::drop_func`].
    pub fn into_inner(self) -> T {
        // SAFETY:
        //
        // - `self.val` is valid for reads, initialized, and properly aligned.
        let val = unsafe { ptr::read(&self.val) };
        mem::forget(self);
        val
    }
}

impl<T, F: FnMut(&mut T)> Drop for DropWrapper<T, F> {
    fn drop(&mut self) {
        (self.drop_func)(&mut self.val)
    }
}

//! Simple spinlock implementation.

use core::{
    cell::UnsafeCell,
    error, fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// The locking component of a [`Spinlock`].
#[derive(Debug)]
pub struct RawSpinlock {
    /// The lock.
    lock: AtomicBool,
}

impl RawSpinlock {
    /// Creates a new [`RawSpinlock`] in the unlocked state.
    pub const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }

    /// Locks the [`RawSpinlock`], spinning until the lock is acquired.
    ///
    /// This function does not return until the lock has been acquired.
    pub fn lock(&self) {
        let mut was_locked = self.lock.load(Ordering::Relaxed);

        loop {
            if !was_locked {
                match self
                    .lock
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                {
                    Ok(_) => break,
                    Err(state) => was_locked = state,
                }
            }

            core::hint::spin_loop();
        }
    }

    /// Attempts to lock the [`RawSpinlock`].
    ///
    /// This function does not spin or block.
    ///
    /// # Errors
    /// If the [`RawSpinlock`] was already locked, then this call will return an [`Err`].
    pub fn try_lock(&self) -> Result<(), SpinlockAcquisitionError> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(())
        } else {
            Err(SpinlockAcquisitionError)
        }
    }

    /// Unlocks the [`RawSpinlock`].
    pub fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl Default for RawSpinlock {
    fn default() -> Self {
        Self::new()
    }
}

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Spinlock<T: ?Sized> {
    /// The lock.
    lock: RawSpinlock,
    /// The value protected by the [`Spinlock`].
    value: UnsafeCell<T>,
}

// SAFETY:
// Nothing about `Spinlock<T>` changes whether it
// is safe to send `T` across threads.
unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}

// SAFETY:
// If `T` is safe to send across threads, then `Spinlock<T>`
// makes it safe to access from multiple threads simultaneously.
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Creates a new [`Spinlock`] in an unlocked state ready for use.
    pub const fn new(value: T) -> Self {
        Self {
            lock: RawSpinlock::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes this [`Spinlock`], returning the underlying data.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> Spinlock<T> {
    /// Acquires the [`Spinlock`], spinning until the lock is available.
    ///
    /// This function will spin until the lock is available. Upon returning, this context is the
    /// only context with the lock held. A RAII guard is returned to allow for scoped unlock of the
    /// [`Spinlock`].
    pub fn lock<'a>(&'a self) -> SpinlockGuard<'a, T> {
        self.lock.lock();

        SpinlockGuard {
            lock: &self.lock,
            value: &self.value,
        }
    }

    /// Attempts to acquire this [`Spinlock`].
    ///
    /// If the lock could not be acquired, then [`Err`] is returned. Otherwise, a RAII guard is
    /// returned. The lock will be unlocked when the guard is dropped.
    ///
    /// This function does not block.
    ///
    /// # Errors
    /// If the [`Spinlock`] could not be acquire because it is already locked, then this call will
    /// return an [`Err`].
    pub fn try_lock<'a>(&'a self) -> Result<SpinlockGuard<'a, T>, SpinlockAcquisitionError> {
        self.lock.try_lock().map(|()| SpinlockGuard {
            lock: &self.lock,
            value: &self.value,
        })
    }

    /// Method that makes unlocking a mutex more explicit.
    pub fn unlock(guard: SpinlockGuard<T>) {
        drop(guard)
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`Spinlock`] mutably, no actual locking needs to take place:
    /// the mutable borrow statically guarantees no locks exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

/// A RAII implementation of a "scoped lock" implemented using a [`Spinlock`]. When this structure
/// is dropped, the [`Spinlock`] will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its [`Deref`] and
/// [`DerefMut`] implementations.
///
/// This structure is created by the [`Spinlock::lock()`] and [`Spinlock::try_lock()`] methods.
#[must_use = "lock guard must be held to access protected data"]
pub struct SpinlockGuard<'a, T: ?Sized> {
    /// The [`RawSpinlock`] guarding [`SpinlockGuard::value`].
    lock: &'a RawSpinlock,
    /// The value to which this [`SpinlockGuard`] provides access.
    value: &'a UnsafeCell<T>,
}

impl<'a, T: ?Sized> SpinlockGuard<'a, T> {
    /// Returns a new [`SpinlockGuard`] that allows for safe access to `value`.
    ///
    /// # Safety
    /// - `lock` must be locked.
    /// - `value` must be safe to return immutable or mutable references to until `lock` is
    ///   unlocked.
    pub unsafe fn new(lock: &'a RawSpinlock, value: &'a UnsafeCell<T>) -> Self {
        Self { lock, value }
    }
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value_ptr = self.value.get();

        // SAFETY:
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &*value_ptr }
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let value_ptr = self.value.get();

        // SAFETY:
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &mut *value_ptr }
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

/// Represents the failure to acquire a [`Spinlock`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpinlockAcquisitionError;

impl fmt::Display for SpinlockAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("try_lock failed because spinlock is already held")
    }
}

impl error::Error for SpinlockAcquisitionError {}

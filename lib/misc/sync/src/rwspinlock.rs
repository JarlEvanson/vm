//! Simple reader writer spinlock implementation.

use core::{
    cell::UnsafeCell,
    error, fmt, hint,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

/// The state of [`RawRwSpinlock::state`] that indicates that the [`RawRwSpinlock`] is in locked in
/// the writer state.
const WRITE_STATE: usize = usize::MAX;

/// The locking component of a [`RawRwSpinlock`].
#[derive(Debug)]
pub struct RawRwSpinlock {
    #[expect(clippy::missing_docs_in_private_items)]
    state: AtomicUsize,
}

impl RawRwSpinlock {
    /// Creates a new [`RawRwSpinlock`] in the unlocked state.
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(0),
        }
    }

    /// Locks the [`RawRwSpinlock`] in the read state, spinning until a read lock has been acquired.
    ///
    /// This function does not return until the read lock has been acquired.
    pub fn read(&self) {
        let mut state = self.state.load(Ordering::Relaxed);

        loop {
            if state < WRITE_STATE {
                assert!(state < WRITE_STATE - 1, "too many readers");
                match self.state.compare_exchange_weak(
                    state,
                    state + 1,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(new_state) => {
                        hint::spin_loop();

                        state = new_state;
                    }
                }
            } else {
                hint::spin_loop();

                state = self.state.load(Ordering::Relaxed);
            }
        }
    }

    /// Attempts to acquire a read lock from the [`RawRwSpinlock`].
    ///
    /// This function does not spin or block.
    ///
    /// # Errors
    ///
    /// If the [`RawRwSpinlock`] has been locked too many times or is already locked for writing.
    /// then this call will return an [`Err`].
    pub fn try_read(&self) -> Result<(), RwSpinlockReadAcquisitionError> {
        let state = self.state.load(Ordering::Relaxed);
        if state < WRITE_STATE {
            if state < WRITE_STATE - 1 {
                return Err(RwSpinlockReadAcquisitionError);
            }

            if self
                .state
                .compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(());
            }
        }

        Err(RwSpinlockReadAcquisitionError)
    }

    /// Locks the [`RawRwSpinlock`] in the write state, spinning until the write lock is acquired.
    ///
    /// This function does not return until the write lock has been acquired.
    pub fn write(&self) {
        let mut state = self.state.load(Ordering::Relaxed);

        loop {
            if state != 0 {
                hint::spin_loop();

                state = self.state.load(Ordering::Relaxed);
                continue;
            }

            match self.state.compare_exchange_weak(
                state,
                WRITE_STATE,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(new_state) => {
                    hint::spin_loop();

                    state = new_state;
                }
            }
        }
    }

    /// Attempts to acquire the write lock from the [`RawRwSpinlock`].
    ///
    /// This function does not spin or block.
    ///
    /// # Errors
    ///
    /// If the [`RawRwSpinlock`] is already locked for reading or writing, then this call will
    /// return an [`Err`].
    pub fn try_write(&self) -> Result<(), RwSpinlockWriteAcquisitionError> {
        if self
            .state
            .compare_exchange(0, WRITE_STATE, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(())
        } else {
            Err(RwSpinlockWriteAcquisitionError)
        }
    }

    /// Releases the read lock acquired from this [`RawRwSpinlock`].
    pub fn read_unlock(&self) {
        self.state.fetch_sub(1, Ordering::Release);
    }

    /// Releases the write lock acquired from this [`RawRwSpinlock`].
    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
    }
}

impl Default for RawRwSpinlock {
    fn default() -> Self {
        Self::new()
    }
}

/// A reader-writer lock useful for protecting shared data from concurrent mutable access.
pub struct RwSpinlock<T: ?Sized> {
    /// The lock.
    lock: RawRwSpinlock,
    /// The value protected by the [`RwSpinlock`].
    value: UnsafeCell<T>,
}

impl<T> RwSpinlock<T> {
    /// Creates a new [`RwSpinlock`] in the unlocked state.
    pub const fn new(value: T) -> Self {
        Self {
            lock: RawRwSpinlock::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes this [`RwSpinlock`], returning the underlying value.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> RwSpinlock<T> {
    /// Acquires a read lock from the [`RwSpinlock`], spinning until such an acquisition succeeds.
    ///
    /// This function will spin until the lock is available. Upon returning, this context will have
    /// immutable access to the contained value. A RAII guard is returned to enable scoped unlock of
    /// the [`RwSpinlock`].
    pub fn read<'a>(&'a self) -> RwSpinlockReadGuard<'a, T> {
        self.lock.read();

        RwSpinlockReadGuard {
            lock: &self.lock,
            value: &self.value,
        }
    }

    /// Attempts to acquire a read lock from the [`RwSpinlock`].
    ///
    /// This function does not spin or block.
    ///
    /// # Errors
    ///
    /// If the [`RwSpinlock`] could not be acquired because it has been locked too many times at
    /// once or is already locked for writing, then this call will return an [`Err`].
    pub fn try_read<'a>(
        &'a self,
    ) -> Result<RwSpinlockReadGuard<'a, T>, RwSpinlockReadAcquisitionError> {
        self.lock.try_read().map(|()| RwSpinlockReadGuard {
            lock: &self.lock,
            value: &self.value,
        })
    }

    /// Acquires a write lock from the [`RwSpinlock`], spinning until such an acquisition succeeds.
    ///
    /// This function will spin until the lock is available. Upon returning, this context will have
    /// unique mutable access to the contained value. A RAII guard is returned to enable scoped
    /// unlock of the [`RwSpinlock`].
    pub fn write<'a>(&'a self) -> RwSpinlockWriteGuard<'a, T> {
        self.lock.write();

        RwSpinlockWriteGuard {
            lock: &self.lock,
            value: &self.value,
        }
    }

    /// Attempts to acquire the write lock from the [`RwSpinlock`].
    ///
    /// This function does not spin or block.
    ///
    /// # Errors
    ///
    /// If the [`RwSpinlock`] could not be acquired because it is already locked for reading or
    /// writing, then this call will return an [`Err`].
    pub fn try_write<'a>(
        &'a self,
    ) -> Result<RwSpinlockWriteGuard<'a, T>, RwSpinlockWriteAcquisitionError> {
        self.lock.try_write().map(|()| RwSpinlockWriteGuard {
            lock: &self.lock,
            value: &self.value,
        })
    }

    /// Releases the [`RwSpinlock`] with which the provided [`RwSpinlockReadGuard`] is associated.
    pub fn read_unlock(guard: RwSpinlockReadGuard<T>) {
        drop(guard)
    }

    /// Releases the [`RwSpinlock`] with which the provided [`RwSpinlockWriteGuard`] is associated.
    pub fn write_unlock(guard: RwSpinlockWriteGuard<T>) {
        drop(guard)
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`RwSpinlock`] mutably, no actual locking needs to take place:
    /// the mutable borrow statically guarantees no other references to the lock exist.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

// SAFETY:
//
// Nothing about `RwSpinlock<T>` changes whether it
// is safe to send `T` across threads.
unsafe impl<T: ?Sized + Send> Send for RwSpinlock<T> {}

// SAFETY:
//
// If `T` is safe to send across threads, then `RwSpinlock<T>`
// is safe to access from multiple threads at once.
unsafe impl<T: ?Sized + Sync> Sync for RwSpinlock<T> {}

/// A RAII implementation of a "scoped read lock" implemented using a [`RwSpinlock`]. When this
/// structure is dropped, the [`RwSpinlock`] will have a single read lock removed.
///
/// The data protected by the [`RwSpinlock`] can be accessed through this guard via its [`Deref`]
/// implementation.
///
/// This structure is created by the [`RwSpinlock::read()`] and [`RwSpinlock::try_read()`] methods.
pub struct RwSpinlockReadGuard<'a, T: ?Sized> {
    /// The [`RawRwSpinlock`] with which this [`RwSpinlockReadGuard`] is associated.
    lock: &'a RawRwSpinlock,
    /// The value with which this [`RwSpinlockReadGuard`] is associated.
    value: &'a UnsafeCell<T>,
}

impl<'a, T: ?Sized> RwSpinlockReadGuard<'a, T> {
    /// Returns a new [`RwSpinlockReadGuard`] that allows for safe immutable access to `value`.
    ///
    /// # Safety
    ///
    /// - `lock` must have a read lock acquisition.
    /// - `value` must be safe to return immutable references to until `lock` is unlocked.
    pub unsafe fn new(lock: &'a RawRwSpinlock, value: &'a UnsafeCell<T>) -> Self {
        Self { lock, value }
    }
}

impl<T: ?Sized> Deref for RwSpinlockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value_ptr = self.value.get();

        // SAFETY:
        //
        // We have shared access to the value pointed to by `value_ptr`.
        unsafe { &*value_ptr }
    }
}

impl<T: ?Sized> Drop for RwSpinlockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.read_unlock();
    }
}

/// A RAII implementation of a "scoped write lock" implemented using a [`RwSpinlock`]. When this
/// structure is dropped, the [`RwSpinlock`] will have a write lock removed.
///
/// The data protected by the [`RwSpinlock`] can be accessed through this guard via its [`Deref`]
/// implementation.
///
/// This structure is created by the [`RwSpinlock::write()`] and [`RwSpinlock::try_write()`] methods.
pub struct RwSpinlockWriteGuard<'a, T: ?Sized> {
    /// The [`RawRwSpinlock`] with which this [`RwSpinlockWriteGuard`] is associated.
    lock: &'a RawRwSpinlock,
    /// The value with which this [`RwSpinlockWriteGuard`] is associated.
    value: &'a UnsafeCell<T>,
}

impl<'a, T: ?Sized> RwSpinlockWriteGuard<'a, T> {
    /// Returns a new [`RwSpinlockWriteGuard`] that allows for safe mutable access to `value`.
    ///
    /// # Safety
    ///
    /// - `lock` must have a write lock acquisition.
    /// - `value` must be safe to return mutable references to until `lock` is unlocked.
    pub unsafe fn new(lock: &'a RawRwSpinlock, value: &'a UnsafeCell<T>) -> Self {
        Self { lock, value }
    }
}

impl<T: ?Sized> Deref for RwSpinlockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let value_ptr = self.value.get();

        // SAFETY:
        //
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &*value_ptr }
    }
}

impl<T: ?Sized> DerefMut for RwSpinlockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let value_ptr = self.value.get();

        // SAFETY:
        //
        // We have exclusive access to the value pointed to by `value_ptr`.
        unsafe { &mut *value_ptr }
    }
}

impl<T: ?Sized> Drop for RwSpinlockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.write_unlock();
    }
}

/// Represents the failure to acquire a read lock from a [`RwSpinlock`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RwSpinlockReadAcquisitionError;

impl fmt::Display for RwSpinlockReadAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("try_read failed because the read-write spinlock has the write lock held")
    }
}

impl error::Error for RwSpinlockReadAcquisitionError {}

/// Represents the failure to acquire the write lock from a [`RwSpinlock`].
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RwSpinlockWriteAcquisitionError;

impl fmt::Display for RwSpinlockWriteAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("try_write failed because the read-write spinlock has read locks held")
    }
}

impl error::Error for RwSpinlockWriteAcquisitionError {}

//! Implementation of reference-counted access to the `T` instance stored in a [`GraphNode`][gn].
//!
//! [gn]: super::GraphNode

use core::{
    mem::MaybeUninit,
    ops,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use sync::ControlledModificationCell;

/// The wrapper around the data stored in a [`GraphNode`][gn].
///
/// [gn]: super::GraphNode
#[derive(Debug)]
pub(super) struct Data<T> {
    /// The atomic reference counter.
    references: AtomicUsize,
    /// The stored data.
    data: ControlledModificationCell<MaybeUninit<T>>,
}

impl<T> Data<T> {
    /// The value of [`Self::references`] that indicates that [`Self::data`] is uninitialized.
    const EMPTY: usize = 0;
    /// The value of [`Self::references`] that indicates that [`Self::data`] is locked and thus is
    /// either initializing or uninitializing itself.
    const LOCKED: usize = 1;
    /// The value of [`Self::references`] that indicates that [`Self::data`] is initialized
    /// but has zero references.
    const INITIALIZED: usize = 2;

    /// Creates an unreferenced [`Data`] with an uninitialized `T` instance.
    pub(super) const fn new() -> Self {
        Self {
            references: AtomicUsize::new(0),
            data: ControlledModificationCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initializes this [`Data`] with the provided `val`, panicking if [`Reference`]s to the `T`
    /// instance exist, it is already locked, or it is already initialized.
    pub(super) fn initialize(&self, val: T) {
        let result = self.references.compare_exchange(
            Self::EMPTY,
            Self::LOCKED,
            Ordering::Acquire,
            Ordering::Relaxed,
        );
        if let Err(references_value) = result {
            if references_value == Self::LOCKED {
                panic!("cannot initialize locked node data");
            } else if references_value == Self::INITIALIZED {
                panic!("cannot initialize initialized node data");
            } else {
                let reference_count = references_value - Self::INITIALIZED;
                panic!(
                    "cannot initialize initialized node data with {reference_count} reference(s)"
                );
            }
        }

        // SAFETY:
        //
        // Since `self.references` was `Self::EMPTY`, `data` must not have been initialized. Since
        // `self.references = Self::LOCKED` indicates that access to the `T` instance is locked, we
        // have exclusive access to write to `data`. Moreover, the `T` instance is currently
        // uninitialized and thus doesn't need to be dropped.
        unsafe { self.data.get_mut().write(val) };
        // Transition to an initialized state.
        self.references.store(Self::INITIALIZED, Ordering::Release);
    }

    /// Takes a [`Reference`] to the `T` instance associated with this [`GraphNode`][gn].
    ///
    /// [gn]: super::GraphNode
    pub(super) fn ref_data<'a>(self: Pin<&'a Self>) -> Reference<'a, T> {
        self.ref_data_opt()
            .expect("attempted to reference data associated with an inactive node")
    }

    /// Attempts to take a [`Reference`] to the `T` instance associated with this
    /// [`GraphNode`][gn].
    ///
    /// [gn]: super::GraphNode
    pub(super) fn ref_data_opt<'a>(self: Pin<&'a Self>) -> Option<Reference<'a, T>> {
        if self.add_ref() {
            Some(Reference { ptr: self })
        } else {
            None
        }
    }

    /// Claims the stored data, panicking if [`Reference`]s to this `T` instance exist, it is
    /// already locked, or it is uninitialized.
    pub(super) fn take(&self) -> T {
        let result = self.references.compare_exchange(
            Self::INITIALIZED,
            Self::LOCKED,
            Ordering::Acquire,
            Ordering::Relaxed,
        );
        if let Err(references_value) = result {
            if references_value == Self::EMPTY {
                panic!("cannot take uninitialized node data");
            } else if references_value == Self::LOCKED {
                panic!("cannot take locked node data");
            } else {
                let reference_count = references_value - Self::INITIALIZED;
                panic!("cannot take node data with {reference_count} reference(s)");
            }
        }

        // SAFETY:
        //
        // Since `self.references` was `Self::INITIALIZED`, `data` must have been initialized.
        // Since `self.references = Self::LOCKED` indicates that it is locked, we have exclusive
        // access to read from `data`.
        let data = unsafe { self.data.get().assume_init_read() };
        // Transition to an uninitialized state.
        self.references.store(Self::EMPTY, Ordering::Release);
        data
    }

    /// Attempts to take an additional reference on the `T` instance.
    fn add_ref(self: Pin<&Self>) -> bool {
        let mut current = self.references.load(Ordering::Relaxed);
        loop {
            if current == Self::EMPTY || current == Self::LOCKED {
                return false;
            }

            let Some(new) = current.checked_add(1) else {
                panic!("too many references to node data");
            };
            let result = self.references.compare_exchange_weak(
                current,
                new,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
            match result {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }

        true
    }

    /// Clones a reference on the `T` instance.
    ///
    /// # Safety
    ///
    /// This must only be called when another [`Reference`] already exists.
    unsafe fn clone_ref(self: Pin<&Self>) {
        let mut current = self.references.load(Ordering::Relaxed);
        loop {
            let Some(new) = current.checked_add(1) else {
                panic!("cannot clone node reference: too many active references");
            };
            let result = self.references.compare_exchange_weak(
                current,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
            match result {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }
    }
}

impl<T> Drop for Data<T> {
    fn drop(&mut self) {
        let references_value = self.references.load(Ordering::Acquire);
        if references_value == Self::EMPTY {
            return;
        } else if references_value == Self::LOCKED {
            panic!("cannot drop locked node data");
        } else if references_value != Self::INITIALIZED {
            unreachable!();
        }

        drop(self.take());
    }
}

/// A thread-safe reference-counted wrapper around the `T` instance stored in a [`GraphNode`][gn].
///
/// [gn]: super::GraphNode
pub struct Reference<'a, T> {
    /// The [`Data`] to which this [`Reference`] refers.
    ptr: Pin<&'a Data<T>>,
}

impl<'a, T> Clone for Reference<'a, T> {
    fn clone(&self) -> Self {
        // SAFETY:
        //
        // There exists a [`Reference`] and thus this operation is safe.
        unsafe { self.ptr.clone_ref() };
        Self { ptr: self.ptr }
    }
}

impl<'a, T> ops::Deref for Reference<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        //
        // The presence of a [`Reference`] indicates that the `T` instance is initialized and
        // valid to reference.
        unsafe { self.ptr.data.get().assume_init_ref() }
    }
}

impl<T> Drop for Reference<'_, T> {
    fn drop(&mut self) {
        self.ptr.references.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod test {
    use core::pin::pin;

use super::Data;

    #[test]
    fn ref_single() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);
        let data = pin!(data);

        let reference = data.as_ref().ref_data();
        assert_eq!(*reference, init_val);
    }

    #[test]
    fn ref_many() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let data = pin!(data);

        let mut references = Vec::new();
        for _ in 0..1000 {
            references.push(data.as_ref().ref_data());
        }

        for reference in references {
            assert_eq!(*reference, init_val);
        }
    }

    #[test]
    fn ref_deref_take() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let data = pin!(data);

        let reference = data.as_ref().ref_data();
        assert_eq!(*reference, init_val);
        drop(reference);

        assert_eq!(data.take(), init_val);
    }

    #[test]
    #[should_panic]
    fn take_referenced() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let data = pin!(data);

        let reference = data.as_ref().ref_data();
        assert_eq!(*reference, init_val);

        assert_eq!(data.take(), init_val);
    }

    #[test]
    fn ref_clone() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let data = pin!(data);

        let reference_0 = data.as_ref().ref_data();
        assert_eq!(*reference_0, init_val);
        let reference_1 = reference_0.clone();
        assert_eq!(*reference_1, init_val);
        assert_eq!(*reference_0, *reference_1);
    }
}

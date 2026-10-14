//! Implementation of reference-counted access to the data stored in a [`GraphNode`][gn].
//!
//! [gn]: super::GraphNode

use core::{
    mem::MaybeUninit, ops, sync::atomic::{AtomicUsize, Ordering},
};

use sync::ControlledModificationCell;

/// The wrapper around the data stored in a [`GraphNode`][gn].
///
/// [gn]: super::GraphNode
pub(super) struct Data<T> {
    /// The atomic reference counter.
    references: AtomicUsize,
    /// The stored data.
    data: ControlledModificationCell<MaybeUninit<T>>,
}

impl<T> Data<T> {
    /// Creates an unreferenced [`Data`] with its internal `data` being uninitialized.
    pub(super) fn new() -> Self {
        Self {
            references: AtomicUsize::new(0),
            data: ControlledModificationCell::new(MaybeUninit::uninit()),
        }
    }

    /// Takes a [`Reference`] to the data associated with this [`GraphNode`][gn].
    ///
    /// [gn]: super::GraphNode
    pub(super) fn ref_data<'a>(&'a self) -> Reference<'a, T> {
        self.ref_data_opt()
            .expect("attempted to reference data associated with an inactive node")
    }

    /// Attempts to take a [`Reference`] to the data associated with this [`GraphNode`][gn].
    ///
    /// [gn]: super::GraphNode
    pub(super) fn ref_data_opt<'a>(&'a self) -> Option<Reference<'a, T>> {
        let mut current = self.references.load(Ordering::Relaxed);
        loop {
            if current == 0 || current == 1 {
                return None;
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

        Some(Reference { ptr: self })
    }

    /// Initializes the node data with the provided `val`, panicking if [`Reference`]s to this
    /// [`Data`] structure exist, it is already locked, or it is already initialized.
    pub(super) fn initialize(&self, val: T) {
        let result = self
            .references
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed);
        if let Err(references_value) = result {
            if references_value == 1 {
                panic!("cannot initialize locked node data");
            } else if references_value == 2 {
                panic!("cannot initialize initialized node data");
            } else {
                let reference_count = references_value - 2;
                panic!(
                    "cannot initialize initialized node data with {reference_count} reference(s)"
                );
            }
        }

        // SAFETY:
        //
        // Since `self.references` was 0, `data` must not have been initialized. Since
        // `self.references = 1` indicates that it is locked, we have exclusive access to write to
        // `data`.
        unsafe { self.data.get_mut().write(val) };
        self.references.store(2, Ordering::Release);
    }

    /// Claims the stored data, panicking if [`Reference`]s to this [`Data`] structure exist, it is
    /// already locked, or it is uninitialized.
    pub(super) fn take(&self) -> T {
        let result = self
            .references
            .compare_exchange(2, 1, Ordering::Acquire, Ordering::Relaxed);
        if let Err(references_value) = result {
            if references_value == 0 {
                panic!("cannot take uninitialized node data");
            } else if references_value == 1 {
                panic!("cannot take locked node data");
            } else {
                let reference_count = references_value - 2;
                panic!("cannot take node data with {reference_count} reference(s)");
            }
        }

        // SAFETY:
        //
        // Since `self.references` was 2, `data` must have been initialized. Since `self.references
        // = 1` indicates that it is locked, we have exclusive access to read from `data`.
        let data = unsafe { self.data.get().assume_init_read() };
        // Transition to an uninitialized state.
        self.references.store(0, Ordering::Release);
        data
    }
}

impl<T> Drop for Data<T> {
    fn drop(&mut self) {
        let references_value = self.references.load(Ordering::Acquire);
        if references_value == 0 {
            return;
        } else if references_value == 1 {
            panic!("cannot drop locked node data");
        } else if references_value != 2 {
            let reference_count = references_value - 2;
            panic!("cannot drop node data with {reference_count} references(s)");
        }

        drop(self.take());
    }
}

/// A thread-safe reference-counted wrapper around the data stored in a [`GraphNode`][gn].
///
/// [gn]: super::GraphNode
pub struct Reference<'node, T> {
    /// The [`Data`] to which this [`Reference`] refers.
    ptr: &'node Data<T>,
}

impl<'node, T> Clone for Reference<'node, T> {
    fn clone(&self) -> Self {
        let mut current = self.ptr.references.load(Ordering::Relaxed);
        loop {
            let Some(new) = current.checked_add(1) else {
                panic!("cannot clone node data reference: too many active references");
            };
            let result = self.ptr.references.compare_exchange_weak(
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

        Self { ptr: self.ptr }
    }
}

impl<'node, T> ops::Deref for Reference<'node, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        //
        // The presence of a [`Reference`] indicates that the associated data is initialized and
        // valid to reference.
        unsafe { self.ptr.data.get().assume_init_ref() }
    }
}

impl<T> Drop for Reference<'_, T> {
    fn drop(&mut self) {
        let references_value = self.ptr.references.fetch_sub(1, Ordering::Release);

        // [`Reference`] instances should only exist while `self.references` is greater than 2,
        // since a value of 0 indicates that the data is uninitialized, a value of 1 indicates that
        // the data is locked, and a value of 2 indicates that the data is initialized and
        // unreferenced.
        debug_assert!(references_value > 2);
    }
}

#[cfg(test)]
mod test {
    use super::Data;

    #[test]
    fn ref_single() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);
        let reference = data.ref_data();
        assert_eq!(*reference, init_val);
    }

    #[test]
    fn ref_many() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let mut references = Vec::new();
        for _ in 0..1000 {
            references.push(data.ref_data());
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

        let reference = data.ref_data();
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

        let reference = data.ref_data();
        assert_eq!(*reference, init_val);
        
        assert_eq!(data.take(), init_val);
    }

    #[test]
    fn ref_clone() {
        let data = Data::new();

        let init_val = 1u32;
        data.initialize(init_val);

        let reference_0 = data.ref_data();
        assert_eq!(*reference_0, init_val);
        let reference_1 = reference_0.clone();
        assert_eq!(*reference_1, init_val);
        assert_eq!(*reference_0, *reference_1);
    }
}

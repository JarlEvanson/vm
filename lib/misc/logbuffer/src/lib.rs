#![no_std]

use core::{
    mem, ptr::{self, NonNull}, slice, sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

pub struct LogBuffer {
    data_ptr: NonNull<AtomicU8>,
    data_capacity: usize,

    logical_start: AtomicUsize,
    logical_end: AtomicUsize,
}

impl LogBuffer {
    /// Creates and initializes a new [`LogBuffer`].
    ///
    /// `data_capacity` must be a power of two.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `data_ptr` must be valid for writes of `data_capacity` consecutive [`AtomicU8`]s.
    /// - `data_ptr` must be properly aligned.
    pub unsafe fn new(data_ptr: NonNull<AtomicU8>, data_capacity: usize) -> Self {
        assert!(data_capacity != 0 && data_capacity.is_power_of_two());
        assert!(data_capacity.strict_mul(mem::size_of::<AtomicU8>()) <= isize::MAX as usize);
        assert!(data_capacity >= mem::size_of::<usize>() && data_capacity >= mem::align_of::<usize>());

        // SAFETY:
        //
        // The invariants of [`LogBuffer::new()`] are a superset of the invariants of this call.
        unsafe { ptr::write_bytes(data_ptr.as_ptr(), 0, data_capacity) }

        let logical_start = 0usize.wrapping_sub(data_capacity);
        let id_start = Self::compute_base_id();

        Self {
            data_ptr,
            data_capacity,

            logical_start: AtomicUsize::new(logical_start),
            logical_end: AtomicUsize::new(logical_start),
        }
    }

    fn data(&self) -> &[AtomicU8] {
        // SAFETY:
        //
        // The invariants of the [`LogBuffer`] creation functions ensures the following:
        //
        // - `self.data_ptr` is non-null, valid for reads and writes for `self.data_capacity`
        //   [`AtomicU8`]s, and properly aligned.
        // - `self.data_ptr` points to `self.data_capacity` properly initialized [`AtomicU8`]s.
        // - `self.data_capacity * mem::size_of<AtomicU8>()` is less than or equal to `isize::MAX`.
        unsafe { slice::from_raw_parts(self.data_ptr.as_ptr(), self.data_capacity) }
    }

    fn base_id(&self) -> usize {
        Self::compute_base_id()
    }

    fn compute_base_id() -> usize {
        0
    }
}

fn compute_block_size(data_size: usize) -> usize {
    data_size
        .strict_add(mem::size_of::<usize>())
        .checked_next_multiple_of(mem::align_of::<usize>())
        .expect("data size is too large")
}

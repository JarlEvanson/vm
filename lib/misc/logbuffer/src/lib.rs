#![no_std]

use core::{
    mem,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

pub struct LogBuffer {
    data_ptr: NonNull<AtomicU8>,
    data_capacity: usize,

    logical_start: AtomicUsize,
    logical_end: AtomicUsize,

    descriptor_ptr: NonNull<Descriptor>,
    descriptor_capacity: usize,
}

impl LogBuffer {
    /// Creates and initializes a new [`LogBuffer`].
    ///
    /// `data_capacity` and `descriptor_capacity` must be powers of two.
    ///
    /// # Safety
    ///
    /// Behavior is undefined if any of the following conditions are violated:
    ///
    /// - `data_ptr` must be valid for writes of `data_capacity` consecutive [`AtomicU8`]s.
    /// - `data_ptr` must be properly aligned.
    /// - `descriptor_ptr` must be valid for writes of `descriptor_capacity` consecutive
    ///   [`Descriptor`]s.
    /// - `descriptor_ptr` must be properly aligned.
    pub unsafe fn new(
        data_ptr: NonNull<AtomicU8>,
        data_capacity: usize,
        descriptor_ptr: NonNull<Descriptor>,
        descriptor_capacity: usize,
        starter_message: &str,
    ) -> Self {
        assert!(data_capacity != 0 && data_capacity.is_power_of_two());
        assert!(data_capacity.strict_mul(mem::size_of::<AtomicU8>()) <= isize::MAX as usize);
        assert!(
            data_capacity >= mem::size_of::<usize>() && data_capacity >= mem::align_of::<usize>()
        );
        assert!(descriptor_capacity != 0 && descriptor_capacity.is_power_of_two());
        assert!(
            descriptor_capacity.strict_mul(mem::size_of::<Descriptor>()) <= isize::MAX as usize
        );

        // SAFETY:
        //
        // The invariants of [`LogBuffer::new()`] are a superset of the invariants of this call.
        unsafe { ptr::write_bytes(data_ptr.as_ptr(), 0, data_capacity) }

        let logical_start = 0usize.wrapping_sub(data_capacity);

        Self {
            data_ptr,
            data_capacity,

            logical_start: AtomicUsize::new(logical_start),
            logical_end: AtomicUsize::new(logical_start),

            descriptor_ptr,
            descriptor_capacity,
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
        // - `self.data_capacity * mem::size_of::<AtomicU8>()` is less than or equal to
        //   `isize::MAX`.
        unsafe { slice::from_raw_parts(self.data_ptr.as_ptr(), self.data_capacity) }
    }

    fn descriptors(&self) -> &[Descriptor] {
        // SAFETY:
        //
        // The invariants of the [`LogBuffer`] creation functions ensure the following:
        //
        // - `self.descriptor_ptr` is non-null, valid for reads and writes for
        //   `self.descriptor_capacity` [`Descriptor`]s, and properly aligned.
        // - `self.descriptor_ptr` points to [`self.descriptor_capacity`] properly initialized
        //   [`Descriptor`]s.
        // - `self.descriptor_capacity * mem::size_of::<Descriptor>()` is less than or equal to
        //   `isize::MAX`.
        unsafe { slice::from_raw_parts(self.descriptor_ptr.as_ptr(), self.descriptor_capacity) }
    }
}

/// Internal data associated with a particular message.
pub struct Descriptor {
    state_id: AtomicDescriptorStateId,
    logical_start: AtomicUsize,
    logical_end: AtomicUsize,
    sequence: AtomicUsize,
}

#[repr(transparent)]
struct AtomicDescriptorStateId(AtomicUsize);

impl AtomicDescriptorStateId {
    const fn new(id: DescriptorStateId) -> Self {
        Self(AtomicUsize::new(id.to_raw()))
    }

    fn load(&self, order: Ordering) -> DescriptorStateId {
        DescriptorStateId::from_raw(self.0.load(order))
    }

    fn store(&self, id: DescriptorStateId, order: Ordering) {
        self.0.store(id.to_raw(), order);
    }

    fn compare_exchange(
        &self,
        current: DescriptorStateId,
        new: DescriptorStateId,
        success: Ordering,
        failure: Ordering,
    ) -> Result<DescriptorStateId, DescriptorStateId> {
        self.0
            .compare_exchange(current.to_raw(), new.to_raw(), success, failure)
            .map(DescriptorStateId::from_raw)
            .map_err(DescriptorStateId::from_raw)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DescriptorStateId(usize);

impl DescriptorStateId {
    const ID_SHIFT: u32 = 0;
    const ID_BASE_MASK: usize = !Self::FLAGS_MASK;
    const ID_MASK: usize = Self::ID_BASE_MASK.strict_shl(Self::ID_SHIFT);

    const FLAGS_SHIFT: u32 = usize::BITS - 2;
    const FLAGS_BASE_MASK: usize = 0b11;
    const FLAGS_MASK: usize = Self::FLAGS_BASE_MASK.strict_shl(Self::FLAGS_SHIFT);

    const fn from_raw(val: usize) -> Self {
        Self(val)
    }

    const fn new(id: DescriptorId, state: DescriptorState) -> Self {
        let raw_state: usize = match state {
            DescriptorState::MissingData => 0b00,
            DescriptorState::Reserved => 0b01,
            DescriptorState::Committed => 0b10,
            DescriptorState::Finalized => 0b11,
        };

        Self(id.to_raw().strict_shl(Self::ID_SHIFT) | (raw_state.strict_shl(Self::FLAGS_SHIFT)))
    }

    const fn id(self) -> DescriptorId {
        DescriptorId::from_raw(self.0.strict_shr(Self::ID_SHIFT) & Self::ID_BASE_MASK)
    }

    const fn state(self) -> DescriptorState {
        match self.0.strict_shr(Self::FLAGS_SHIFT) & Self::FLAGS_BASE_MASK {
            0b00 => DescriptorState::MissingData,
            0b01 => DescriptorState::Reserved,
            0b10 => DescriptorState::Committed,
            0b11 => DescriptorState::Finalized,
            _ => unreachable!(),
        }
    }

    const fn to_raw(self) -> usize {
        self.0
    }
}

#[repr(transparent)]
struct AtomicDescriptorId(AtomicUsize);

impl AtomicDescriptorId {
    const fn new(id: DescriptorId) -> Self {
        Self(AtomicUsize::new(id.to_raw()))
    }

    fn load(&self, order: Ordering) -> DescriptorId {
        DescriptorId::from_raw(self.0.load(order))
    }

    fn store(&self, id: DescriptorId, order: Ordering) {
        self.0.store(id.to_raw(), order);
    }

    fn compare_exchange(
        &self,
        current: DescriptorId,
        new: DescriptorId,
        success: Ordering,
        failure: Ordering,
    ) -> Result<DescriptorId, DescriptorId> {
        self.0
            .compare_exchange(current.to_raw(), new.to_raw(), success, failure)
            .map(DescriptorId::from_raw)
            .map_err(DescriptorId::from_raw)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DescriptorId(usize);

impl DescriptorId {
    /// Creates a new [`DescriptorId`] from the raw value.
    const fn from_raw(val: usize) -> Self {
        Self(val)
    }

    /// Creates a new [`DescriptorId`] from the provided value, truncating as necessary.
    const fn new_truncating(val: usize) -> Self {
        Self(val & DescriptorStateId::ID_MASK)
    }

    /// Returns the previous [`DescriptorId`] in the modular representation.
    const fn prev(self) -> Self {
        Self::new_truncating(self.to_raw().wrapping_sub(1))
    }

    /// Returns the next [`DescriptorId`] in the modular representation.
    const fn next(self) -> Self {
        Self::new_truncating(self.to_raw().wrapping_add(1))
    }

    /// Returns the [`DescriptorId`] associated with the same index, but one wrap earlier.
    const fn prev_wrap(self, descriptor_capacity: usize) -> Self {
        Self::new_truncating(self.to_raw().wrapping_sub(descriptor_capacity))
    }

    /// Returns how many [`DescriptorId`]s `self` is ahead of `lhs`.
    const fn difference(self, lhs: Self) -> usize {
        self.to_raw().wrapping_sub(lhs.to_raw()) & DescriptorStateId::ID_MASK
    }

    /// Returns the raw representation of this [`DescriptorId`].
    const fn to_raw(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DescriptorState {
    /// The slot is empty or the data was overwritten.
    MissingData,
    /// A producer has claimed this slot and is currently writing to it.
    Reserved,
    /// The data is written, and the message is visible, but it might still be "reopened" for more
    /// data.
    Committed,
    /// The message is immutable and ready to be consumed or eventually overwritten.
    Finalized,
}

#![cfg_attr(not(test), no_std)]

use core::{
    mem,
    ptr::{self, NonNull},
    slice,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

pub struct LogBuffer {
    data_ptr: NonNull<AtomicU8>,
    data_capacity: usize,

    // A logical view into the data buffer enables abstracting over the size of the buffer.
    /// The logical position of the next data byte.
    logical_head: AtomicUsize,
    /// The logical position of the last valid data byte.
    logical_tail: AtomicUsize,

    descriptor_ptr: NonNull<Descriptor>,
    descriptor_capacity: usize,

    /// The [`DescriptorId`] of the next [`Descriptor`] to be allocated.
    id_head: AtomicDescriptorId,
    /// The [`DescriptorId`] of the last valid [`Descriptor`].
    id_tail: AtomicDescriptorId,

    base_sequence: usize,
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
    pub const unsafe fn new(
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

        let logical_head = 0usize.wrapping_sub(data_capacity);
        let logical_tail = logical_head;
        let base_sequence = 0usize.wrapping_sub(descriptor_capacity);
        let id_head = DescriptorId::new_truncating(base_sequence);
        let id_tail = id_head;

        let mut logbuffer = Self {
            data_ptr,
            data_capacity,

            logical_head: AtomicUsize::new(logical_head),
            logical_tail: AtomicUsize::new(logical_tail),

            descriptor_ptr,
            descriptor_capacity,

            id_head: AtomicDescriptorId::new(id_head),
            id_tail: AtomicDescriptorId::new(id_tail),

            base_sequence,
        };

        assert!(logbuffer.check_data_block_size(starter_message.len()));

        let data_block_size = LogBuffer::compute_data_block_size(starter_message.len());
        let logical_head = logbuffer.compute_logical_head(logical_tail, data_block_size);

        let base_descriptor = logbuffer.descriptor_mut(id_head);
        *base_descriptor = Descriptor {
            state_id: AtomicDescriptorStateId::new(DescriptorStateId::new(
                id_head,
                DescriptorState::Committed,
            )),
            logical_head: AtomicUsize::new(logical_head),
            logical_tail: AtomicUsize::new(logical_tail),
            sequence: AtomicUsize::new(base_sequence),
        };

        logbuffer.logical_head = AtomicUsize::new(logical_head);
        logbuffer.id_head = AtomicDescriptorId::new(id_head.next());

        let id_data = logbuffer.data_at_pos_mut(logical_tail);
        let id_bytes = id_head.to_raw().to_ne_bytes();

        let mut index = 0;
        while index < id_bytes.len() {
            id_data[index] = AtomicU8::new(id_bytes[index]);
            index += 1;
        }

        let message_data = logbuffer.data_at_pos_mut(logical_tail + mem::size_of::<DescriptorId>());
        let message_bytes = starter_message.as_bytes();

        let mut index = 0;
        while index < message_bytes.len() {
            message_data[index] = AtomicU8::new(message_bytes[index]);
            index += 1;
        }

        logbuffer
    }

    /// Returns `true` if the size of the data is supported.
    const fn check_data_block_size(&self, data_size: usize) -> bool {
        Self::compute_data_block_size(data_size) <= self.data_capacity / 2
    }

    /// Computes the size of the data block given the size of the data.
    const fn compute_data_block_size(size: usize) -> usize {
        let required_size = size.strict_add(mem::size_of::<DescriptorId>());
        required_size
            .checked_next_multiple_of(mem::align_of::<DescriptorId>())
            .expect("padding data block size failed")
    }

    const fn compute_logical_head(&self, logical_tail: usize, size: usize) -> usize {
        let logical_head = logical_tail.wrapping_add(size);

        if !self.is_block_wrapped(logical_tail, logical_head) {
            return logical_tail;
        }

        (logical_head & !self.data_mask()) + size
    }

    const fn is_block_wrapped(&self, logical_tail: usize, logical_head: usize) -> bool {
        let head_wrap_count = logical_head >> self.data_bits();
        let tail_wrap_count = logical_tail >> self.data_bits();
        head_wrap_count != tail_wrap_count
    }

    /// Returns the number of bits required to represent an arbitrary position in the data buffer.
    const fn data_bits(&self) -> u32 {
        self.data_capacity.trailing_zeros()
    }

    /// Returns a bit mask that truncates any indices that aren't valid for the data buffer.
    const fn data_mask(&self) -> usize {
        1usize << self.data_bits()
    }

    const fn data_at_pos(&self, logical: usize) -> &[AtomicU8] {
        let index = logical % self.data_capacity;
        self.data().split_at(index).1
    }

    const fn data_at_pos_mut(&mut self, logical: usize) -> &mut [AtomicU8] {
        let index = logical % self.data_capacity;
        self.data_mut().split_at_mut(index).1
    }

    const fn descriptor(&self, id: DescriptorId) -> &Descriptor {
        let index = id.to_index(self.descriptor_capacity);
        &self.descriptors()[index]
    }

    const fn descriptor_mut(&mut self, id: DescriptorId) -> &mut Descriptor {
        let index = id.to_index(self.descriptor_capacity);
        &mut self.descriptors_mut()[index]
    }

    const fn data(&self) -> &[AtomicU8] {
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

    const fn data_mut(&mut self) -> &mut [AtomicU8] {
        // SAFETY:
        //
        // The invariants of the [`LogBuffer`] creation functions ensures the following:
        //
        // - `self.data_ptr` is non-null, valid for reads and writes for `self.data_capacity`
        //   [`AtomicU8`]s, and properly aligned.
        // - `self.data_ptr` points to `self.data_capacity` properly initialized [`AtomicU8`]s.
        // - `self.data_capacity * mem::size_of::<AtomicU8>()` is less than or equal to
        //   `isize::MAX`.
        unsafe { slice::from_raw_parts_mut(self.data_ptr.as_ptr(), self.data_capacity) }
    }

    const fn descriptors(&self) -> &[Descriptor] {
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

    const fn descriptors_mut(&mut self) -> &mut [Descriptor] {
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
        unsafe { slice::from_raw_parts_mut(self.descriptor_ptr.as_ptr(), self.descriptor_capacity) }
    }
}

/// Internal data associated with a particular message.
pub struct Descriptor {
    state_id: AtomicDescriptorStateId,
    logical_head: AtomicUsize,
    logical_tail: AtomicUsize,
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

    /// Returns the index into the descriptor buffer with which this [`DescriptorId`] is
    /// associated.
    const fn to_index(self, descriptor_capacity: usize) -> usize {
        self.to_raw() % descriptor_capacity
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

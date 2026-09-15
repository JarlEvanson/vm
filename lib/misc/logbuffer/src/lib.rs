use core::{
    fmt,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
    slice,
    sync::atomic::{self, AtomicU8, AtomicUsize, Ordering},
};

pub mod allocated;

const EMPTY_LOGICAL_POS: usize = 0x1;
const FAILED_LOGICAL_POS: usize = 0x3;

const _: () = assert!(!EMPTY_LOGICAL_POS.is_multiple_of(mem::size_of_val(&EMPTY_LOGICAL_POS)));
const _: () = assert!(!EMPTY_LOGICAL_POS.is_multiple_of(mem::align_of_val(&EMPTY_LOGICAL_POS)));
const _: () = assert!(!FAILED_LOGICAL_POS.is_multiple_of(mem::size_of_val(&FAILED_LOGICAL_POS)));
const _: () = assert!(!FAILED_LOGICAL_POS.is_multiple_of(mem::align_of_val(&FAILED_LOGICAL_POS)));

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

    last_finalized_sequence: AtomicUsize,

    initial_sequence: usize,
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

        let logical_head = 0usize.wrapping_sub(data_capacity);
        let logical_tail = logical_head;
        let initial_sequence = 0usize.wrapping_sub(descriptor_capacity);
        let id_head = DescriptorId::new_from_seq(initial_sequence);
        let id_tail = id_head;

        // SAFETY:
        //
        // The invariants of [`LogBuffer::new()`] are a superset of the invariants of this call.
        unsafe { ptr::write_bytes(data_ptr.as_ptr(), 0, data_capacity) }

        // SAFETY:
        //
        // The invariants of [`LogBuffer::new()`] are a superset of the invariants of this call.
        let descriptors = unsafe {
            slice::from_raw_parts_mut(
                descriptor_ptr.as_ptr().cast::<MaybeUninit<Descriptor>>(),
                descriptor_capacity,
            )
        };

        let mut index = 0;
        while index < descriptor_capacity {
            let descriptor = &mut descriptors[index];

            let sequence = initial_sequence
                .wrapping_add(index)
                .wrapping_sub(descriptor_capacity);
            descriptor.write(Descriptor {
                state_id: AtomicDescriptorStateId::new(DescriptorStateId::new(
                    DescriptorId::new_from_seq(sequence),
                    DescriptorState::MissingData,
                )),
                logical_head: AtomicUsize::new(EMPTY_LOGICAL_POS),
                logical_tail: AtomicUsize::new(EMPTY_LOGICAL_POS),
                sequence: AtomicUsize::new(sequence),
            });

            index += 1;
        }

        let mut logbuffer = Self {
            data_ptr,
            data_capacity,

            logical_head: AtomicUsize::new(logical_head),
            logical_tail: AtomicUsize::new(logical_tail),

            descriptor_ptr,
            descriptor_capacity,

            id_head: AtomicDescriptorId::new(id_head),
            id_tail: AtomicDescriptorId::new(id_tail),

            last_finalized_sequence: AtomicUsize::new(initial_sequence),

            initial_sequence,
        };

        assert!(logbuffer.check_data_block_size(starter_message.len()));

        let data_block_size = LogBuffer::compute_data_block_size(starter_message.len());
        let logical_head = logbuffer.compute_logical_head(logical_tail, data_block_size);

        let base_descriptor = logbuffer.descriptor_mut(id_head);
        *base_descriptor = Descriptor {
            state_id: AtomicDescriptorStateId::new(DescriptorStateId::new(
                id_head,
                DescriptorState::Finalized,
            )),
            logical_head: AtomicUsize::new(logical_head),
            logical_tail: AtomicUsize::new(logical_tail),
            sequence: AtomicUsize::new(initial_sequence),
        };

        logbuffer.logical_head = AtomicUsize::new(logbuffer.compute_logical_head(
            logical_tail,
            LogBuffer::compute_data_block_size_padded(starter_message.len()),
        ));
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

    /// Returns the [`Message`] associated with the provided `seqeuence`, or if said message is
    /// gone, the next available [`Message`].
    ///
    /// On success, the user must check [`Message::sequence`] to see which [`Message`] was actually
    /// returned.
    ///
    /// This returns [`None`] if the requested `sequence` is not yet available.
    pub fn read<'buffer>(
        &self,
        sequence: usize,
        buffer: &'buffer mut [u8],
    ) -> Option<Message<'buffer>> {
        let mut sequence = sequence;
        loop {
            let (state_id, data_lost) = loop {
                let expected_id = DescriptorId::new_from_seq(sequence);

                let descriptor = self.descriptor(expected_id);

                let start_state_id = descriptor.state_id.load(Ordering::Acquire);
                if start_state_id != DescriptorStateId::new(expected_id, DescriptorState::Finalized)
                {
                    break (start_state_id, false);
                }

                let loaded_sequence = descriptor.sequence.load(Ordering::Relaxed);
                if loaded_sequence != sequence {
                    break (start_state_id, false);
                }

                let logical_head = descriptor.logical_head.load(Ordering::Relaxed);
                let logical_tail = descriptor.logical_tail.load(Ordering::Relaxed);

                atomic::fence(Ordering::Acquire);

                let middle_state_id = descriptor.state_id.load(Ordering::Relaxed);
                if start_state_id != middle_state_id || logical_tail == FAILED_LOGICAL_POS {
                    break (middle_state_id, logical_tail == FAILED_LOGICAL_POS);
                }

                if logical_tail == EMPTY_LOGICAL_POS {
                    let message = Message {
                        sequence,
                        buffer: &mut [],
                    };

                    return Some(message);
                }

                let (block_pos, mut data_size) =
                    if !self.is_block_wrapped(logical_tail, logical_head) {
                        (logical_tail, logical_head.strict_sub(logical_tail))
                    } else {
                        let block_pos = logical_head & !self.data_mask();
                        let data_size = logical_head - block_pos;

                        (block_pos, data_size)
                    };

                data_size -= mem::size_of::<DescriptorId>();

                let data = self.data_at_pos(block_pos + mem::size_of::<DescriptorId>());
                let data = &data[..data_size];

                let copied_size = buffer.len().min(data_size);

                for (buffer_byte, data_byte) in buffer.iter_mut().zip(data.iter()).take(copied_size)
                {
                    *buffer_byte = data_byte.load(Ordering::Relaxed);
                }

                atomic::fence(Ordering::Acquire);

                let end_state_id = descriptor.state_id.load(Ordering::Relaxed);
                if start_state_id != end_state_id {
                    break (end_state_id, false);
                }

                let message = Message {
                    sequence,
                    buffer: &mut buffer[..copied_size],
                };

                return Some(message);
            };

            let tail_sequence = self.tail_sequence_internal();
            if sequence < tail_sequence {
                sequence = tail_sequence;
            } else if state_id
                == DescriptorStateId::new(
                    DescriptorId::new_from_seq(sequence),
                    DescriptorState::MissingData,
                )
                || data_lost
            {
                sequence = sequence.wrapping_add(1);
            } else {
                return None;
            }
        }
    }

    /// Reserves at least `size` bytes in this [`LogBuffer`].
    pub fn reserve(&self, size: usize) -> Option<ReservedMessage<'_>> {
        if !self.check_data_block_size(size) {
            return None;
        }

        let mut id_head = self.id_head.load(Ordering::Acquire);
        let id = loop {
            let id = id_head.next();
            let id_previous_wrap = id.prev_wrap(self.descriptor_capacity);

            if id_previous_wrap == self.id_tail.load(Ordering::Relaxed) {
                todo!("handle descriptor invalidation")
            }

            if let Err(actual_id_head) =
                self.id_head
                    .compare_exchange(id_head, id, Ordering::AcqRel, Ordering::Acquire)
            {
                id_head = actual_id_head;
            } else {
                break id;
            }
        };

        let descriptor = self.descriptor(id);

        let current_state_id = descriptor.state_id.load(Ordering::Acquire);
        if current_state_id
            != DescriptorStateId::new(
                id.prev_wrap(self.descriptor_capacity),
                DescriptorState::MissingData,
            )
        {
            todo!("handle ABA issue");
        }

        let new_state_id = DescriptorStateId::new(id, DescriptorState::Reserved);
        if descriptor
            .state_id
            .compare_exchange(
                current_state_id,
                new_state_id,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        {
            todo!("handle ABA issue");
        }

        descriptor
            .sequence
            .fetch_add(self.descriptor_capacity, Ordering::Relaxed);

        let prev_descriptor = self.descriptor(id.prev());
        let expected_state_id = DescriptorStateId::new(id.prev(), DescriptorState::Committed);
        let new_state_id = DescriptorStateId::new(id.prev(), DescriptorState::Finalized);
        if prev_descriptor
            .state_id
            .compare_exchange(
                expected_state_id,
                new_state_id,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
        {
            self.update_last_finalized();
        }

        todo!()
    }

    /// Returns the sequence number associated with the tail [`Message`].
    pub fn tail_sequence(&self) -> usize {
        let Some(message) = self.read(self.tail_sequence_internal(), &mut []) else {
            todo!()
        };

        message.sequence
    }

    pub const fn initial_sequence(&self) -> usize {
        self.initial_sequence
    }

    fn allocate_data(&self, id: DescriptorId, size: usize) -> (usize, usize) {
        if size == 0 {
            return (EMPTY_LOGICAL_POS, EMPTY_LOGICAL_POS);
        }

        let data_block_size = Self::compute_data_block_size(size);

        // The current logical head is the logical tail of the newly allocated data block.
        let mut logical_tail = self.logical_head.load(Ordering::Acquire);
        let logical_head = loop {
            let logical_head = self.compute_logical_head(logical_tail, data_block_size);
            let logical_head_padded = self.compute_logical_head(logical_tail, Self::compute_data_block_size_padded(size));

            if let Err(value) = self.logical_head.compare_exchange(logical_tail, logical_head_padded, Ordering::AcqRel, Ordering::Acquire) {
                logical_tail = value;
                continue;
            }

            break logical_head;
        };

        todo!()
    }

    fn update_last_finalized(&self) {
        let mut old_sequence = self.last_finalized_sequence.load(Ordering::Acquire);

        loop {
            let mut finalized_sequence = old_sequence;
            let mut try_sequence = finalized_sequence.wrapping_add(1);

            while let Some(message) = self.read(try_sequence, &mut []) {
                finalized_sequence = message.sequence;
                try_sequence = finalized_sequence.wrapping_add(1);
            }

            if finalized_sequence == old_sequence {
                return;
            }

            match self.last_finalized_sequence.compare_exchange(
                old_sequence,
                finalized_sequence,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(sequence) => old_sequence = sequence,
            }
        }
    }

    /// Returns the sequence number associated with the tail [`DescriptorId`].
    fn tail_sequence_internal(&self) -> usize {
        loop {
            let id_tail = self.id_tail.load(Ordering::Acquire);

            let descriptor = self.descriptor(id_tail);

            let start_state_id = descriptor.state_id.load(Ordering::Acquire);

            let sequence = descriptor.sequence.load(Ordering::Relaxed);

            atomic::fence(Ordering::Acquire);

            let end_state_id = descriptor.state_id.load(Ordering::Relaxed);
            if start_state_id != end_state_id || start_state_id.id() != id_tail {
                continue;
            }

            if matches!(
                start_state_id.state(),
                DescriptorState::MissingData | DescriptorState::Finalized
            ) {
                return sequence;
            }

            core::hint::spin_loop();
        }
    }

    /// Returns `true` if the size of the data is supported.
    const fn check_data_block_size(&self, data_size: usize) -> bool {
        Self::compute_data_block_size_padded(data_size) <= self.data_capacity / 2
    }

    /// Computes the size of the data block given the size of the data.
    const fn compute_data_block_size(size: usize) -> usize {
        size.strict_add(mem::size_of::<DescriptorId>())
    }

    /// Computes the size of the data block and its padding given the size of the data.
    const fn compute_data_block_size_padded(size: usize) -> usize {
        Self::compute_data_block_size(size)
            .checked_next_multiple_of(mem::align_of::<DescriptorId>())
            .expect("padding data block size failed")
    }

    /// Returns the `logical_head` associated with the provided `logical_tail` and size.
    ///
    /// This can be used with the results of [`Self::compute_data_block_size()`] and
    /// [`Self::compute_data_block_size_padded()`].
    const fn compute_logical_head(&self, logical_tail: usize, size: usize) -> usize {
        let logical_head = logical_tail.wrapping_add(size);

        if !self.is_block_wrapped(logical_tail, logical_head) {
            return logical_head;
        }

        (logical_head & !self.data_mask()) + size
    }

    const fn is_block_wrapped(&self, logical_tail: usize, logical_head: usize) -> bool {
        let head_wrap_count = logical_head.wrapping_sub(1) >> self.data_bits();
        let tail_wrap_count = logical_tail >> self.data_bits();
        head_wrap_count != tail_wrap_count
    }

    /// Returns the number of bits required to represent an arbitrary position in the data buffer.
    const fn data_bits(&self) -> u32 {
        self.data_capacity.trailing_zeros()
    }

    /// Returns a bit mask that truncates any indices that aren't valid for the data buffer.
    const fn data_mask(&self) -> usize {
        (1usize << self.data_bits()) - 1
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

/// A log message, with its associated sequence number, metadata, and the extracted part of the
/// data buffer.
pub struct Message<'buffer> {
    /// The sequence number of the [`Message`].
    pub sequence: usize,
    /// Extracted portion of the data buffer associated with this [`Message`].
    pub buffer: &'buffer mut [u8],
}

/// A reserved segment of the [`Ringbuffer`].
pub struct ReservedMessage<'buffer> {
    logbuffer: &'buffer LogBuffer,
    sequence: usize,
    buffer: &'buffer [AtomicU8],
}

impl<'buffer> ReservedMessage<'buffer> {
    /// Returns the sequence of the [`ReservedMessage`].
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    /// Returns the [`ReservedMessage`] buffer.
    pub fn buffer(&self) -> &[AtomicU8] {
        self.buffer
    }

    /// Commits the [`ReservedMessage`] into the associated [`LogBuffer`].
    pub fn commit(self) -> CommittedMessage<'buffer> {
        let descriptor_id = DescriptorId::new_from_seq(self.sequence);
        let descriptor = self.logbuffer.descriptor(descriptor_id);

        let current_state_id = DescriptorStateId::new(descriptor_id, DescriptorState::Reserved);
        let new_state_id = DescriptorStateId::new(descriptor_id, DescriptorState::Committed);

        if descriptor
            .state_id
            .compare_exchange(
                current_state_id,
                new_state_id,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        {
            unreachable!()
        }

        let id_head = self.logbuffer.id_head.load(Ordering::Acquire);
        if id_head != descriptor_id {
            let current_state_id = new_state_id;
            let new_state_id = DescriptorStateId::new(descriptor_id, DescriptorState::Finalized);

            if descriptor
                .state_id
                .compare_exchange(
                    current_state_id,
                    new_state_id,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                self.logbuffer.update_last_finalized();
            }
        }

        CommittedMessage {
            logbuffer: self.logbuffer,
            sequence: self.sequence,
        }
    }

    /// Finalizes the [`ReservedMessage`], thereby closing the editing window.
    pub fn finalize(self) {
        let descriptor_id = DescriptorId::new_from_seq(self.sequence);
        let descriptor = self.logbuffer.descriptor(descriptor_id);

        let current_state_id = DescriptorStateId::new(descriptor_id, DescriptorState::Reserved);
        let new_state_id = DescriptorStateId::new(descriptor_id, DescriptorState::Finalized);

        if descriptor
            .state_id
            .compare_exchange(
                current_state_id,
                new_state_id,
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        {
            unreachable!()
        }

        self.logbuffer.update_last_finalized();
    }
}

/// A committed [`Message`].
///
/// The associated [`Message`] might be able to be reopened for editing.
pub struct CommittedMessage<'buffer> {
    logbuffer: &'buffer LogBuffer,
    sequence: usize,
}

impl<'buffer> CommittedMessage<'buffer> {
    /// Returns the sequence of the [`ReservedMessage`].
    pub fn sequence(&self) -> usize {
        self.sequence
    }

    /// Attempts to reopen the [`CommittedMessage`] for additional manipulation.
    pub fn reopen(self, additional_size: usize) -> Result<ReservedMessage<'buffer>, Self> {
        todo!()
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

impl fmt::Display for DescriptorStateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("DescriptorStateId");

        debug_struct.field("state", &self.state());
        debug_struct.field("id", &self.id());

        debug_struct.finish()
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

    const fn new_from_seq(seq: usize) -> Self {
        Self::new_truncating(seq)
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

#[cfg(test)]
mod test {
    use crate::allocated::AllocatedLogBuffer;

    #[test]
    fn initial_read() {
        let logbuffer = AllocatedLogBuffer::new(128 * 1024, 1024, "TEST");

        let mut sequence = logbuffer.tail_sequence();
        let mut buffer = [0; 4096];

        let message = logbuffer.read(sequence, &mut buffer).unwrap();
        assert!(logbuffer.read(message.sequence + 1, &mut buffer).is_none());
    }

    #[test]
    fn reserve_write_and_finalize() {
        let logbuffer = AllocatedLogBuffer::new(128 * 1024, 1024, "INIT");

        let msg_payload = b"Hello, World!";
        let reserved = logbuffer
            .reserve(msg_payload.len())
            .expect("Failed to reserve memory block");

        let reserved_seq = reserved.sequence();

        // Write data to atomic slice buffer
        for (i, &byte) in msg_payload.iter().enumerate() {
            reserved.buffer()[i].store(byte, Ordering::Relaxed);
        }

        // Finalize the message directly
        reserved.finalize();

        // Read and verify the newly appended message
        let mut read_buf = [0u8; 256];
        let message = logbuffer
            .read(reserved_seq, &mut read_buf)
            .expect("Expected message to be readable");

        assert_eq!(message.sequence, reserved_seq);
        assert_eq!(message.buffer, msg_payload);
    }
}

#![cfg_attr(target_os = "none", no_std)]

use core::{
    fmt,
    ptr::NonNull,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

#[cfg(not(target_has_atomic = "64"))]
use core::sync::atomic::AtomicU32;
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::AtomicU64;

#[cfg(not(all(
    target_has_atomic = "8",
    any(target_has_atomic = "64", target_has_atomic = "32"),
    target_has_atomic = "ptr"
)))]
compile_error!("logbuffer requires AtomicU8, AtomicUsize, and either AtomicU64 or AtomicU32");

#[derive(Debug)]
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

    /// The sequence of the last [`Descriptor`] that was finalized.
    #[cfg(not(target_has_atomic = "64"))]
    last_finalized_sequence: AtomicU32,
    /// The sequence of the last [`Descriptor`] that was finalized.
    #[cfg(target_has_atomic = "64")]
    last_finalized_sequence: AtomicU64,
}

impl LogBuffer {
    /// Creates and initializes a new [`LogBuffer`].
    ///
    /// - Both `data_capacity` and `descriptor_capacity` must be a power of two.
    /// - `starter_message.len()` must be less than or equal to half of `data_capacity`.
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
        todo!()
    }

    /// Returns the [`Message`] associated with the provided `sequence`. If the associated
    /// [`Message`] is gone, the next available [`Message`] is returned.
    ///
    /// On success, the user must check [`Message::sequence()`] to see which [`Message`] was actually
    /// returned.
    ///
    /// This returns [`None`] if the requested `sequence` is not yet available.
    pub fn read<'buffer>(
        &self,
        sequence: usize,
        buffer: &'buffer mut [u8],
    ) -> Option<Message<'buffer>> {
        todo!()
    }

    /// Reserves at least `size` bytes in this [`LogBuffer`].
    pub fn reserve(&self, size: usize) -> Option<ReservedMessage<'_>> {
        todo!()
    }

    /// Returns the sequence number that will be associated with the next [`Message`] available for
    /// readers.
    pub fn next_read_sequence(&self) -> u64 {
        todo!()
    }

    /// Returns the sequence number that will be associated with the next [`Message`] to be
    /// reserved.
    pub fn next_reserved_sequence(&self) -> u64 {
        todo!()
    }

    /// Returns the sequence number associated with the tail [`Message`].
    ///
    /// This is the oldest (from the context of the [`LogBuffer`]) [`Message`] and will be deleted
    /// before any other [`Message`] in the [`LogBuffer`].
    pub fn tail_sequence(&self) -> u64 {
        todo!()
    }
}

/// A log message, with its associated sequence number, metadata, and the extracted part of the
/// data buffer.
#[derive(Debug)]
pub struct Message<'buffer> {
    /// The sequence number of the [`Message`].
    sequence: u64,
    /// Extracted portion of the data buffer associated with this [`Message`].
    buffer: &'buffer mut str,
}

impl<'buffer> Message<'buffer> {
    fn new(sequence: u64, buffer: &'buffer mut [u8]) -> Self {
        debug_assert!(
            str::from_utf8(buffer).is_ok(),
            "message contents must be UTF-8"
        );

        // SAFETY:
        //
        // [`LogBuffer`] requires each and every buffer to be UTF-8.
        let buffer = unsafe { str::from_utf8_unchecked_mut(buffer) };
        Self { sequence, buffer }
    }

    /// Returns the sequence number associated with this [`Message`].
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns an immutable reference to the buffer that was provided when retrieving this
    /// [`Message`], truncated to the message data length.
    pub const fn buffer(&self) -> &str {
        self.buffer
    }

    /// Returns a mutable reference to the buffer that was provided when retrieving this
    /// [`Message`], truncated to the message data length.
    pub const fn buffer_mut(&mut self) -> &mut str {
        self.buffer
    }
}

/// A reserved segment of the [`LogBuffer`].
#[derive(Debug)]
pub struct ReservedMessage<'buffer> {
    logbuffer: &'buffer LogBuffer,
    sequence: u64,
    buffer: &'buffer [AtomicU8],
}

impl<'buffer> ReservedMessage<'buffer> {
    /// Returns the sequence associated with this [`ReservedMessage`].
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns an immutable reference to the buffer that was provided when retrieving this
    /// [`Message`], truncated to the message data length.
    pub const fn buffer(&self) -> &[AtomicU8] {
        self.buffer
    }

    /// Commits the [`ReservedMessage`] into the associated [`LogBuffer`].
    pub fn commit(self) -> CommittedMessage<'buffer> {
        todo!()
    }

    /// Finalizes the [`ReservedMessage`], thereby closing the editing window.
    pub fn finalize(self) {
        todo!()
    }
}

/// A committed [`Message`] which might be able to be reopened for editing.
#[derive(Debug)]
pub struct CommittedMessage<'buffer> {
    logbuffer: &'buffer LogBuffer,
    sequence: u64,
}

impl<'buffer> CommittedMessage<'buffer> {
    /// Returns the sequence of the [`ReservedMessage`].
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Attempts to reopen the [`CommittedMessage`] for additional manipulation.
    pub fn reopen(self, additional_size: usize) -> Result<ReservedMessage<'buffer>, Self> {
        todo!()
    }
}

/// Internal data associated with a particular message.
#[derive(Debug)]
pub struct Descriptor {
    state_id: AtomicDescriptorStateId,
    logical_head: AtomicUsize,
    logical_tail: AtomicUsize,
    #[cfg(target_has_atomic = "64")]
    sequence: AtomicU64,
    #[cfg(not(target_has_atomic = "64"))]
    sequence: [AtomicU32; 2],
}

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
#[derive(Debug)]
struct AtomicDescriptorStateId(AtomicUsize);

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
#[derive(Debug)]
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
        debug_assert!(descriptor_capacity.is_power_of_two());

        Self::new_truncating(self.to_raw().wrapping_sub(descriptor_capacity))
    }

    /// Returns the index into the descriptor buffer with which this [`DescriptorId`] is
    /// associated.
    const fn to_index(self, descriptor_capacity: usize) -> usize {
        debug_assert!(descriptor_capacity != 0);
        debug_assert!(descriptor_capacity.is_power_of_two());

        self.to_raw() & (descriptor_capacity - 1)
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

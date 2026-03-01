//! Structures related to physical memory and its management.

use core::fmt;

use conversion::usize_to_u64;
use memory::address::{Address, AddressChunk, AddressChunkRange};

use crate::memory::page_frame_size;

/// An address in the physical memory space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(Address);

impl PhysicalAddress {
    /// Creates a new [`PhysicalAddress`] with a value of 0.
    pub const fn zero() -> Self {
        Self(Address::zero())
    }

    /// Creates a new [`PhysicalAddress`] with a value of `value`.
    pub const fn new(value: u64) -> PhysicalAddress {
        Self(Address::new(value))
    }

    /// Returns the underlying `u64` value for this [`PhysicalAddress`].
    pub const fn value(self) -> u64 {
        self.0.value()
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes higher.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_add(self, count: u64) -> Option<Self> {
        let Some(value) = self.0.checked_add(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes higher.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_add(self, count: u64) -> Self {
        Self(self.0.strict_add(count))
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes lower.
    ///
    /// Returns `None` if the operation would unerflow.
    pub const fn checked_sub(self, count: u64) -> Option<Self> {
        let Some(value) = self.0.checked_sub(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes lower.
    ///
    /// Panics if the operation would underflow.
    pub const fn strict_sub(self, count: u64) -> Self {
        Self(self.0.strict_sub(count))
    }

    /// Returns `true` if the [`PhysicalAddress`] is a multiple of `alignment`.
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0.is_aligned(alignment)
    }

    /// Returns the greatest [`PhysicalAddress`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    pub const fn align_down(self, alignment: u64) -> Self {
        Self(self.0.align_down(alignment))
    }

    /// Returns the smallest [`PhysicalAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_align_up(self, alignment: u64) -> Option<Self> {
        let Some(value) = self.0.checked_align_up(alignment) else {
            return None;
        };

        Some(Self(value))
    }

    /// Returns the smallest [`PhysicalAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_align_up(self, alignment: u64) -> Self {
        Self(self.0.strict_align_up(alignment))
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysicalAddress({:#0x})", self.value())
    }
}

impl fmt::Display for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#0x}p", self.value())
    }
}

/// A chunk of physical memory aligned to a frame boundary.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(AddressChunk);

impl Frame {
    /// Creates a new [`Frame`] with a value of 0.
    pub const fn zero() -> Self {
        Self(AddressChunk::zero())
    }

    /// Creates a new [`Frame`] with a value of `value`.
    pub const fn new(value: u64) -> Self {
        Self(AddressChunk::new(value))
    }

    /// Returns the [`Frame`] in which `address` is contained.
    pub fn containing_address(address: PhysicalAddress) -> Self {
        Self(AddressChunk::containing_address(
            address.0,
            usize_to_u64(page_frame_size()),
        ))
    }

    /// Returns the underlying `u64` value for this [`Frame`].
    ///
    /// This is a `page_frame_size()`-sized indexing of physical memory.
    pub const fn number(self) -> u64 {
        self.0.number()
    }

    /// Returns the [`PhysicalAddress`] at the start of this [`Frame`].
    pub fn start_address(self) -> PhysicalAddress {
        PhysicalAddress(self.0.start_address(usize_to_u64(page_frame_size())))
    }

    /// Returns the [`PhysicalAddress`] at the end of this [`Frame`].
    ///
    /// This is an inclusive end.
    pub fn end_address_inclusive(self) -> PhysicalAddress {
        PhysicalAddress(
            self.0
                .end_address_inclusive(usize_to_u64(page_frame_size())),
        )
    }

    /// Returns the [`PhysicalAddress`] at the end of this [`Frame`].
    ///
    /// This is an exclusive end.
    pub fn end_address_exclusive(self) -> PhysicalAddress {
        PhysicalAddress(
            self.0
                .end_address_exclusive(usize_to_u64(page_frame_size())),
        )
    }

    /// Creates a new [`Frame`] that is `count` [`Frame`]s higher.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_add(self, count: u64) -> Option<Self> {
        let Some(value) = self.0.checked_add(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`Frame`] that is `count` [`Frame`]s higher.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_add(self, count: u64) -> Self {
        Self(self.0.strict_add(count))
    }

    /// Creates a new [`Frame`] that is `count` [`Frame`]s lower.
    ///
    /// Returns `None` if the operation would unerflow.
    pub const fn checked_sub(self, count: u64) -> Option<Self> {
        let Some(value) = self.0.checked_sub(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`Frame`] that is `count` [`Frame`]s lower.
    ///
    /// Panics if the operation would underflow.
    pub const fn strict_sub(self, count: u64) -> Self {
        Self(self.0.strict_sub(count))
    }

    /// Returns `true` if the [`Frame`] is a multiple of `alignment`.
    ///
    /// `alignment` is given in bytes.
    pub fn is_aligned(self, alignment: u64) -> bool {
        self.0
            .is_aligned(usize_to_u64(page_frame_size()), alignment)
    }

    /// Returns the greatest [`Frame`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// `alignment` is given in bytes.
    pub fn align_down(self, alignment: u64) -> Self {
        Self(
            self.0
                .align_down(usize_to_u64(page_frame_size()), alignment),
        )
    }

    /// Returns the smallest [`Frame`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Returns `None` if the operation would overflow.
    pub fn checked_align_up(self, alignment: u64) -> Option<Self> {
        let value = self
            .0
            .checked_align_up(usize_to_u64(page_frame_size()), alignment)?;

        Some(Self(value))
    }

    /// Returns the smallest [`Frame`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Panics if the operation would overflow.
    pub fn strict_align_up(self, alignment: u64) -> Self {
        Self(
            self.0
                .strict_align_up(usize_to_u64(page_frame_size()), alignment),
        )
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame({:#0x})", self.number())
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame {:#0x}", self.number())
    }
}

/// A range of contiguous [`Frame`]s in physical memory.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameRange(AddressChunkRange);

impl FrameRange {
    /// Creates an empty [`FrameRange`].
    pub const fn empty() -> Self {
        FrameRange(AddressChunkRange::empty())
    }

    /// Returns a new [`FrameRange`] with a base of `start` that contains `count` [`Frame`]s.
    pub const fn new(start: Frame, count: u64) -> Self {
        Self(AddressChunkRange::new(start.0, count))
    }

    /// Creates a new [`FrameRange`] between the two [`Frame`]s.
    ///
    /// If `end` is less than `start`, the [`FrameRange`] is empty.
    pub const fn from_inclusive(start: Frame, end: Frame) -> Self {
        Self(AddressChunkRange::from_inclusive(start.0, end.0))
    }

    /// Creates a new [`FrameRange`] between the two [`Frame`]s.
    ///
    /// If `end` is less than `start`, the [`FrameRange`] is empty.
    pub const fn from_exclusive(start: Frame, end: Frame) -> Self {
        Self(AddressChunkRange::from_exclusive(start.0, end.0))
    }

    /// Returns the [`Frame`] at the start of this [`FrameRange`].
    pub const fn start(self) -> Frame {
        Frame(self.0.start())
    }

    /// Returns the [`PhysicalAddress`] at the start of this [`FrameRange`].
    pub fn start_address(self) -> PhysicalAddress {
        PhysicalAddress(
            self.0
                .start()
                .start_address(usize_to_u64(page_frame_size())),
        )
    }

    /// Returns the [`Frame`] at the end of this [`FrameRange`].
    ///
    /// This is an inclusive end.
    pub const fn end_inclusive(self) -> Frame {
        Frame(self.0.end_inclusive())
    }

    /// Returns the [`PhysicalAddress`] at the end of this [`FrameRange`].
    ///
    /// This is an inclusive end.
    pub fn end_address_inclusive(self) -> PhysicalAddress {
        PhysicalAddress(
            self.0
                .end_inclusive()
                .end_address_inclusive(usize_to_u64(page_frame_size())),
        )
    }

    /// Returns the [`Frame`] at the end of this [`FrameRange`].
    ///
    /// This is an exclusive end.
    pub const fn end_exclusive(self) -> Frame {
        Frame(self.0.end_exclusive())
    }

    /// Returns the [`PhysicalAddress`] at the end of this [`FrameRange`].
    ///
    /// This is an exclusive end.
    pub fn end_address_exclusive(self) -> PhysicalAddress {
        PhysicalAddress(
            self.0
                .end_exclusive()
                .start_address(usize_to_u64(page_frame_size())),
        )
    }

    /// Returns the number of [`Frame`]s in this [`FrameRange`].
    pub const fn count(self) -> u64 {
        self.0.count()
    }

    /// Returns the number of bytes in this [`FrameRange`].
    pub fn byte_count(self) -> u64 {
        self.0.byte_count(usize_to_u64(page_frame_size()))
    }

    /// Returns `true` if the [`FrameRange`] is empty.
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the provided [`Frame`] is contained in this [`FrameRange`].
    pub const fn contains(self, frame: Frame) -> bool {
        self.0.contains(frame.0)
    }

    /// Splits this [`FrameRange`] into two seperate [`FrameRange`]s.
    ///
    /// - [start : start + index)
    /// - [start + index : end)
    ///
    /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the second
    /// [`FrameRange`] will be empty.
    pub const fn split_at_index(self, index: u64) -> Option<(Self, Self)> {
        let Some((a, b)) = self.0.split_at_index(index) else {
            return None;
        };

        Some((FrameRange(a), FrameRange(b)))
    }

    /// Splits this [`FrameRange`] into two seperate [`FrameRange`]s.
    ///
    /// - [start : at)
    /// - [at : end)
    ///
    /// - If `at_frame` == `self.start()`, the first [`FrameRange`] will be empty
    /// - If `at_frame` == `self.end()`, the second [`FrameRange`] will be empty.
    ///
    /// Returns `None` if `at` is not adjacent or contained within this [`FrameRange`]. Adjacent
    /// `at`s will produce empty [`FrameRange`]s on either side.
    pub const fn split_at(self, at: Frame) -> Option<(Self, Self)> {
        let Some((a, b)) = self.0.split_at(at.0) else {
            return None;
        };

        Some((FrameRange(a), FrameRange(b)))
    }

    /// Returns `true` if `self` and `other` share at least one [`Frame`] in their [`FrameRange`]s.
    pub const fn overlaps(self, other: Self) -> bool {
        self.0.overlaps(other.0)
    }

    /// Returns the merged [`FrameRange`] if the two provided [`FrameRange`]s are adjacent or
    /// overlapping.
    ///
    /// Otherwise, `None` is returned.
    pub const fn merge(self, other: Self) -> Option<Self> {
        let Some(value) = self.0.merge(other.0) else {
            return None;
        };

        Some(FrameRange(value))
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`FrameRange`]s do not overlap, then an empty [`FrameRange`] is returned.
    pub const fn intersection(self, other: Self) -> Self {
        FrameRange(self.0.intersection(other.0))
    }

    /// Partitions `self` into three disjoint [`FrameRange`]s relative to `other`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`Frame`]s in
    /// `self` according to their position relative to `other`:
    ///
    /// - `lower`   — [`Frame`]s in `self` strictly below `other`
    /// - `overlap` — [`Frame`]s in `self` that intersect `other`
    /// - `upper`   — [`Frame`]s in `self` strictly above `other`
    pub const fn partition(self, other: Self) -> (Self, Self, Self) {
        let (lower, middle, upper) = self.0.partition(other.0);
        (FrameRange(lower), FrameRange(middle), FrameRange(upper))
    }

    /// Returns an [`Iterator`] over all the frames in this [`FrameRange`].
    pub fn iter(self) -> impl Iterator<Item = Frame> {
        self.0.iter().map(Frame)
    }
}

impl fmt::Debug for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..={:?}", self.start(), self.end_exclusive())
    }
}

impl fmt::Display for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..={}", self.start(), self.end_exclusive())
    }
}

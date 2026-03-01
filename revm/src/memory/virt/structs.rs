//! Structures related to virtual memory and its management.

use core::fmt;

use crate::memory::page_frame_size;

/// An address in the virtual memory space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(memory::address::VirtualAddress);

impl VirtualAddress {
    /// Creates a new [`VirtualAddress`] with a value of 0.
    pub const fn zero() -> Self {
        Self(memory::address::VirtualAddress::zero())
    }

    /// Creates a new [`VirtualAddress`] with a value of `value`.
    pub const fn new(value: usize) -> VirtualAddress {
        Self(memory::address::VirtualAddress::new(value))
    }

    /// Returns the underlying `usize` value for this [`VirtualAddress`].
    pub const fn value(self) -> usize {
        self.0.value()
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes higher.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_add(self, count: usize) -> Option<Self> {
        let Some(value) = self.0.checked_add(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes higher.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_add(self, count: usize) -> Self {
        Self(self.0.strict_add(count))
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes lower.
    ///
    /// Returns `None` if the operation would unerflow.
    pub const fn checked_sub(self, count: usize) -> Option<Self> {
        let Some(value) = self.0.checked_sub(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes lower.
    ///
    /// Panics if the operation would underflow.
    pub const fn strict_sub(self, count: usize) -> Self {
        Self(self.0.strict_sub(count))
    }

    /// Returns `true` if the [`VirtualAddress`] is a multiple of `alignment`.
    pub const fn is_aligned(self, alignment: usize) -> bool {
        self.0.is_aligned(alignment)
    }

    /// Returns the greatest [`VirtualAddress`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    pub const fn align_down(self, alignment: usize) -> Self {
        Self(self.0.align_down(alignment))
    }

    /// Returns the smallest [`VirtualAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_align_up(self, alignment: usize) -> Option<Self> {
        let Some(value) = self.0.checked_align_up(alignment) else {
            return None;
        };

        Some(Self(value))
    }

    /// Returns the smallest [`VirtualAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_align_up(self, alignment: usize) -> Self {
        Self(self.0.strict_align_up(alignment))
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtualAddress({:#0x})", self.value())
    }
}

impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#0x}p", self.value())
    }
}

/// A chunk of virtual memory aligned to a page boundary.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(memory::address::Page);

impl Page {
    /// Creates a new [`Page`] with a value of 0.
    pub const fn zero() -> Self {
        Self(memory::address::Page::zero())
    }

    /// Creates a new [`Page`] with a value of `value`.
    pub const fn new(value: usize) -> Self {
        Self(memory::address::Page::new(value))
    }

    /// Returns the [`Page`] in which `address` is contained.
    pub fn containing_address(address: VirtualAddress) -> Self {
        Self(memory::address::Page::containing_address(
            address.0,
            page_frame_size(),
        ))
    }

    /// Returns the underlying `usize` value for this [`Page`].
    ///
    /// This is a `page_frame_size()`-sized indexing of virtual memory.
    pub const fn number(self) -> usize {
        self.0.number()
    }

    /// Returns the [`VirtualAddress`] at the start of this [`Page`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0.start_address(page_frame_size()))
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    ///
    /// This is an inclusive end.
    pub fn end_address_inclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_inclusive(page_frame_size()))
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    ///
    /// This is an exclusive end.
    pub fn end_address_exclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_exclusive(page_frame_size()))
    }

    /// Creates a new [`Page`] that is `count` [`Page`]s higher.
    ///
    /// Returns `None` if the operation would overflow.
    pub const fn checked_add(self, count: usize) -> Option<Self> {
        let Some(value) = self.0.checked_add(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`Page`] that is `count` [`Page`]s higher.
    ///
    /// Panics if the operation would overflow.
    pub const fn strict_add(self, count: usize) -> Self {
        Self(self.0.strict_add(count))
    }

    /// Creates a new [`Page`] that is `count` [`Page`]s lower.
    ///
    /// Returns `None` if the operation would unerflow.
    pub const fn checked_sub(self, count: usize) -> Option<Self> {
        let Some(value) = self.0.checked_sub(count) else {
            return None;
        };

        Some(Self(value))
    }

    /// Creates a new [`Page`] that is `count` [`Page`]s lower.
    ///
    /// Panics if the operation would underflow.
    pub const fn strict_sub(self, count: usize) -> Self {
        Self(self.0.strict_sub(count))
    }

    /// Returns `true` if the [`Page`] is a multiple of `alignment`.
    ///
    /// `alignment` is given in bytes.
    pub fn is_aligned(self, alignment: usize) -> bool {
        self.0.is_aligned(page_frame_size(), alignment)
    }

    /// Returns the greatest [`Page`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// `alignment` is given in bytes.
    pub fn align_down(self, alignment: usize) -> Self {
        Self(self.0.align_down(page_frame_size(), alignment))
    }

    /// Returns the smallest [`Page`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Returns `None` if the operation would overflow.
    pub fn checked_align_up(self, alignment: usize) -> Option<Self> {
        let value = self.0.checked_align_up(page_frame_size(), alignment)?;

        Some(Self(value))
    }

    /// Returns the smallest [`Page`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    ///
    /// Panics if the operation would overflow.
    pub fn strict_align_up(self, alignment: usize) -> Self {
        Self(self.0.strict_align_up(page_frame_size(), alignment))
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({:#0x})", self.number())
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page {:#0x}", self.number())
    }
}

/// A range of contiguous [`Page`]s in virtual memory.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageRange(memory::address::PageRange);

impl PageRange {
    /// Creates an empty [`PageRange`].
    pub const fn empty() -> Self {
        PageRange(memory::address::PageRange::empty())
    }

    /// Returns a new [`PageRange`] with a base of `start` that contains `count` [`Page`]s.
    pub const fn new(start: Page, count: usize) -> Self {
        Self(memory::address::PageRange::new(start.0, count))
    }

    /// Creates a new [`PageRange`] between the two [`Page`]s.
    ///
    /// If `end` is less than `start`, the [`PageRange`] is empty.
    pub const fn from_inclusive(start: Page, end: Page) -> Self {
        Self(memory::address::PageRange::from_inclusive(start.0, end.0))
    }

    /// Creates a new [`PageRange`] between the two [`Page`]s.
    ///
    /// If `end` is less than `start`, the [`PageRange`] is empty.
    pub const fn from_exclusive(start: Page, end: Page) -> Self {
        Self(memory::address::PageRange::from_exclusive(start.0, end.0))
    }

    /// Returns the [`Page`] at the start of this [`PageRange`].
    pub const fn start(self) -> Page {
        Page(self.0.start())
    }

    /// Returns the [`VirtualAddress`] at the start of this [`PageRange`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0.start().start_address(page_frame_size()))
    }

    /// Returns the [`Page`] at the end of this [`PageRange`].
    ///
    /// This is an inclusive end.
    pub const fn end_inclusive(self) -> Page {
        Page(self.0.end_inclusive())
    }

    /// Returns the [`VirtualAddress`] at the end of this [`PageRange`].
    ///
    /// This is an inclusive end.
    pub fn end_address_inclusive(self) -> VirtualAddress {
        VirtualAddress(
            self.0
                .end_inclusive()
                .end_address_inclusive(page_frame_size()),
        )
    }

    /// Returns the [`Page`] at the end of this [`PageRange`].
    ///
    /// This is an exclusive end.
    pub const fn end_exclusive(self) -> Page {
        Page(self.0.end_exclusive())
    }

    /// Returns the [`VirtualAddress`] at the end of this [`PageRange`].
    ///
    /// This is an exclusive end.
    pub fn end_address_exclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_exclusive().start_address(page_frame_size()))
    }

    /// Returns the number of [`Page`]s in this [`PageRange`].
    pub const fn count(self) -> usize {
        self.0.count()
    }

    /// Returns the number of bytes in this [`PageRange`].
    pub fn byte_count(self) -> usize {
        self.0.byte_count(page_frame_size())
    }

    /// Returns `true` if the [`PageRange`] is empty.
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the provided [`Page`] is contained in this [`PageRange`].
    pub const fn contains(self, frame: Page) -> bool {
        self.0.contains(frame.0)
    }

    /// Splits this [`PageRange`] into two seperate [`PageRange`]s.
    ///
    /// - [start : start + index)
    /// - [start + index : end)
    ///
    /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the second
    /// [`PageRange`] will be empty.
    pub const fn split_at_index(self, index: usize) -> Option<(Self, Self)> {
        let Some((a, b)) = self.0.split_at_index(index) else {
            return None;
        };

        Some((PageRange(a), PageRange(b)))
    }

    /// Splits this [`PageRange`] into two seperate [`PageRange`]s.
    ///
    /// - [start : at)
    /// - [at : end)
    ///
    /// - If `at_frame` == `self.start()`, the first [`PageRange`] will be empty
    /// - If `at_frame` == `self.end()`, the second [`PageRange`] will be empty.
    ///
    /// Returns `None` if `at` is not adjacent or contained within this [`PageRange`]. Adjacent
    /// `at`s will produce empty [`PageRange`]s on either side.
    pub const fn split_at(self, at: Page) -> Option<(Self, Self)> {
        let Some((a, b)) = self.0.split_at(at.0) else {
            return None;
        };

        Some((PageRange(a), PageRange(b)))
    }

    /// Returns `true` if `self` and `other` share at least one [`Page`] in their [`PageRange`]s.
    pub const fn overlaps(self, other: Self) -> bool {
        self.0.overlaps(other.0)
    }

    /// Returns the merged [`PageRange`] if the two provided [`PageRange`]s are adjacent or
    /// overlapping.
    ///
    /// Otherwise, `None` is returned.
    pub const fn merge(self, other: Self) -> Option<Self> {
        let Some(value) = self.0.merge(other.0) else {
            return None;
        };

        Some(PageRange(value))
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`PageRange`]s do not overlap, then an empty [`PageRange`] is returned.
    pub const fn intersection(self, other: Self) -> Self {
        PageRange(self.0.intersection(other.0))
    }

    /// Partitions `self` into three disjoint [`PageRange`]s relative to `other`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`Page`]s in
    /// `self` according to their position relative to `other`:
    ///
    /// - `lower`   — [`Page`]s in `self` strictly below `other`
    /// - `overlap` — [`Page`]s in `self` that intersect `other`
    /// - `upper`   — [`Page`]s in `self` strictly above `other`
    pub const fn partition(self, other: Self) -> (Self, Self, Self) {
        let (lower, middle, upper) = self.0.partition(other.0);
        (PageRange(lower), PageRange(middle), PageRange(upper))
    }

    /// Returns an [`Iterator`] over all the frames in this [`PageRange`].
    pub fn iter(self) -> impl Iterator<Item = Page> {
        self.0.iter().map(Page)
    }
}

impl fmt::Debug for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..={:?}", self.start(), self.end_exclusive())
    }
}

impl fmt::Display for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..={}", self.start(), self.end_exclusive())
    }
}

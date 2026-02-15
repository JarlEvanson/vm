//! Structures related to physical and virtual memory and its management.

use core::fmt;

use crate::platform::generic::platform;

/// An address in the physical memory space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    /// Creates a new [`PhysicalAddress`] with a value of 0.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Creates a new [`PhysicalAddress`] with a value of `value`.
    pub const fn new(value: u64) -> PhysicalAddress {
        Self(value)
    }

    /// Returns the underlying `u64` value for this [`PhysicalAddress`].
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes higher.
    pub const fn add(self, count: u64) -> Self {
        Self::new(self.0.strict_add(count))
    }

    /// Creates a new [`PhysicalAddress`] that is `count` bytes lower.
    pub const fn sub(self, count: u64) -> Self {
        Self::new(self.0.strict_sub(count))
    }

    /// Returns the offset, in bytes, from the start of a [`Frame`].
    pub fn frame_offset(self) -> u64 {
        self.0 % platform().frame_size()
    }

    /// Returns `true` if the [`PhysicalAddress`] is a multiple of `alignment`.
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0.is_multiple_of(alignment)
    }

    /// Returns the greatest [`PhysicalAddress`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    pub const fn align_down(self, alignment: u64) -> Self {
        Self::new((self.0 / alignment) * alignment)
    }

    /// Returns the smallest [`PhysicalAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    #[expect(clippy::missing_panics_doc)]
    pub const fn align_up(self, alignment: u64) -> Self {
        Self::new(
            self.0
                .checked_next_multiple_of(alignment)
                .expect("failed to align PhysicalAddress up"),
        )
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysicalAddress({:#x})", self.0)
    }
}

impl fmt::Display for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// A chunk of physical memory aligned to a frame boundary.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(u64);

impl Frame {
    /// Creates a new [`Frame`] with a value of 0.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Creates a new [`Frame`] with a value of `value`.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the [`Frame`] in which `address` is contained.
    pub fn containing_address(address: PhysicalAddress) -> Self {
        Self(address.value() / platform().frame_size())
    }

    /// Returns the underlying `u64` value for this [`Frame`].
    ///
    /// This is a `frame_size()`-sized indexing of physical memory.
    pub const fn number(self) -> u64 {
        self.0
    }

    /// Returns the [`PhysicalAddress`] at the start of this [`Frame`].
    pub fn start_address(self) -> PhysicalAddress {
        PhysicalAddress::new(self.0 * platform().frame_size())
    }

    /// Returns the [`PhysicalAddress`] at the end of this [`Frame`].
    ///
    /// This is an exclusive end.
    pub fn end_address(self) -> PhysicalAddress {
        PhysicalAddress::new(self.0 * platform().frame_size() + platform().frame_size())
    }

    /// Creates a new [`Frame`] that is `count` bytes higher.
    pub const fn add(self, count: u64) -> Self {
        Self::new(self.0.strict_add(count))
    }

    /// Creates a new [`Frame`] that is `count` bytes lower.
    pub const fn sub(self, count: u64) -> Self {
        Self::new(self.0.strict_sub(count))
    }

    /// Returns a new [`Frame`] that is aligned up from this [`Frame`] to the nearest multiple of
    /// `alignment`. `alignment` is an alignment in bytes.
    ///
    /// If `alignment` is less than [`frame_size()`], then [`Frame`] remains the same.
    #[expect(clippy::missing_panics_doc, reason = "guard against stupidity")]
    pub fn align_up(self, alignment: u64) -> Self {
        let number_alignment = alignment.div_ceil(platform().frame_size());
        Self(
            self.number()
                .checked_next_multiple_of(number_alignment)
                .expect("aligned up too much"),
        )
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame({:#x})", self.0)
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame {:#x}", self.0)
    }
}

/// A range of contiguous [`Frame`]s in physical memory.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameRange {
    /// The [`Frame`] at the start of this [`FrameRange`].
    start: Frame,
    /// The number of [`Frame`]s in this [`FrameRange`].
    count: u64,
}

impl FrameRange {
    /// Creates an empty [`FrameRange`].
    pub const fn empty() -> Self {
        Self {
            start: Frame::zero(),
            count: 0,
        }
    }

    /// Returns a new [`FrameRange`] with a base of `start` that contains `count` [`Frame`]s.
    pub const fn new(start: Frame, count: u64) -> Self {
        Self { start, count }
    }

    /// Creates a new [`FrameRange`] between the two [`Frame`]s.
    ///
    /// If `end` is less than `start`, the [`FrameRange`] is empty.
    pub const fn from_inclusive(start: Frame, end: Frame) -> Self {
        let count = end.number().saturating_sub(start.number()).strict_add(1);
        Self { start, count }
    }

    /// Creates a new [`FrameRange`] between the two [`Frame`]s.
    ///
    /// If `end` is less than `start`, the [`FrameRange`] is empty.
    pub const fn from_exclusive(start: Frame, end: Frame) -> Self {
        let count = end.number().saturating_sub(start.number());
        Self { start, count }
    }

    /// Creates a new [`FrameRange`] that completely contains the range represented by `start` and
    /// `end` with an exclusive end.
    ///
    /// This means that `start` is aligned down to the nearest [`Frame`] while `end` is aligned up
    /// to the nearest `frame`.
    pub fn from_addresses(start: PhysicalAddress, end: PhysicalAddress) -> Self {
        let start_frame = Frame::containing_address(start);
        let end_frame = Frame::containing_address(end.align_up(platform().frame_size()));

        Self::from_exclusive(start_frame, end_frame)
    }

    /// Returns the [`Frame`] at the start of this [`FrameRange`].
    pub const fn start(self) -> Frame {
        self.start
    }

    /// Returns the [`Frame`] at the end of this [`FrameRange`].
    ///
    /// This is an exclusive end.
    pub const fn end(self) -> Frame {
        Frame::new(self.start.number().strict_add(self.count))
    }

    /// Returns the number of [`Frame`]s in this [`FrameRange`].
    pub const fn count(self) -> u64 {
        self.count
    }

    /// Returns `true` if the [`FrameRange`] is empty.
    pub const fn is_empty(self) -> bool {
        self.count() == 0
    }

    /// Returns the number of bytes contained in this [`FrameRange`].
    pub fn byte_count(self) -> u64 {
        self.count().strict_mul(platform().frame_size())
    }

    /// Returns `true` if the provided [`Frame`] is contained in this [`FrameRange`].
    pub const fn contains(self, frame: Frame) -> bool {
        self.start().number() <= frame.number() && frame.number() < self.end().number()
    }

    /// Splits this [`FrameRange`] into two seperate [`FrameRange`]s.
    ///
    /// - [start : at_frame - 1]
    /// - [at_frame : end]
    ///
    /// - If `at_frame` == `self.start()`, the first [`FrameRange`] will be empty
    /// - If `at_frame` == `self.end()`, the second [`FrameRange`] will be empty.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] containing `self` if `at_frame` is out of bounds or if `self` is empty.
    pub const fn split_at(self, at_frame: Frame) -> Result<(Self, Self), Self> {
        if at_frame.number() < self.start().number() || self.end().number() < at_frame.number() {
            return Err(self);
        }

        let upper = FrameRange::from_exclusive(at_frame, self.end());
        let lower = FrameRange {
            start: self.start(),
            count: self.count().strict_sub(upper.count()),
        };
        Ok((lower, upper))
    }

    /// Returns `true` if `self` and `other` overlap.
    ///
    /// Edges touching does not count.
    pub const fn overlaps(self, other: Self) -> bool {
        self.start().number() < other.end().number() && other.start().number() < self.end().number()
    }

    /// Returns the merged [`FrameRange`], where merging only occurs if the ranges are overlapping
    /// or contiguous.
    pub const fn merge(self, other: Self) -> Option<Self> {
        if !(self.overlaps(other)
            || self.start().number() == other.end().number()
            || other.start().number() == self.end().number())
        {
            return None;
        }

        let start = if self.start().number() <= other.start().number() {
            self.start()
        } else {
            other.start()
        };

        let end = if self.end().number() >= other.end().number() {
            self.end()
        } else {
            other.end()
        };

        let count = end.number().strict_sub(start.number());
        Some(Self { start, count })
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`FrameRange`]s do not overlap, then an empty [`FrameRange`] is returned.
    pub const fn intersection(self, other: Self) -> Self {
        let start = if self.start().number() >= other.start().number() {
            self.start()
        } else {
            other.start()
        };
        let end = if self.end().number() <= other.end().number() {
            self.end()
        } else {
            other.end()
        };

        Self::from_exclusive(start, end)
    }

    /// Partitions `other` into three disjoint [`FrameRange`]s relative to `self`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`Frame`]s in
    /// `other` according to their position relative to `self`:
    ///
    /// - `lower`   — [`Frame`]s in `other` strictly below `self`
    /// - `overlap` — [`Frame`]s in `other` that intersect `self`
    /// - `upper`   — [`Frame`]s in `other` strictly above `self`
    pub const fn partition_with(self, other: FrameRange) -> (FrameRange, FrameRange, FrameRange) {
        let lower_end = if other.end().number() <= self.start().number() {
            other.end()
        } else {
            self.start()
        };

        let upper_start = if other.start().number() >= self.end().number() {
            other.start()
        } else {
            self.end()
        };

        let lower = FrameRange::from_exclusive(other.start(), lower_end);
        let overlap = self.intersection(other);
        let upper = FrameRange::from_exclusive(upper_start, other.end());
        (lower, overlap, upper)
    }

    /// Returns an [`Iterator`] over all the frames in this [`FrameRange`].
    pub fn iter(self) -> impl Iterator<Item = Frame> {
        (self.start().number()..self.end().number()).map(Frame::new)
    }
}

impl fmt::Debug for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FrameRange({:?}..{:?})", self.start(), self.end())
    }
}

impl fmt::Display for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start(), self.end())
    }
}

/// An address in the virtual memory space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(usize);

impl VirtualAddress {
    /// Creates a new [`VirtualAddress`] with a value of 0.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Creates a new [`VirtualAddress`] with a value of `value`.
    pub const fn new(value: usize) -> VirtualAddress {
        Self(value)
    }

    /// Returns the underlying `usize` value for this [`VirtualAddress`].
    pub const fn value(self) -> usize {
        self.0
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes higher.
    pub const fn add(self, count: usize) -> Self {
        Self::new(self.0.strict_add(count))
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes lower.
    pub const fn sub(self, count: usize) -> Self {
        Self::new(self.0.strict_sub(count))
    }

    /// Returns the offset, in bytes, from the start of a [`Page`].
    pub fn page_offset(self) -> usize {
        self.0 % platform().page_size()
    }

    /// Returns `true` if the [`VirtualAddress`] is a multiple of `alignment`.
    pub const fn is_aligned(self, alignment: usize) -> bool {
        self.0.is_multiple_of(alignment)
    }

    /// Returns the greatest [`VirtualAddress`] that is less than or equal to `self` and is a
    /// multiple of `alignment`.
    pub const fn align_down(self, alignment: usize) -> Self {
        Self::new((self.0 / alignment) * alignment)
    }

    /// Returns the smallest [`VirtualAddress`] that is greater than or equal to `self` and is a
    /// multiple of `alignment`.
    #[expect(clippy::missing_panics_doc)]
    pub const fn align_up(self, alignment: usize) -> Self {
        Self::new(
            self.0
                .checked_next_multiple_of(alignment)
                .expect("failed to align VirtualAddress up"),
        )
    }
}

impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtualAddress({:#x})", self.0)
    }
}

impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// A chunk of virtual memory aligned to a page boundary.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(usize);

impl Page {
    /// Creates a new [`Page`] with a value of 0.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Creates a new [`Page`] with a value of `value`.
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Returns the [`Page`] in which `address` is contained.
    pub fn containing_address(address: VirtualAddress) -> Self {
        Self(address.value() / platform().page_size())
    }

    /// Returns the underlying `usize` value for this [`Page`].
    ///
    /// This is a `page_size()`-sized indexing of virtual memory.
    pub const fn number(self) -> usize {
        self.0
    }

    /// Returns the [`VirtualAddress`] at the start of this [`Page`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress::new(self.0 * platform().page_size())
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    ///
    /// This is an exclusive end.
    pub fn end_address(self) -> VirtualAddress {
        VirtualAddress::new(self.0 * platform().page_size() + platform().page_size())
    }

    /// Creates a new [`Page`] that is `count` bytes higher.
    pub const fn add(self, count: usize) -> Self {
        Self::new(self.0.strict_add(count))
    }

    /// Creates a new [`Page`] that is `count` bytes lower.
    pub const fn sub(self, count: usize) -> Self {
        Self::new(self.0.strict_sub(count))
    }

    /// Returns a new [`Page`] that is aligned up from this [`Page`] to the nearest multiple of
    /// `alignment`. `alignment` is an alignment in bytes.
    ///
    /// If `alignment` is less than [`page_size()`], then [`Page`] remains the same.
    #[expect(clippy::missing_panics_doc, reason = "guard against stupidity")]
    pub fn align_up(self, alignment: usize) -> Self {
        let number_alignment = alignment.div_ceil(platform().page_size());
        Self(
            self.number()
                .checked_next_multiple_of(number_alignment)
                .expect("aligned up too much"),
        )
    }
}

impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page({:#x})", self.0)
    }
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Page {:#x}", self.0)
    }
}

/// A range of contiguous [`Page`]s in virtual memory.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageRange {
    /// The [`Page`] at the start of this [`PageRange`].
    start: Page,
    /// The number of [`Page`]s in this [`PageRange`].
    count: usize,
}

impl PageRange {
    /// Creates an empty [`PageRange`].
    pub const fn empty() -> Self {
        Self {
            start: Page::zero(),
            count: 0,
        }
    }

    /// Returns a new [`PageRange`] with a base of `start` that contains `count` [`Page`]s.
    pub const fn new(start: Page, count: usize) -> Self {
        Self { start, count }
    }

    /// Creates a new [`PageRange`] between the two [`Page`]s.
    ///
    /// If `end` is less than `start`, the [`PageRange`] is empty.
    pub const fn from_inclusive(start: Page, end: Page) -> Self {
        let count = end.number().saturating_sub(start.number()).strict_add(1);
        Self { start, count }
    }

    /// Creates a new [`PageRange`] between the two [`Page`]s.
    ///
    /// If `end` is less than `start`, the [`PageRange`] is empty.
    pub const fn from_exclusive(start: Page, end: Page) -> Self {
        let count = end.number().saturating_sub(start.number());
        Self { start, count }
    }

    /// Creates a new [`PageRange`] that completely contains the range represented by `start` and
    /// `end` with an exclusive end.
    ///
    /// This means that `start` is aligned down to the nearest [`Page`] while `end` is aligned up
    /// to the nearest `page`.
    pub fn from_addresses(start: VirtualAddress, end: VirtualAddress) -> Self {
        let start_page = Page::containing_address(start);
        let end_page = Page::containing_address(end.align_up(platform().page_size()));

        Self::from_exclusive(start_page, end_page)
    }

    /// Returns the [`Page`] at the start of this [`PageRange`].
    pub const fn start(self) -> Page {
        self.start
    }

    /// Returns the [`Page`] at the end of this [`PageRange`].
    ///
    /// This is an exclusive end.
    pub const fn end(self) -> Page {
        Page::new(self.start.number().strict_add(self.count))
    }

    /// Returns the number of [`Page`]s in this [`PageRange`].
    pub const fn count(self) -> usize {
        self.count
    }

    /// Returns `true` if the [`PageRange`] is empty.
    pub const fn is_empty(self) -> bool {
        self.count() == 0
    }

    /// Returns the number of bytes contained in this [`PageRange`].
    pub fn byte_count(self) -> usize {
        self.count().strict_mul(platform().page_size())
    }

    /// Returns `true` if the provided [`Page`] is contained in this [`PageRange`].
    pub const fn contains(self, page: Page) -> bool {
        self.start().number() <= page.number() && page.number() < self.end().number()
    }

    /// Splits this [`PageRange`] into two seperate [`PageRange`]s.
    ///
    /// - [start : at_page - 1]
    /// - [at_page : end]
    ///
    /// - If `at_page` == `self.start()`, the first [`PageRange`] will be empty
    /// - If `at_page` == `self.end()`, the second [`PageRange`] will be empty.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] containing `self` if `at_page` is out of bounds or if `self` is empty.
    pub const fn split_at(self, at_page: Page) -> Result<(Self, Self), Self> {
        if at_page.number() < self.start().number() || self.end().number() < at_page.number() {
            return Err(self);
        }

        let upper = PageRange::from_exclusive(at_page, self.end());
        let lower = PageRange {
            start: self.start(),
            count: self.count().strict_sub(upper.count()),
        };
        Ok((lower, upper))
    }

    /// Returns `true` if `self` and `other` overlap.
    ///
    /// Edges touching does not count.
    pub const fn overlaps(self, other: Self) -> bool {
        self.start().number() < other.end().number() && other.start().number() < self.end().number()
    }

    /// Returns the merged [`PageRange`], where merging only occurs if the ranges are overlapping
    /// or contiguous.
    pub const fn merge(self, other: Self) -> Option<Self> {
        if !(self.overlaps(other)
            || self.start().number() == other.end().number()
            || other.start().number() == self.end().number())
        {
            return None;
        }

        let start = if self.start().number() <= other.start().number() {
            self.start()
        } else {
            other.start()
        };

        let end = if self.end().number() >= other.end().number() {
            self.end()
        } else {
            other.end()
        };

        let count = end.number().strict_sub(start.number());
        Some(Self { start, count })
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`PageRange`]s do not overlap, then an empty [`PageRange`] is returned.
    pub const fn intersection(self, other: Self) -> Self {
        let start = if self.start().number() >= other.start().number() {
            self.start()
        } else {
            other.start()
        };
        let end = if self.end().number() <= other.end().number() {
            self.end()
        } else {
            other.end()
        };

        Self::from_exclusive(start, end)
    }

    /// Partitions `other` into three disjoint [`PageRange`]s relative to `self`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`Page`]s in
    /// `other` according to their position relative to `self`:
    ///
    /// - `lower`   — [`Page`]s in `other` strictly below `self`
    /// - `overlap` — [`Page`]s in `other` that intersect `self`
    /// - `upper`   — [`Page`]s in `other` strictly above `self`
    pub const fn partition_with(self, other: PageRange) -> (PageRange, PageRange, PageRange) {
        let lower_end = if other.end().number() <= self.start().number() {
            other.end()
        } else {
            self.start()
        };

        let upper_start = if other.start().number() >= self.end().number() {
            other.start()
        } else {
            self.end()
        };

        let lower = PageRange::from_exclusive(other.start(), lower_end);
        let overlap = self.intersection(other);
        let upper = PageRange::from_exclusive(upper_start, other.end());
        (lower, overlap, upper)
    }

    /// Returns an [`Iterator`] over all the pages in this [`PageRange`].
    pub fn iter(self) -> impl Iterator<Item = Page> {
        (self.start().number()..self.end().number()).map(Page::new)
    }
}

impl fmt::Debug for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageRange({:?}..{:?})", self.start(), self.end())
    }
}

impl fmt::Display for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start(), self.end())
    }
}

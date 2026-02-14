//! Structures related to virtual memory and its management.

use crate::memory::page_frame_size;

/// An address in the virtual memory space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
        self.0 % page_frame_size()
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

/// A chunk of virtual memory aligned to a page boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
        Self(address.value() / page_frame_size())
    }

    /// Returns the underlying `usize` value for this [`Page`].
    ///
    /// This is a `page_size()`-sized indexing of virtual memory.
    pub const fn number(self) -> usize {
        self.0
    }

    /// Returns the [`VirtualAddress`] at the start of this [`Page`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress::new(self.0 * page_frame_size())
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    ///
    /// This is an exclusive end.
    pub fn end_address(self) -> VirtualAddress {
        VirtualAddress::new(self.0 * page_frame_size() + page_frame_size())
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
    /// If `alignment` is less than [`page_frame_size()`], then [`Page`] remains the same.
    #[expect(clippy::missing_panics_doc, reason = "guard against stupidity")]
    pub fn align_up(self, alignment: usize) -> Self {
        let number_alignment = alignment.div_ceil(page_frame_size());
        Self(
            self.number()
                .checked_next_multiple_of(number_alignment)
                .expect("aligned up too much"),
        )
    }
}

/// A range of contiguous [`Page`]s in virtual memory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
        let end_page = Page::containing_address(end.align_up(page_frame_size()));

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
        self.count().strict_mul(page_frame_size())
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

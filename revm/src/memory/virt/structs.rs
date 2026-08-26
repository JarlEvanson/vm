use core::fmt;

use crate::memory::page_frame_size;

/// An address in the vitual memory space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(AddressUsize);

impl VirtualAddress {
    /// Creates a new [`VirtualAddress`] with a value of 0.
    pub const fn zero() -> Self {
        Self(AddressUsize::zero())
    }

    /// Creates a new [`VirtualAddress`] with a value of `value`.
    pub const fn new(value: usize) -> VirtualAddress {
        Self(AddressUsize::new(value))
    }

    /// Returns the underlying `usize` value for this [`VirtualAddress`].
    pub const fn value(self) -> usize {
        self.0.value()
    }

    /// Creates a new [`VirtualAddress`] that is `count` bytes higher.
    ///
    /// Returns [`None`] if the operation would overflow.
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
    /// Returns [`None`] if the operation would underflow.
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
    /// Returns [`None`] if the operation would overflow.
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

/// A range of contiguous [`VirtualAddress`]es.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddressRange(AddressUsizeRange);

impl VirtualAddressRange {
    /// Creates a new [`VirtualAddressRange`] of the form `start..=end`.
    pub const fn new(start: VirtualAddress, end: VirtualAddress) -> Self {
        Self(AddressUsizeRange::new(start.0, end.0))
    }

    /// Returns the [`VirtualAddress`] at the start of this [`VirtualAddressRange`].
    pub const fn start(self) -> VirtualAddress {
        VirtualAddress(self.0.start())
    }

    /// Returns the number of bytes in the [`VirtualAddressRange`].
    pub const fn count(self) -> usize {
        self.0.count()
    }

    /// Returns the [`VirtualAddress`] at the inclusive end of this [`VirtualAddressRange`].
    ///
    /// The result of this function is the same when called with a [`VirtualAddressRange`]
    /// of 0 bytes and with a [`VirtualAddressRange`] of 1 byte.
    pub const fn end_inclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_inclusive())
    }

    /// Returns the [`VirtualAddress`] at the exclusive end of this [`VirtualAddressRange`].
    pub const fn end_exclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_exclusive())
    }

    /// Returns `true` if the provided [`VirtualAddress`] is contained within this
    /// [`VirtualAddressRange`].
    pub const fn contains(self, address: VirtualAddress) -> bool {
        self.0.contains(address.0)
    }

    /// Returns `true` if `self` and `other` share at least one byte in their
    /// [`VirtualAddressRange`]es.
    pub const fn overlaps(self, other: Self) -> bool {
        self.0.overlaps(other.0)
    }

    /// Returns the merged [`VirtualAddressRange`] if the two provided [`VirtualAddressRange`]s
    /// are adjacent or overlapping.
    ///
    /// Otherwise, [`None`] will be returned.
    pub const fn merge(self, other: Self) -> Option<Self> {
        let Some(range) = self.0.merge(other.0) else {
            return None;
        };

        Some(Self(range))
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`VirtualAddressRange`]s do not overlap, then [`None`] will be returned.
    pub const fn intersection(self, other: Self) -> Option<Self> {
        if let Some(range) = self.0.intersection(other.0) {
            Some(Self(range))
        } else {
            None
        }
    }

    /// Partitions `self` into three disjoint [`VirtualAddressRange`]s relative to `other`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`VirtualAddress`]es in
    /// `self` according to their position relative to `other`:
    ///
    /// - `lower`   — [`VirtualAddress`]es in `self` strictly below `other`
    /// - `overlap` — [`VirtualAddress`]es in `self` that are contained inside `other`
    /// - `upper`   — [`VirtualAddress`]es in `self` strictly above `other`
    pub const fn partition(self, other: Self) -> (Option<Self>, Option<Self>, Option<Self>) {
        let (lower, overlap, upper) = self.0.partition(other.0);

        let lower = if let Some(range) = lower {
            Some(Self(range))
        } else {
            None
        };

        let overlap = if let Some(range) = overlap {
            Some(Self(range))
        } else {
            None
        };

        let upper = if let Some(range) = upper {
            Some(Self(range))
        } else {
            None
        };

        (lower, overlap, upper)
    }

    /// Returns an [`Iterator`] over all [`VirtualAddress`]es in this [`VirtualAddressRange`].
    pub fn iter(self) -> impl Iterator<Item = VirtualAddress> {
        self.0.iter().map(VirtualAddress)
    }
}

impl fmt::Debug for VirtualAddressRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VirtualAddressRange({:#0x}..={:#0x})",
            self.start().value(),
            self.end_inclusive().value()
        )
    }
}

impl fmt::Display for VirtualAddressRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:#0x}..={:#0x}",
            self.start().value(),
            self.end_inclusive().value()
        )
    }
}

/// A [`page_frame_size()`] sized and aligned contiguous range of virtual memory.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(AddressUsizeChunk);

impl Page {
    /// Creates a new [`Page`] with a value of 0.
    pub const fn zero() -> Self {
        Self(AddressUsizeChunk::zero())
    }

    /// Creates a new [`Page`] with a value of `value`.
    pub const fn new(value: usize) -> Self {
        Self(AddressUsizeChunk::new(value))
    }

    /// Returns the [`Page`] in which `address` is contained.
    pub fn containing_address(address: VirtualAddress) -> Self {
        Self(AddressUsizeChunk::containing_address(
            address.0,
            page_frame_size(),
        ))
    }

    /// Returns the underlying `usize` value for this [`Page`].
    ///
    /// This is a [`page_frame_size()`]-sized indexing of virtual memory.
    pub const fn number(self) -> usize {
        self.0.number()
    }

    /// Returns the [`VirtualAddress`] at the start of this [`Page`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0.start_address(page_frame_size()))
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    pub fn end_address_inclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_inclusive(page_frame_size()))
    }

    /// Returns the [`VirtualAddress`] at the end of this [`Page`].
    pub fn end_address_exclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_exclusive(page_frame_size()))
    }

    /// Returns the [`VirtualAddressRange`] that this [`Page`] represents.
    pub fn address_range(self) -> VirtualAddressRange {
        VirtualAddressRange::new(self.start_address(), self.end_address_inclusive())
    }

    /// Creates a new [`Page`] that is `count` [`Page`]s higher.
    ///
    /// Returns [`None`] if the operation would overflow.
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
    /// Returns [`None`] if the operation would underflow.
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
pub struct PageRange(AddressUsizeChunkRange);

impl PageRange {
    /// Creates a new [`PageRange`] of the form `start..=end`.
    pub const fn new(start: Page, end: Page) -> Self {
        Self(AddressUsizeChunkRange::new(start.0, end.0))
    }

    /// Returns the [`Page`] at the start of this [`PageRange`].
    pub const fn start(self) -> Page {
        Page(self.0.start())
    }

    /// Returns the [`VirtualAddress`] at the start of this [`PageRange`].
    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0.start_address(page_frame_size()))
    }

    /// Returns the number of [`Page`]s in this [`PageRange`].
    pub const fn count(self) -> usize {
        self.0.count()
    }

    /// Returns the number of bytes in this [`PageRange`].
    pub fn byte_count(self) -> usize {
        self.0.byte_count(page_frame_size())
    }

    /// Returns the [`Page`] at the end of this [`PageRange`].
    pub const fn end_inclusive(self) -> Page {
        Page(self.0.end_inclusive())
    }

    /// Returns the [`Page`] at the end of this [`PageRange`].
    pub const fn end_exclusive(self) -> Page {
        Page(self.0.end_exclusive())
    }

    /// Returns the [`VirtualAddress`] at the end of this [`PageRange`].
    pub fn end_address_inclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_inclusive(page_frame_size()))
    }

    /// Returns the [`VirtualAddress`] at the end of this [`PageRange`].
    pub fn end_address_exclusive(self) -> VirtualAddress {
        VirtualAddress(self.0.end_address_exclusive(page_frame_size()))
    }

    /// Returns the [`VirtualAddressRange`] that this [`PageRange`] represents.
    pub fn address_range(self) -> VirtualAddressRange {
        VirtualAddressRange(self.0.address_range(page_frame_size()))
    }

    /// Returns `true` if the provided [`Page`] is contained in this [`PageRange`].
    pub const fn contains(self, page: Page) -> bool {
        self.0.contains(page.0)
    }

    /// Returns `true` if `self` and `other` share at least one [`Page`] in their [`PageRange`]s.
    pub const fn overlaps(self, other: Self) -> bool {
        self.0.overlaps(other.0)
    }

    /// Returns the merged [`PageRange`] if the two provided [`PageRange`]s are adjacent or
    /// overlapping.
    ///
    /// Otherwise, [`None`] will be returned.
    pub const fn merge(self, other: Self) -> Option<Self> {
        let Some(value) = self.0.merge(other.0) else {
            return None;
        };

        Some(PageRange(value))
    }

    /// Returns the intersection of `self` and `other`.
    ///
    /// If the two [`PageRange`]s do not overlap, then [`None`] will be returned.
    pub const fn intersection(self, other: Self) -> Option<Self> {
        if let Some(range) = self.0.intersection(other.0) {
            Some(Self(range))
        } else {
            None
        }
    }

    /// Partitions `self` into three disjoint [`PageRange`]s relative to `other`.
    ///
    /// The returned tuple `(lower, overlap, upper)` classifies the [`Page`]s in
    /// `self` according to their position relative to `other`:
    ///
    /// - `lower`   — [`Page`]s in `self` strictly below `other`
    /// - `overlap` — [`Page`]s in `self` that are contained inside `other`
    /// - `upper`   — [`Page`]s in `self` strictly above `other`
    pub const fn partition(self, other: Self) -> (Option<Self>, Option<Self>, Option<Self>) {
        let (lower, overlap, upper) = self.0.partition(other.0);

        let lower = if let Some(range) = lower {
            Some(Self(range))
        } else {
            None
        };

        let overlap = if let Some(range) = overlap {
            Some(Self(range))
        } else {
            None
        };

        let upper = if let Some(range) = upper {
            Some(Self(range))
        } else {
            None
        };

        (lower, overlap, upper)
    }

    /// Returns an [`Iterator`] over all the pages in this [`PageRange`].
    pub fn iter(self) -> impl Iterator<Item = Page> {
        self.0.iter().map(Page)
    }
}

impl fmt::Debug for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..={:?}", self.start(), self.end_inclusive())
    }
}

impl fmt::Display for PageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..={}", self.start(), self.end_inclusive())
    }
}

memory::implement_address!(AddressUsize, "A native-sized address.", usize);
memory::implement_address_range!(
    AddressUsize,
    AddressUsizeRange,
    "A contiguous range of native-sized addresses.",
    inclusive,
    usize,
    contains_base_count_usize,
    overlaps_base_count_usize,
    merge_base_count_usize,
    intersection_base_count_usize,
    partition_base_count_usize
);
memory::implement_address_chunk!(
    AddressUsize,
    AddressUsizeChunk,
    "A `chunk-size`d contiguous range of native-sized addresses with `chunk-size` alignment.",
    usize
);
memory::implement_address_chunk_range!(
    AddressUsize,
    AddressUsizeRange,
    AddressUsizeChunk,
    AddressUsizeChunkRange,
    "A contiguous range of native-sized address chunks.",
    inclusive,
    usize,
    contains_base_count_usize,
    overlaps_base_count_usize,
    merge_base_count_usize,
    intersection_base_count_usize,
    partition_base_count_usize
);

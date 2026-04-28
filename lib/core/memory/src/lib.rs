//! Abstractions over physical and virtual memory.
#![no_std]

pub mod range;

/// A description of the parameters of an address space.
///
/// This can be utilized to describe both physical and virtual address spaces.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AddressSpaceDescriptor {
    /// The inclusive valid ranges that make up the address space.
    ///
    /// If `start > end`, then the range is empty.
    ranges: [(u64, u64); 2],
}

impl AddressSpaceDescriptor {
    /// Constructs a new [`AddressSpaceDescriptor`] with the specified parameters.
    ///
    /// # Panics
    ///
    /// Panics if the provided `implemented_bits` is greater than 64 bits, as that is nonsensical.
    #[must_use]
    pub const fn new(implemented_bits: u8, sign_extend_canonical: bool) -> Self {
        assert!(implemented_bits <= 64);

        if implemented_bits == 64 {
            return Self {
                ranges: [(0, u64::MAX), (1, 0)],
            };
        } else if implemented_bits == 0 {
            return Self {
                ranges: [(1, 0), (1, 0)],
            };
        }

        if sign_extend_canonical {
            let sign_bit = 1u64 << (implemented_bits - 1);

            let lower = (0, sign_bit - 1);
            let upper = ((!0u64) << (implemented_bits - 1), u64::MAX);

            Self {
                ranges: [lower, upper],
            }
        } else {
            let lower = (0, (1u64 << implemented_bits) - 1);
            let upper = (1, 0);

            Self {
                ranges: [lower, upper],
            }
        }
    }

    /// Constructs a new [`AddressSpaceDescriptor`] from two ranges of a given bit size.
    ///
    /// # Panics
    ///
    /// Panics if the provided `lower_bits` or `upper_bits` is greater than 64 bits, as that is
    /// nonsensical.
    #[must_use]
    pub const fn bit_range(lower_bits: u8, upper_bits: u8) -> Self {
        assert!(lower_bits <= 64);
        assert!(upper_bits <= 64);

        let mut lower = if lower_bits == 0 {
            (1, 0)
        } else if lower_bits == 64 {
            (0, u64::MAX)
        } else {
            (0, (1u64 << lower_bits) - 1)
        };

        let mut upper = if upper_bits == 0 {
            (1, 0)
        } else if upper_bits == 64 {
            (0, u64::MAX)
        } else {
            ((!0u64) << upper_bits, u64::MAX)
        };

        if lower.0 > lower.1 {
            lower = upper;
            upper = (1, 0);
        }

        Self {
            ranges: [lower, upper],
        }
    }

    /// Returns `true` if the provided address is a valid address in the address space described by
    /// [`AddressSpaceDescriptor`].
    #[inline]
    #[must_use]
    pub const fn is_valid(self, address: u64) -> bool {
        let (s0, e0) = self.ranges[0];
        let (s1, e1) = self.ranges[1];

        (address >= s0 && address <= e0) || (address >= s1 && address <= e1)
    }

    /// Returns `true` if the provided inclusive range `[start, end]` is entirely valid within the
    /// address space described by [`AddressSpaceDescriptor`].
    #[inline]
    #[must_use]
    pub const fn is_valid_range(self, start: u64, end: u64) -> bool {
        // Reject wrapping ranges.
        if start > end {
            return false;
        }

        let (s0, e0) = self.ranges[0];
        let (s1, e1) = self.ranges[1];

        (start >= s0 && end <= e0) || (start >= s1 && end <= e1)
    }

    /// Returns the valid ranges for the address space described by [`AddressSpaceDescriptor`].
    ///
    /// If the valid ranges for the [`AddressSpaceDescriptor`] can be described by a single range,
    /// then the second range will be empty.
    #[inline]
    #[must_use]
    pub const fn valid_ranges(self) -> [(u64, u64); 2] {
        self.ranges
    }
}

/// Constructs an address primitive.
#[macro_export]
macro_rules! implement_address {
    ($address_name:ident,
     $address_doc:expr,
     $impl_type:ident
    ) => {
        #[doc = $address_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_name($impl_type);

        impl $address_name {
            /// Creates a new address with a value of 0.
            #[inline]
            #[must_use]
            pub const fn zero() -> Self {
                Self(0)
            }

            /// Creates a new address with a value of `value`.
            #[inline]
            #[must_use]
            pub const fn new(value: $impl_type) -> Self {
                Self(value)
            }

            /// Returns the underlying value for this address.
            #[inline]
            #[must_use]
            pub const fn value(self) -> $impl_type {
                self.0
            }

            /// Creates a new address that is `count` bytes higher.
            ///
            /// Returns [`None`] if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn checked_add(self, count: $impl_type) -> Option<Self> {
                let Some(new_address) = self.0.checked_add(count) else {
                    return None;
                };

                Some(Self::new(new_address))
            }

            /// Creates a new address that is `count` bytes higher.
            ///
            /// Panics if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn strict_add(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_add(count))
            }

            /// Creates a new address that is `count` bytes lower.
            ///
            /// Returns [`None`] if the operation would underflow.
            #[inline]
            #[must_use]
            pub const fn checked_sub(self, count: $impl_type) -> Option<Self> {
                let Some(new_address) = self.0.checked_sub(count) else {
                    return None;
                };

                Some(Self::new(new_address))
            }

            /// Creates a new address that is `count` bytes lower.
            ///
            /// Panics if the operation would underflow.
            #[inline]
            #[must_use]
            pub const fn strict_sub(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_sub(count))
            }

            /// Returns `true` if the address is a multiple of `alignment`.
            #[inline]
            #[must_use]
            pub const fn is_aligned(self, alignment: $impl_type) -> bool {
                debug_assert!(alignment.is_power_of_two());

                self.0.is_multiple_of(alignment)
            }

            /// Returns the greatest address that is less than or equal to `self` and is a
            /// multiple of `alignment`.
            #[inline]
            #[must_use]
            pub const fn align_down(self, alignment: $impl_type) -> Self {
                debug_assert!(alignment.is_power_of_two());

                Self::new((self.0 / alignment) * alignment)
            }

            /// Returns the smallest address that is greater than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// Returns [`None`] if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn checked_align_up(self, alignment: $impl_type) -> Option<Self> {
                debug_assert!(alignment.is_power_of_two());

                let Some(new_address) = self.0.checked_next_multiple_of(alignment) else {
                    return None;
                };

                Some(Self::new(new_address))
            }

            /// Returns the smallest address that is greater than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// Panics if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn strict_align_up(self, alignment: $impl_type) -> Self {
                debug_assert!(alignment.is_power_of_two());

                Self::new(
                    self.0
                        .checked_next_multiple_of(alignment)
                        .expect("failed to align Address up"),
                )
            }
        }
    };
}

/// Constructs an address range primitive.
#[macro_export]
macro_rules! implement_address_range {
    ($address_name:ident,
     $address_range_name:ident,
     $address_range_doc:expr,
     inclusive,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $address_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_range_name {
            /// The start of the range.
            start: $address_name,
            /// The inclusive end of the range.
            end: $address_name,
        }

        impl $address_range_name {
            /// Creates a new inclusive address range of the form `start..=end`.
            #[inline]
            #[must_use]
            pub const fn new(start: $address_name, end: $address_name) -> Self {
                assert!(start.value() <= end.value());

                Self { start, end }
            }

            /// Returns the address at the start of this range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $address_name {
                self.start
            }

            /// Returns the number of bytes in the address range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                let Some(difference) = self.end.value().checked_sub(self.start.value()) else {
                    return 0;
                };

                difference.strict_add(1)
            }

            /// Returns the address at the inclusive end of this range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 bytes and a range of 1 byte.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $address_name {
                self.end
            }

            /// Returns the address at the exclusive end of this range.
            pub const fn end_exclusive(self) -> $address_name {
                self.end.strict_add(1)
            }

            /// Returns `true` if the provided address is contained within this address range.
            #[inline]
            #[must_use]
            pub const fn contains(self, address: $address_name) -> bool {
                $crate::range::$contains(
                    self.start().value(),
                    self.end_inclusive().value(),
                    address.value(),
                )
            }

            /// Returns `true` if `self` and `other` share at least one address in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().value(),
                    self.end_inclusive().value(),
                    other.start().value(),
                    other.end_inclusive().value(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, end_inclusive)) = $crate::range::$merge(
                    self.start().value(),
                    self.end_inclusive().value(),
                    other.start().value(),
                    other.end_inclusive().value(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), $address_name::new(end_inclusive));
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, end_inclusive)) = $crate::range::$intersection(
                    self.start().value(),
                    self.end_inclusive().value(),
                    other.start().value(),
                    other.end_inclusive().value(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), $address_name::new(end_inclusive));
                Some(range)
            }

            /// Partitions `self` into three disjoint address ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the addresses in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — addresses in `self` strictly below `other`
            /// - `overlap` — addresses in `self` that are contained inside `other`
            /// - `upper`   — addresses in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().value(),
                    self.end_inclusive().value(),
                    other.start().value(),
                    other.end_inclusive().value(),
                );

                let lower = if let Some((start, end_inclusive)) = result.0 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, end_inclusive)) = result.1 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, end_inclusive)) = result.2 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the addresses in this address range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $address_name> {
                (self.start().value()..=self.end_inclusive().value()).map($address_name::new)
            }
        }
    };
    ($address_name:ident,
     $address_range_name:ident,
     $address_range_doc:expr,
     exclusive,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $address_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_range_name {
            /// The start of the range.
            start: $address_name,
            /// The exclusive end of the range.
            end: $address_name,
        }

        impl $address_range_name {
            /// Creates an empty address range.
            #[inline]
            #[must_use]
            pub const fn empty() -> Self {
                Self {
                    start: $address_name::zero(),
                    end: $address_name::zero(),
                }
            }

            /// Creates a new inclusive address range of the form `start..end`.
            #[inline]
            #[must_use]
            pub const fn new(start: $address_name, end: $address_name) -> Self {
                assert!(start.value() <= end.value());

                Self { start, end }
            }

            /// Returns the address at the start of this range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $address_name {
                self.start
            }

            /// Returns the number of bytes in the address range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                let Some(difference) = self.end.value().checked_sub(self.start.value()) else {
                    return 0;
                };

                difference
            }

            /// Returns the address at the inclusive end of this range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 bytes and a range of 1 byte.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $address_name {
                if !self.is_empty() {
                    self.end.strict_sub(1)
                } else {
                    self.start
                }
            }

            /// Returns the address at the exclusive end of this range.
            #[inline]
            #[must_use]
            pub const fn end_exclusive(self) -> $address_name {
                self.end
            }

            /// Returns `true` if the address range is empty.
            #[inline]
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.start.value() > self.end.value()
            }

            /// Returns `true` if the provided address is contained within this address range.
            #[inline]
            #[must_use]
            pub const fn contains(self, address: $address_name) -> bool {
                $crate::range::$contains(
                    self.start().value(),
                    self.end_exclusive().value(),
                    address.value(),
                )
            }

            /// Returns `true` if `self` and `other` share at least one address in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().value(),
                    self.end_exclusive().value(),
                    other.start().value(),
                    other.end_exclusive().value(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, end_exclusive)) = $crate::range::$merge(
                    self.start().value(),
                    self.end_exclusive().value(),
                    other.start().value(),
                    other.end_exclusive().value(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), $address_name::new(end_exclusive));
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, end_exclusive)) = $crate::range::$intersection(
                    self.start().value(),
                    self.end_exclusive().value(),
                    other.start().value(),
                    other.end_exclusive().value(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), $address_name::new(end_exclusive));
                Some(range)
            }

            /// Partitions `self` into three disjoint address ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the addresses in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — addresses in `self` strictly below `other`
            /// - `overlap` — addresses in `self` that are contained inside `other`
            /// - `upper`   — addresses in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().value(),
                    self.end_exclusive().value(),
                    other.start().value(),
                    other.end_exclusive().value(),
                );

                let lower = if let Some((start, end_exclusive)) = result.0 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, end_exclusive)) = result.1 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, end_exclusive)) = result.2 {
                    let range =
                        Self::new($address_name::new(start), $address_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the addresses in this address range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $address_name> {
                (self.start().value()..self.end_exclusive().value()).map($address_name::new)
            }
        }
    };
    ($address_name:ident,
     $address_range_name:ident,
     $address_range_doc:expr,
     base_count,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $address_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_range_name {
            /// The start of the range.
            start: $address_name,
            /// The number of addresses contained in the range.
            count: $impl_type,
        }

        impl $address_range_name {
            /// Creates an empty address range.
            #[inline]
            #[must_use]
            pub const fn empty() -> Self {
                Self {
                    start: $address_name::zero(),
                    count: 0,
                }
            }

            /// Creates a new address range with a base of `start` that contains `count` addresses.
            #[inline]
            #[must_use]
            pub const fn new(start: $address_name, count: $impl_type) -> Self {
                Self { start, count }
            }

            /// Returns the address at the start of this range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $address_name {
                self.start
            }

            /// Returns the number of bytes in the address range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                self.count
            }

            /// Returns the address at the inclusive end of this range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 bytes and a range of 1 byte.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $address_name {
                self.start().strict_add(self.count().saturating_sub(1))
            }

            /// Returns the address at the exclusive end of this range.
            #[inline]
            #[must_use]
            pub const fn end_exclusive(self) -> $address_name {
                self.start().strict_add(self.count())
            }

            /// Returns `true` if the address range is empty.
            #[inline]
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.count == 0
            }

            /// Returns `true` if the provided address is contained within this address range.
            #[inline]
            #[must_use]
            pub const fn contains(self, address: $address_name) -> bool {
                $crate::range::$contains(self.start().value(), self.count(), address.value())
            }

            /// Returns `true` if `self` and `other` share at least one address in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().value(),
                    self.count(),
                    other.start().value(),
                    other.count(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, count)) = $crate::range::$merge(
                    self.start().value(),
                    self.count(),
                    other.start().value(),
                    other.count(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), count);
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, count)) = $crate::range::$intersection(
                    self.start().value(),
                    self.count(),
                    other.start().value(),
                    other.count(),
                ) else {
                    return None;
                };

                let range = Self::new($address_name::new(start), count);
                Some(range)
            }

            /// Partitions `self` into three disjoint address ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the addresses in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — addresses in `self` strictly below `other`
            /// - `overlap` — addresses in `self` that are contained inside `other`
            /// - `upper`   — addresses in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().value(),
                    self.count(),
                    other.start().value(),
                    other.count(),
                );

                let lower = if let Some((start, count)) = result.0 {
                    let range = Self::new($address_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, count)) = result.1 {
                    let range = Self::new($address_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, count)) = result.2 {
                    let range = Self::new($address_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the addresses in this address range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $address_name> {
                (self.start().value()..self.end_exclusive().value()).map($address_name::new)
            }
        }
    };
}

/// Constructs an address chunk primitive.
#[macro_export]
macro_rules! implement_address_chunk {
    ($address_name:ident,
     $chunk_name:ident,
     $chunk_doc:expr,
     $impl_type:ident
    ) => {
        #[doc = $chunk_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_name($impl_type);

        impl $chunk_name {
            /// Creates a new address chunk with a value of 0.
            #[inline]
            #[must_use]
            pub const fn zero() -> Self {
                Self(0)
            }

            /// Creates a new address chunk with a value of `value`.
            #[inline]
            #[must_use]
            pub const fn new(value: $impl_type) -> Self {
                Self(value)
            }

            /// Returns the chunk containing `address` for the provided `chunk_size`.
            #[inline]
            #[must_use]
            pub const fn containing_address(
                address: $address_name,
                chunk_size: $impl_type,
            ) -> Self {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                Self(address.value() / chunk_size)
            }

            /// Returns the underyling value for this address chunk.
            #[inline]
            #[must_use]
            pub const fn number(self) -> $impl_type {
                self.0
            }

            /// Returns the address at the start of this address chunk.
            #[inline]
            #[must_use]
            pub const fn start_address(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                $address_name::new(self.0.strict_mul(chunk_size))
            }

            /// Returns the address at the inclusive end of this address chunk.
            #[inline]
            #[must_use]
            pub const fn end_address_inclusive(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                let sub_chunk_offset = chunk_size.strict_sub(1);
                self.start_address(chunk_size).strict_add(sub_chunk_offset)
            }

            /// Returns the address at the exclusive end of this address chunk.
            #[inline]
            #[must_use]
            pub const fn end_address_exclusive(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                self.start_address(chunk_size).strict_add(chunk_size)
            }

            /// Creates a new address chunk that is `count` chunks higher.
            ///
            /// Returns [`None`] if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn checked_add(self, count: $impl_type) -> Option<$chunk_name> {
                let Some(chunk_number) = self.0.checked_add(count) else {
                    return None;
                };

                Some(Self::new(chunk_number))
            }

            /// Creates a new address chunk that is `count` chunks higher.
            ///
            /// Panics if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn strict_add(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_add(count))
            }

            /// Creates a new address chunk that is `count` chunks lower.
            ///
            /// Returns `None` if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn checked_sub(self, count: $impl_type) -> Option<$chunk_name> {
                let Some(chunk_number) = self.0.checked_sub(count) else {
                    return None;
                };

                Some(Self::new(chunk_number))
            }

            /// Creates a new address chunk that is `count` chunks lower.
            ///
            /// Panics if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn strict_sub(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_sub(count))
            }

            /// Returns `true` if the address chunk is a multiple of `alignment`.
            ///
            /// `alignment` is given in bytes.
            #[inline]
            #[must_use]
            pub const fn is_aligned(self, chunk_size: $impl_type, alignment: $impl_type) -> bool {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());
                debug_assert!(alignment.is_power_of_two());

                let chunk_alignment = alignment.div_ceil(chunk_size);
                self.0.is_multiple_of(chunk_alignment)
            }

            /// Returns the greatest address chunk that is less than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// `alignment` is given in bytes.
            #[inline]
            #[must_use]
            pub const fn align_down(self, chunk_size: $impl_type, alignment: $impl_type) -> Self {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());
                debug_assert!(alignment.is_power_of_two());

                let number_alignment = alignment.div_ceil(chunk_size);
                Self::new((self.0 / number_alignment) * number_alignment)
            }

            /// Returns the smallest address chunk that is greater than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// `alignment` is given in bytes.
            ///
            /// Returns [`None`] if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn checked_align_up(
                self,
                chunk_size: $impl_type,
                alignment: $impl_type,
            ) -> Option<Self> {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());
                debug_assert!(alignment.is_power_of_two());

                let chunk_alignment = alignment.div_ceil(chunk_size);
                let Some(new_chunk) = self.0.checked_next_multiple_of(chunk_alignment) else {
                    return None;
                };

                Some(Self::new(new_chunk))
            }

            /// Returns the smallest address chunk that is greater than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// `alignment` is given in bytes.
            ///
            /// Panics if the operation would overflow.
            #[inline]
            #[must_use]
            pub const fn strict_align_up(
                self,
                chunk_size: $impl_type,
                alignment: $impl_type,
            ) -> Self {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());
                debug_assert!(alignment.is_power_of_two());

                let chunk_alignment = alignment.div_ceil(chunk_size);
                let Some(new_chunk) = self.0.checked_next_multiple_of(chunk_alignment) else {
                    panic!("failed to align address chunk up");
                };

                Self::new(new_chunk)
            }
        }
    };
}

/// Constructs an address chunk range primitive.
#[macro_export]
macro_rules! implement_address_chunk_range {
    ($address_name:ident,
     $address_range_name:ident,
     $chunk_name:ident,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     inclusive,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $chunk_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_range_name {
            /// The start of the range.
            start: $chunk_name,
            /// The inclusive end of the range.
            end: $chunk_name,
        }

        impl $chunk_range_name {
            /// Creates a new inclusive address chunk range of the form `start..=end`.
            #[inline]
            #[must_use]
            pub const fn new(start: $chunk_name, end: $chunk_name) -> Self {
                assert!(start.number() <= end.number());

                Self { start, end }
            }

            /// Returns the address chunk at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $chunk_name {
                self.start
            }

            /// Returns the address at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start_address(self, chunk_size: $impl_type) -> $address_name {
                self.start.start_address(chunk_size)
            }

            /// Returns the number of address chunks in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                let Some(difference) = self.end.number().checked_sub(self.start.number()) else {
                    return 0;
                };

                difference.strict_add(1)
            }

            /// Returns the number of bytes in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn byte_count(self, chunk_size: $impl_type) -> $impl_type {
                self.count().strict_mul(chunk_size)
            }

            /// Returns the address chunk at the inclusive end of this address chunk range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 address chunks and a range of 1 address chunk.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $chunk_name {
                self.end
            }

            /// Returns the address chunk at the exclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_exclusive(self) -> $chunk_name {
                self.end.strict_add(1)
            }

            /// Returns the address at the end of inclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_inclusive(self, chunk_size: $impl_type) -> $address_name {
                self.end_inclusive().end_address_inclusive(chunk_size)
            }

            /// Returns the address at the end of inclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_exclusive(self, chunk_size: $impl_type) -> $address_name {
                self.end_exclusive().start_address(chunk_size)
            }

            /// Returns the address range to which this address chunk range corresponds.
            #[inline]
            #[must_use]
            pub const fn address_range(self, chunk_size: $impl_type) -> $address_range_name {
                $address_range_name::new(
                    self.start_address(chunk_size),
                    self.end_address_inclusive(chunk_size),
                )
            }
            /// Returns `true` if the provided chunk is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains(self, chunk: $chunk_name) -> bool {
                $crate::range::$contains(
                    self.start().number(),
                    self.end_inclusive().number(),
                    chunk.number(),
                )
            }

            /// Returns `true` if the provided address is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains_address(
                self,
                chunk_size: $impl_type,
                address: $address_name,
            ) -> bool {
                self.address_range(chunk_size).contains(address)
            }

            /// Returns `true` if `self` and `other` share at least one chunk in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().number(),
                    self.end_inclusive().number(),
                    other.start().number(),
                    other.end_inclusive().number(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, end_inclusive)) = $crate::range::$merge(
                    self.start().number(),
                    self.end_inclusive().number(),
                    other.start().number(),
                    other.end_inclusive().number(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), $chunk_name::new(end_inclusive));
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, end_inclusive)) = $crate::range::$intersection(
                    self.start().number(),
                    self.end_inclusive().number(),
                    other.start().number(),
                    other.end_inclusive().number(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), $chunk_name::new(end_inclusive));
                Some(range)
            }

            /// Partitions `self` into three disjoint address chunk ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the address chunks in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — address chunks in `self` strictly below `other`
            /// - `overlap` — address chunks in `self` that are contained inside `other`
            /// - `upper`   — address chunks in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().number(),
                    self.end_inclusive().number(),
                    other.start().number(),
                    other.end_inclusive().number(),
                );

                let lower = if let Some((start, end_inclusive)) = result.0 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, end_inclusive)) = result.1 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, end_inclusive)) = result.2 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_inclusive));

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the chunks in this address chunk range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $chunk_name> {
                (self.start().number()..=self.end_inclusive().number()).map($chunk_name::new)
            }
        }
    };
    ($address_name:ident,
     $address_range_name:ident,
     $chunk_name:ident,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     exclusive,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $chunk_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_range_name {
            /// The start of the range.
            start: $chunk_name,
            /// The exclusive end of the range.
            end: $chunk_name,
        }

        impl $chunk_range_name {
            /// Creates an empty address chunk range.
            #[inline]
            #[must_use]
            pub const fn empty() -> Self {
                Self {
                    start: $chunk_name::zero(),
                    end: $chunk_name::zero(),
                }
            }

            /// Creates a new exclusive address chunk range of the form `start..end`.
            #[inline]
            #[must_use]
            pub const fn new(start: $chunk_name, end: $chunk_name) -> Self {
                assert!(start.number() <= end.number());

                Self { start, end }
            }

            /// Returns the address chunk at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $chunk_name {
                self.start
            }

            /// Returns the address at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start_address(self, chunk_size: $impl_type) -> $address_name {
                self.start.start_address(chunk_size)
            }

            /// Returns the number of address chunks in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                self.end.number().saturating_sub(self.start.number())
            }

            /// Returns the number of bytes in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn byte_count(self, chunk_size: $impl_type) -> $impl_type {
                self.count().strict_mul(chunk_size)
            }

            /// Returns the address chunk at the inclusive end of this address chunk range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 address chunks and a range of 1 address chunk.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $chunk_name {
                if !self.is_empty() {
                    self.end.strict_sub(1)
                } else {
                    self.start
                }
            }

            /// Returns the address chunk at the exclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_exclusive(self) -> $chunk_name {
                self.end
            }

            /// Returns the address at the end of inclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_inclusive(self, chunk_size: $impl_type) -> $address_name {
                if !self.is_empty() {
                    self.end_inclusive().end_address_inclusive(chunk_size)
                } else {
                    self.start.start_address(chunk_size)
                }
            }

            /// Returns the address at the end of exclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_exclusive(self, chunk_size: $impl_type) -> $address_name {
                self.end_exclusive().start_address(chunk_size)
            }

            /// Returns the address range to which this address chunk range corresponds.
            #[inline]
            #[must_use]
            pub const fn address_range(self, chunk_size: $impl_type) -> $address_range_name {
                $address_range_name::new(
                    self.start_address(chunk_size),
                    self.end_address_exclusive(chunk_size),
                )
            }

            /// Returns `true` if the address chunk range is empty.
            #[inline]
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.start.number() >= self.end.number()
            }

            /// Returns `true` if the provided chunk is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains(self, chunk: $chunk_name) -> bool {
                $crate::range::$contains(
                    self.start().number(),
                    self.end_exclusive().number(),
                    chunk.number(),
                )
            }

            /// Returns `true` if the provided address is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains_address(
                self,
                chunk_size: $impl_type,
                address: $address_name,
            ) -> bool {
                self.address_range(chunk_size).contains(address)
            }

            /// Returns `true` if `self` and `other` share at least one chunk in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().number(),
                    self.end_exclusive().number(),
                    other.start().number(),
                    other.end_exclusive().number(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, end_exclusive)) = $crate::range::$merge(
                    self.start().number(),
                    self.end_exclusive().number(),
                    other.start().number(),
                    other.end_exclusive().number(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), $chunk_name::new(end_exclusive));
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, end_exclusive)) = $crate::range::$intersection(
                    self.start().number(),
                    self.end_exclusive().number(),
                    other.start().number(),
                    other.end_exclusive().number(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), $chunk_name::new(end_exclusive));
                Some(range)
            }

            /// Partitions `self` into three disjoint address chunk ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the address chunks in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — address chunks in `self` strictly below `other`
            /// - `overlap` — address chunks in `self` that are contained inside `other`
            /// - `upper`   — address chunks in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().number(),
                    self.end_exclusive().number(),
                    other.start().number(),
                    other.end_exclusive().number(),
                );

                let lower = if let Some((start, end_exclusive)) = result.0 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, end_exclusive)) = result.1 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, end_exclusive)) = result.2 {
                    let range = Self::new($chunk_name::new(start), $chunk_name::new(end_exclusive));

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the chunks in this address chunk range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $chunk_name> {
                (self.start().number()..self.end_exclusive().number()).map($chunk_name::new)
            }
        }
    };
    ($address_name:ident,
     $address_range_name:ident,
     $chunk_name:ident,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     base_count,
     $impl_type:ident,
     $contains:ident,
     $overlaps:ident,
     $merge:ident,
     $intersection:ident,
     $partition:ident
    ) => {
        #[doc = $chunk_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_range_name {
            /// The start of the range.
            start: $chunk_name,
            /// The number of address chunks this range contains.
            count: $impl_type,
        }

        impl $chunk_range_name {
            /// Creates an empty address chunk range.
            #[inline]
            #[must_use]
            pub const fn empty() -> Self {
                Self {
                    start: $chunk_name::zero(),
                    count: 0,
                }
            }

            /// Creates a new exclusive address chunk range that starts at `start` and extends for
            /// `count` address chunks.
            #[inline]
            #[must_use]
            pub const fn new(start: $chunk_name, count: $impl_type) -> Self {
                Self { start, count }
            }

            /// Returns the address chunk at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start(self) -> $chunk_name {
                self.start
            }

            /// Returns the address at the start of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn start_address(self, chunk_size: $impl_type) -> $address_name {
                self.start.start_address(chunk_size)
            }

            /// Returns the number of address chunks in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn count(self) -> $impl_type {
                self.count
            }

            /// Returns the number of bytes in this address chunk range.
            #[inline]
            #[must_use]
            pub const fn byte_count(self, chunk_size: $impl_type) -> $impl_type {
                self.count().strict_mul(chunk_size)
            }

            /// Returns the address chunk at the inclusive end of this address chunk range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 address chunks and a range of 1 address chunk.
            #[inline]
            #[must_use]
            pub const fn end_inclusive(self) -> $chunk_name {
                self.start.strict_add(self.count.saturating_sub(1))
            }

            /// Returns the address chunk at the exclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_exclusive(self) -> $chunk_name {
                self.start.strict_add(self.count)
            }

            /// Returns the address at the end of inclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_inclusive(self, chunk_size: $impl_type) -> $address_name {
                if !self.is_empty() {
                    self.end_inclusive().end_address_inclusive(chunk_size)
                } else {
                    self.start.start_address(chunk_size)
                }
            }

            /// Returns the address at the end of exclusive end of this address chunk range.
            #[inline]
            #[must_use]
            pub const fn end_address_exclusive(self, chunk_size: $impl_type) -> $address_name {
                self.end_exclusive().start_address(chunk_size)
            }

            /// Returns the address range to which this address chunk range corresponds.
            #[inline]
            #[must_use]
            pub const fn address_range(self, chunk_size: $impl_type) -> $address_range_name {
                $address_range_name::new(
                    self.start_address(chunk_size),
                    self.byte_count(chunk_size),
                )
            }

            /// Returns `true` if the address chunk range is empty.
            #[inline]
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.count == 0
            }

            /// Returns `true` if the provided chunk is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains(self, chunk: $chunk_name) -> bool {
                $crate::range::$contains(self.start().number(), self.count(), chunk.number())
            }

            /// Returns `true` if the provided address is contained within this address chunk
            /// range.
            #[inline]
            #[must_use]
            pub const fn contains_address(
                self,
                chunk_size: $impl_type,
                address: $address_name,
            ) -> bool {
                self.address_range(chunk_size).contains(address)
            }

            /// Returns `true` if `self` and `other` share at least one chunk in their ranges.
            #[inline]
            #[must_use]
            pub const fn overlaps(self, other: Self) -> bool {
                $crate::range::$overlaps(
                    self.start().number(),
                    self.count(),
                    other.start().number(),
                    other.count(),
                )
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, [`None`] is returned.
            #[inline]
            #[must_use]
            pub const fn merge(self, other: Self) -> Option<Self> {
                let Some((start, count)) = $crate::range::$merge(
                    self.start().number(),
                    self.count(),
                    other.start().number(),
                    other.count(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), count);
                Some(range)
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then [`None`] will be returned.
            #[inline]
            #[must_use]
            pub const fn intersection(self, other: Self) -> Option<Self> {
                let Some((start, count)) = $crate::range::$intersection(
                    self.start().number(),
                    self.count(),
                    other.start().number(),
                    other.count(),
                ) else {
                    return None;
                };

                let range = Self::new($chunk_name::new(start), count);
                Some(range)
            }

            /// Partitions `self` into three disjoint address chunk ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the address chunks in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — address chunks in `self` strictly below `other`
            /// - `overlap` — address chunks in `self` that are contained inside `other`
            /// - `upper`   — address chunks in `self` strictly above `other`
            #[inline]
            #[must_use]
            pub const fn partition(
                self,
                other: Self,
            ) -> (Option<Self>, Option<Self>, Option<Self>) {
                let result = $crate::range::$partition(
                    self.start().number(),
                    self.count(),
                    other.start().number(),
                    other.count(),
                );

                let lower = if let Some((start, count)) = result.0 {
                    let range = Self::new($chunk_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                let overlap = if let Some((start, count)) = result.1 {
                    let range = Self::new($chunk_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                let upper = if let Some((start, count)) = result.2 {
                    let range = Self::new($chunk_name::new(start), count);

                    Some(range)
                } else {
                    None
                };

                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`][i] over all the chunks in this address chunk range.
            ///
            /// [i]: ::core::iter::Iterator
            #[inline]
            pub fn iter(self) -> impl Iterator<Item = $chunk_name> {
                (self.start().number()..self.end_exclusive().number()).map($chunk_name::new)
            }
        }
    };
}

#[cfg(test)]
mod test {
    use super::AddressSpaceDescriptor;

    // Real-world test cases.

    #[test]
    fn x86_64_48bit_canonical_validity() {
        // 48-bit canonical VA (4-level paging)
        let asd = AddressSpaceDescriptor::new(48, true);

        // Lower canonical half
        assert!(asd.is_valid(0x0000_0000_0000_0000));
        assert!(asd.is_valid(0x0000_7FFF_FFFF_FFFF));

        // Upper canonical half
        assert!(asd.is_valid(0xFFFF_8000_0000_0000));
        assert!(asd.is_valid(0xFFFF_FFFF_FFFF_FFFF));

        // Non-canonical (bit 47 = 0, but upper bits not zero)
        assert!(!asd.is_valid(0x0000_8000_0000_0000));

        // Non-canonical (bit 47 = 1, but upper bits not all ones)
        assert!(!asd.is_valid(0xFFFF_7FFF_FFFF_FFFF));
    }

    #[test]
    fn x86_64_48bit_canonical_ranges() {
        let asd = AddressSpaceDescriptor::new(48, true);
        let [(l_start, l_end), (u_start, u_end)] = asd.valid_ranges();

        assert_eq!(l_start, 0x0000_0000_0000_0000);
        assert_eq!(l_end, 0x0000_7FFF_FFFF_FFFF);

        assert_eq!(u_start, 0xFFFF_8000_0000_0000);
        assert_eq!(u_end, 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn x86_64_48bit_range_must_not_cross_halves() {
        let asd = AddressSpaceDescriptor::new(48, true);

        // Valid entirely in lower half
        assert!(asd.is_valid_range(0x0000_1000_0000_0000, 0x0000_2000_0000_0000,));

        // Crossing from lower into non-canonical hole
        assert!(!asd.is_valid_range(0x0000_7FFF_FFFF_F000, 0x0000_8000_0000_1000,));
    }

    #[test]
    fn x86_64_57bit_canonical_validity() {
        // 57-bit canonical VA (5-level paging / LA57)
        let asd = AddressSpaceDescriptor::new(57, true);

        let lower_end = (1u64 << 56) - 1;
        let upper_start = (!0u64) << 56;

        assert!(asd.is_valid(0));
        assert!(asd.is_valid(lower_end));
        assert!(asd.is_valid(upper_start));
        assert!(asd.is_valid(u64::MAX));

        // Non-canonical: bit 56 = 0 but upper bits not zero
        assert!(!asd.is_valid(1u64 << 56));
    }

    #[test]
    fn x86_64_52bit_physical_space() {
        let asd = AddressSpaceDescriptor::new(52, false);
        let max = (1u64 << 52) - 1;

        assert!(asd.is_valid(0));
        assert!(asd.is_valid(max));

        // Above physical width
        assert!(!asd.is_valid(max + 1));

        // Entire valid span
        assert!(asd.is_valid_range(0, max));

        // Crossing upper boundary
        assert!(!asd.is_valid_range(max - 0x1000, max + 1));
    }

    #[test]
    fn i686_legacy_32bit_physical() {
        let asd = AddressSpaceDescriptor::new(32, false);

        assert!(asd.is_valid(0));
        assert!(asd.is_valid(0xFFFF_FFFF));
        assert!(!asd.is_valid(0x1_0000_0000));

        assert!(asd.is_valid_range(0x1000, 0x2000));
        assert!(!asd.is_valid_range(0xFFFF_F000, 0x1_0000_0000));
    }

    // Edge cases.

    #[test]
    fn zero_bit_address_space() {
        let asd = AddressSpaceDescriptor::new(0, false);

        assert!(!asd.is_valid(0));
        assert!(!asd.is_valid(u64::MAX));

        assert!(!asd.is_valid_range(0, 0));
        assert_eq!(asd.valid_ranges(), [(1, 0), (1, 0)]);
    }

    #[test]
    fn full_64bit_address_space() {
        let asd = AddressSpaceDescriptor::new(64, false);

        assert!(asd.is_valid(0));
        assert!(asd.is_valid(u64::MAX));

        assert!(asd.is_valid_range(0, u64::MAX));

        let [(start, end), empty] = asd.valid_ranges();
        assert_eq!(start, 0);
        assert_eq!(end, u64::MAX);
        assert!(empty.0 > empty.1); // second range empty
    }

    #[test]
    fn one_bit_canonical() {
        let asd = AddressSpaceDescriptor::new(1, true);

        // bit 0 = 0 means upper bits must be zero
        assert!(asd.is_valid(0));

        // bit 0 = 1 means upper bits must all be ones
        assert!(asd.is_valid(u64::MAX));

        // non-canonical cases
        assert!(!asd.is_valid(1));
        assert!(!asd.is_valid(!0u64 - 1));

        let [(l_start, l_end), (u_start, u_end)] = asd.valid_ranges();
        assert_eq!((l_start, l_end), (0, 0));
        assert_eq!(u_start, (!0u64) << 0);
        assert_eq!(u_end, u64::MAX);
    }

    #[test]
    fn sixty_three_bit_canonical() {
        let asd = AddressSpaceDescriptor::new(63, true);

        let lower_end = (1u64 << 62) - 1;
        let upper_start = (!0u64) << 62;

        assert!(asd.is_valid(lower_end));
        assert!(asd.is_valid(upper_start));

        assert!(!asd.is_valid(lower_end + 1));
    }

    #[test]
    fn canonical_range_crossing_gap() {
        let asd = AddressSpaceDescriptor::new(48, true);

        let lower_end = 0x0000_7FFF_FFFF_FFFF;
        let upper_start = 0xFFFF_8000_0000_0000;

        // Crossing entire hole
        assert!(!asd.is_valid_range(lower_end, upper_start));
    }

    #[test]
    fn single_element_ranges() {
        let asd = AddressSpaceDescriptor::new(48, true);

        let addr = 0x0000_1234_5678_9ABC;
        assert!(asd.is_valid(addr));
        assert!(asd.is_valid_range(addr, addr));
    }

    #[test]
    fn wrapped_range_rejected() {
        let asd = AddressSpaceDescriptor::new(52, false);
        assert!(!asd.is_valid_range(1000, 999));
    }
}

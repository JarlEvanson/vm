//! Shared abstractions for physical and virtual memory.

/// A description of the parameters of an address space.
///
/// This can be utilized to describe both physical and virtual address spaces.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AddressSpaceDescriptor {
    /// The number of valid bits in the address space.
    implemented_bits: u8,
    /// If `true`, the upper bits must be a sign extension of the last implemented bit
    /// `(implemented_bits - 1)`.
    sign_extend_canonical: bool,
}

impl AddressSpaceDescriptor {
    /// Constructs a new [`AddressSpaceDescriptor`] with the specified parameters.
    ///
    /// # Panics
    ///
    /// Panics if the provided `implemented_bits` is greater than 64 bits, as that is nonsensical.
    pub const fn new(implemented_bits: u8, sign_extend_canonical: bool) -> Self {
        assert!(implemented_bits <= 64);

        Self {
            implemented_bits,
            sign_extend_canonical,
        }
    }

    /// Returns the number of implemented bits in the address space described by
    /// [`AddressSpaceDescriptor`].
    pub const fn implemented_bits(self) -> u8 {
        self.implemented_bits
    }

    /// Returns `true` if the address space described by [`AddressSpaceDescriptor`] requires
    /// canonical address to be sign extended.
    pub const fn sign_extended_canonical(self) -> bool {
        self.sign_extend_canonical
    }

    /// Returns `true` if the provided address is a valid address in the address space described by
    /// [`AddressSpaceDescriptor`].
    pub const fn is_valid(self, address: u64) -> bool {
        if self.implemented_bits == 64 {
            return true;
        } else if self.implemented_bits == 0 {
            return false;
        }

        let mask = (1u64 << self.implemented_bits) - 1;
        if self.sign_extend_canonical {
            let sign_bit = 1u64 << (self.implemented_bits - 1);
            let canonical_mask = !mask;

            let upper_bits = address & canonical_mask;
            if address & sign_bit == 0 {
                upper_bits == 0
            } else {
                upper_bits == canonical_mask
            }
        } else {
            address <= mask
        }
    }

    /// Returns `true` if the provided inclusive range `[start, end]` is entirely valid within the
    /// address space described by [`AddressSpaceDescriptor`].
    pub const fn is_valid_range(self, start: u64, end: u64) -> bool {
        // Reject wrapping ranges.
        if start > end {
            return false;
        }

        // An actual 64-bit address space means that every address is valid.
        if self.implemented_bits == 64 {
            return true;
        } else if self.implemented_bits == 0 {
            return false;
        }

        // Both endpoints must be valid.
        if !self.is_valid(start) || !self.is_valid(end) {
            return false;
        }

        // The start and end addresses are valid, so for a non-canonical address space, the entire
        // range must be valid since there is a single validity range.
        if !self.sign_extend_canonical {
            return true;
        }

        // Canonical case: must remain within one canonical half
        let sign_bit = 1u64 << (self.implemented_bits - 1);

        let start_high = (start & sign_bit) != 0;
        let end_high = (end & sign_bit) != 0;

        start_high == end_high
    }

    /// Returns the valid ranges for the address space described by [`AddressSpaceDescriptor`].
    ///
    /// If the valid ranges for the [`AddressSpaceDescriptor`] can be described by a single range,
    /// then the second range will be empty.
    pub const fn valid_ranges(self) -> [(u64, u64); 2] {
        if self.implemented_bits == 64 {
            return [(0, u64::MAX), (1, 0)];
        } else if self.implemented_bits == 0 {
            return [(1, 0); 2];
        }

        // Non-canonical: simple zero-extended space
        if !self.sign_extend_canonical {
            let max = (1u64 << self.implemented_bits) - 1;
            return [(0, max), (1, 0)];
        }

        // Canonical sign-extended case
        let sign_bit = 1u64 << (self.implemented_bits - 1);

        // Lower canonical range: sign bit = 0
        let lower_start = 0;
        let lower_end = sign_bit - 1;

        // Upper canonical range: sign bit = 1 and upper bits all ones
        let upper_start = (!0u64) << (self.implemented_bits - 1);
        let upper_end = u64::MAX;

        [(lower_start, lower_end), (upper_start, upper_end)]
    }
}

/// Constructs a set of memory primitives with the specified underlying type.
macro_rules! implement_address {
    ($address_name:ident,
     $address_doc:expr,
     $address_range_name:ident,
     $address_range_doc:expr,
     $chunk_name:ident,
     $chunk_doc:expr,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     $impl_type:ident
    ) => {
        #[doc = $address_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_name($impl_type);

        impl $address_name {
            /// Creates a new address with a value of 0.
            pub const fn zero() -> Self {
                Self(0)
            }

            /// Creates a new address with a value of `value`.
            pub const fn new(value: $impl_type) -> Self {
                Self(value)
            }

            /// Returns the underlying value for this address.
            pub const fn value(self) -> $impl_type {
                self.0
            }

            /// Creates a new address that is `count` bytes higher.
            ///
            /// Returns `None` if the operation would overflow.
            pub const fn checked_add(self, count: $impl_type) -> Option<Self> {
                let Some(new_address) = self.0.checked_add(count) else {
                    return None;
                };

                Some(Self::new(new_address))
            }

            /// Creates a new address that is `count` bytes higher.
            ///
            /// Panics if the operation would overflow.
            pub const fn strict_add(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_add(count))
            }

            /// Creates a new address that is `count` bytes lower.
            ///
            /// Returns `None` if the operation would underflow.
            pub const fn checked_sub(self, count: $impl_type) -> Option<Self> {
                let Some(new_address) = self.0.checked_sub(count) else {
                    return None;
                };

                Some(Self::new(new_address))
            }

            /// Creates a new address that is `count` bytes lower.
            ///
            /// Panics if the operation would underflow.
            pub const fn strict_sub(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_sub(count))
            }

            /// Returns `true` if the address is a multiple of `alignment`.
            pub const fn is_aligned(self, alignment: $impl_type) -> bool {
                debug_assert!(alignment.is_power_of_two());

                self.0.is_multiple_of(alignment)
            }

            /// Returns the greatest address that is less than or equal to `self` and is a
            /// multiple of `alignment`.
            pub const fn align_down(self, alignment: $impl_type) -> Self {
                debug_assert!(alignment.is_power_of_two());

                Self::new((self.0 / alignment) * alignment)
            }

            /// Returns the smallest address that is greater than or equal to `self` and is a
            /// multiple of `alignment`.
            ///
            /// Returns `None` if the operation would overflow.
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
            pub const fn strict_align_up(self, alignment: $impl_type) -> Self {
                debug_assert!(alignment.is_power_of_two());

                Self::new(
                    self.0
                        .checked_next_multiple_of(alignment)
                        .expect("failed to align Address up"),
                )
            }
        }

        #[doc = $address_range_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $address_range_name {
            /// The inclusive start of the range.
            start: $address_name,
            count: $impl_type,
        }

        impl $address_range_name {
            /// Creates an empty address range.
            pub const fn empty() -> Self {
                Self {
                    start: $address_name::zero(),
                    count: 0,
                }
            }

            /// Creates a new address range with a base of `start` that contains `count` addresses.
            pub const fn new(start: $address_name, count: $impl_type) -> Self {
                Self { start, count }
            }

            /// Creates a new address range with a base of `start` and an inclusive end of `end`.
            pub const fn from_inclusive(start: $address_name, end: $address_name) -> Self {
                let count = end.value().saturating_sub(start.value()).strict_add(1);
                Self { start, count }
            }

            /// Creates a new address range with a base of `start` and an exclusive end of `end`.
            pub const fn from_exclusive(start: $address_name, end: $address_name) -> Self {
                let count = end.value().saturating_sub(start.value());
                Self { start, count }
            }

            /// Returns the address at the start of this range.
            pub const fn start(self) -> $address_name {
                self.start
            }

            /// Returns the number of bytes in the address range.
            pub const fn count(self) -> $impl_type {
                self.count
            }

            /// Returns the address at the inclusive end of this range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 bytes and a range of 1 byte.
            pub const fn end_inclusive(self) -> $address_name {
                self.start.strict_add(self.count.saturating_sub(1))
            }

            /// Returns the address at the exclusive end of this range.
            pub const fn end_exclusive(self) -> $address_name {
                self.start.strict_add(self.count)
            }

            /// Returns `true` if the address range is empty.
            pub const fn is_empty(self) -> bool {
                self.count == 0
            }

            /// Returns `true` if the provided address is contained within this address range.
            pub const fn contains(self, address: $address_name) -> bool {
                self.start.value() <= address.value()
                    && (address.value() - self.start.value()) < self.count
            }

            /// Splits this address range into two seperate address ranges.
            ///
            /// - [start : start + index)
            /// - [start + index : end)
            ///
            /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the
            /// second address range will be empty.
            pub const fn split_at_index(self, index: $impl_type) -> Option<(Self, Self)> {
                if index > self.count {
                    return None;
                }

                let lower = Self::new(self.start, index);
                let upper = Self::new(lower.end_exclusive(), self.count() - lower.count());
                Some((lower, upper))
            }

            /// Splits this address range into two seperate address ranges.
            ///
            /// - [start : at)
            /// - [at : end)
            ///
            /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the
            /// second address range will be empty.
            pub const fn split_at(self, at: $address_name) -> Option<(Self, Self)> {
                if at.value() < self.start().value()
                    || (at.value() - self.start().value()) > self.count
                {
                    return None;
                }

                let lower = Self::from_exclusive(self.start, at);
                let upper = Self::new(at, self.count().strict_sub(lower.count()));
                Some((lower, upper))
            }

            /// Returns `true` if `self` and `other` share at least one byte in their ranges.
            pub const fn overlaps(self, other: Self) -> bool {
                !self.is_empty()
                    && !other.is_empty()
                    && self.start().value() <= other.end_inclusive().value()
                    && other.start().value() <= self.end_inclusive().value()
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, `None` is returned.
            pub const fn merge(self, other: Self) -> Option<Self> {
                if self.end_exclusive().value() < other.start().value()
                    || other.end_exclusive().value() < self.start().value()
                {
                    return None;
                }

                let start = if self.start().value() <= other.start().value() {
                    self.start()
                } else {
                    other.start()
                };

                let end = if self.end_exclusive().value() >= other.end_exclusive().value() {
                    self.end_exclusive()
                } else {
                    other.end_exclusive()
                };

                Some(Self::from_exclusive(start, end))
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then an empty address range will be returned.
            pub const fn intersection(self, other: Self) -> Self {
                let start = if self.start().value() >= other.start().value() {
                    self.start()
                } else {
                    other.start()
                };

                let end = if self.end_exclusive().value() <= other.end_exclusive().value() {
                    self.end_exclusive()
                } else {
                    other.end_exclusive()
                };

                Self::from_exclusive(start, end)
            }

            /// Partitions `self` into three disjoint address ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the addresses in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — addresses in `self` strictly below `other`
            /// - `overlap` — addresses in `self` that intersect `other`
            /// - `upper`   — addresses in `self` strictly above `other`
            pub const fn partition(self, other: Self) -> (Self, Self, Self) {
                let lower_end = if self.end_exclusive().value() <= other.start().value() {
                    self.end_exclusive()
                } else {
                    other.start()
                };

                let upper_start = if self.start().value() >= other.end_exclusive().value() {
                    self.start()
                } else {
                    other.end_exclusive()
                };

                let lower = $address_range_name::from_exclusive(self.start(), lower_end);
                let overlap = self.intersection(other);
                let upper = $address_range_name::from_exclusive(upper_start, self.end_exclusive());
                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`] over all the addresses in this address range.
            pub fn iter(self) -> impl Iterator<Item = $address_name> {
                (self.start().value()..self.end_exclusive().value()).map($address_name::new)
            }
        }

        #[doc = $chunk_doc]
        #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_name($impl_type);

        impl $chunk_name {
            /// Creates a new address chunk with a value of 0.
            pub const fn zero() -> Self {
                Self(0)
            }

            /// Creates a new address chunk with a value of `value`.
            pub const fn new(value: $impl_type) -> Self {
                Self(value)
            }

            /// Returns the chunk containing `address` for the provided `chunk_size`.
            pub const fn containing_address(
                address: $address_name,
                chunk_size: $impl_type,
            ) -> Self {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                Self(address.value() / chunk_size)
            }

            /// Returns the underyling value for this address chunk.
            pub const fn number(self) -> $impl_type {
                self.0
            }

            /// Returns the address at the start of this address chunk.
            pub const fn start_address(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                $address_name::new(self.0.strict_mul(chunk_size))
            }

            /// Returns the address at the inclusive end of this address chunk.
            pub const fn end_address_inclusive(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                let sub_chunk_offset = chunk_size.strict_sub(1);
                self.start_address(chunk_size).strict_add(sub_chunk_offset)
            }

            /// Returns the address at the exclusive end of this address chunk.
            pub const fn end_address_exclusive(self, chunk_size: $impl_type) -> $address_name {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                self.start_address(chunk_size).strict_add(chunk_size)
            }

            /// Creates a new address chunk that is `count` chunks higher.
            ///
            /// Returns `None` if the operation would overflow.
            pub const fn checked_add(self, count: $impl_type) -> Option<$chunk_name> {
                let Some(chunk_number) = self.0.checked_add(count) else {
                    return None;
                };

                Some(Self::new(chunk_number))
            }

            /// Creates a new address chunk that is `count` chunks higher.
            ///
            /// Panics if the operation would overflow.
            pub const fn strict_add(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_add(count))
            }

            /// Creates a new address chunk that is `count` chunks lower.
            ///
            /// Returns `None` if the operation would overflow.
            pub const fn checked_sub(self, count: $impl_type) -> Option<$chunk_name> {
                let Some(chunk_number) = self.0.checked_sub(count) else {
                    return None;
                };

                Some(Self::new(chunk_number))
            }

            /// Creates a new address chunk that is `count` chunks lower.
            ///
            /// Panics if the operation would overflow.
            pub const fn strict_sub(self, count: $impl_type) -> Self {
                Self::new(self.0.strict_sub(count))
            }

            /// Returns `true` if the address chunk is a multiple of `alignment`.
            ///
            /// `alignment` is given in bytes.
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
            /// `alignment` is given is bytes.
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
            /// `alignment` is given is bytes.
            ///
            /// Returns `None` if the operation would overflow.
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
            /// `alignment` is given is bytes.
            ///
            /// Panics if the operation would overflow.
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

        #[doc = $chunk_range_doc]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $chunk_range_name {
            start: $chunk_name,

            count: $impl_type,
        }

        impl $chunk_range_name {
            /// Creates an empty address chunk range.
            pub const fn empty() -> Self {
                Self {
                    start: $chunk_name::zero(),
                    count: 0,
                }
            }

            /// Creates a new address chunk range with a base of `start` that contains `count`
            /// chunks.
            pub const fn new(start: $chunk_name, count: $impl_type) -> Self {
                Self { start, count }
            }

            /// Creates a new address chunk range with a base of `start` and an inclusive end of
            /// `end`.
            pub const fn from_inclusive(start: $chunk_name, end: $chunk_name) -> Self {
                let count = end.number().saturating_sub(start.number()).strict_add(1);
                Self { start, count }
            }

            /// Creates a new address chunk range with a base of `start` and an exclusive end of
            /// `end`.
            pub const fn from_exclusive(start: $chunk_name, end: $chunk_name) -> Self {
                let count = end.number().saturating_sub(start.number());
                Self { start, count }
            }

            /// Returns the address chunk at the start of this range.
            pub const fn start(self) -> $chunk_name {
                self.start
            }

            /// Returns the number of address chunks in the address chunk range.
            pub const fn count(self) -> $impl_type {
                self.count
            }

            /// Returns the number of bytes in the address chunk range.
            pub const fn byte_count(self, chunk_size: $impl_type) -> $impl_type {
                debug_assert!(chunk_size != 0);
                debug_assert!(chunk_size.is_power_of_two());

                self.count.strict_mul(chunk_size)
            }

            /// Returns the address chunk at the inclusive end of this range.
            ///
            /// There is no method to differentiate between the result of this function when called
            /// with a range of 0 chunks and a range of 1 chunk.
            pub const fn end_inclusive(self) -> $chunk_name {
                self.start.strict_add(self.count.saturating_sub(1))
            }

            /// Returns the address chunk at the exclusive end of this range.
            pub const fn end_exclusive(self) -> $chunk_name {
                self.start.strict_add(self.count)
            }

            /// Returns `true` if the address chunk range is empty.
            pub const fn is_empty(self) -> bool {
                self.count == 0
            }

            /// Returns `true` if the provided chunk is contained within this address chunk range.
            pub const fn contains(self, chunk: $chunk_name) -> bool {
                self.start.number() <= chunk.number()
                    && (chunk.number() - self.start.number()) < self.count
            }

            /// Returns `true` if the provided chunk is contained within this address chunk range.
            pub const fn contains_address(
                self,
                chunk_size: $impl_type,
                address: $address_name,
            ) -> bool {
                self.contains($chunk_name::containing_address(address, chunk_size))
            }

            /// Splits this address chunk range into two seperate address chunk ranges.
            ///
            /// - [start : start + index)
            /// - [start + index : end)
            ///
            /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the
            /// second address range will be empty.
            pub const fn split_at_index(self, index: $impl_type) -> Option<(Self, Self)> {
                if index > self.count {
                    return None;
                }

                let lower = Self::new(self.start, index);
                let upper = Self::new(lower.end_exclusive(), self.count() - lower.count());
                Some((lower, upper))
            }

            /// Splits this address chunk range into two seperate address chunks ranges.
            ///
            /// - [start : at)
            /// - [at : end)
            ///
            /// Returns `None` if `index > self.count()`. If `index == `self.count()`, then the
            /// second address range will be empty.
            pub const fn split_at(self, at: $chunk_name) -> Option<(Self, Self)> {
                if at.number() < self.start().number()
                    || (at.number() - self.start().number()) > self.count
                {
                    return None;
                }

                let lower = Self::from_exclusive(self.start, at);
                let upper = Self::new(at, self.count().strict_sub(lower.count()));
                Some((lower, upper))
            }

            /// Returns `true` if `self` and `other` share at least one chunk in their ranges.
            pub const fn overlaps(self, other: Self) -> bool {
                !self.is_empty()
                    && !other.is_empty()
                    && self.start().number() <= other.end_inclusive().number()
                    && other.start().number() <= self.end_inclusive().number()
            }

            /// Returns the merged range if the two provided ranges are adjacent or overlapping.
            ///
            /// Otherwise, `None` is returned.
            pub const fn merge(self, other: Self) -> Option<Self> {
                if self.end_exclusive().number() < other.start().number()
                    || other.end_exclusive().number() < self.start().number()
                {
                    return None;
                }

                let start = if self.start().number() <= other.start().number() {
                    self.start()
                } else {
                    other.start()
                };

                let end = if self.end_exclusive().number() >= other.end_exclusive().number() {
                    self.end_exclusive()
                } else {
                    other.end_exclusive()
                };

                Some(Self::from_exclusive(start, end))
            }

            /// Returns the intersection of `self` and `other`.
            ///
            /// If the two ranges do not overlap, then an empty address range will be returned.
            pub const fn intersection(self, other: Self) -> Self {
                let start = if self.start().number() >= other.start().number() {
                    self.start()
                } else {
                    other.start()
                };

                let end = if self.end_exclusive().number() <= other.end_exclusive().number() {
                    self.end_exclusive()
                } else {
                    other.end_exclusive()
                };

                Self::from_exclusive(start, end)
            }

            /// Partitions `self` into three disjoint address ranges relative to `other`.
            ///
            /// The returned tuple `(lower, overlap, upper)` classifies the addresses in
            /// `self` according to their position relative to `other`:
            ///
            /// - `lower`   — addresses in `self` strictly below `other`
            /// - `overlap` — addresses in `self` that intersect `other`
            /// - `upper`   — addresses in `self` strictly above `other`
            pub fn partition(self, other: Self) -> (Self, Self, Self) {
                let lower_end = if self.end_exclusive().number() <= other.start().number() {
                    self.end_exclusive()
                } else {
                    other.start()
                };

                let upper_start = if self.start().number() >= other.end_exclusive().number() {
                    self.start()
                } else {
                    other.end_exclusive()
                };

                let lower = $chunk_range_name::from_exclusive(self.start(), lower_end);
                let overlap = self.intersection(other);
                let upper = $chunk_range_name::from_exclusive(upper_start, self.end_exclusive());
                (lower, overlap, upper)
            }

            /// Returns an [`Iterator`] over all the addresses in this address range.
            pub fn iter(self) -> impl Iterator<Item = $chunk_name> {
                (self.start().number()..self.end_exclusive().number()).map($chunk_name::new)
            }
        }
    };
    ($address_name:ident,
     $address_doc:expr,
     $address_range_name:ident,
     $address_range_doc:expr,
     $chunk_name:ident,
     $chunk_doc:expr,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     $impl_type:ident,
     false
    ) => {
        implement_address!(
            $address_name,
            $address_doc,
            $address_range_name,
            $address_range_doc,
            $chunk_name,
            $chunk_doc,
            $chunk_range_name,
            $chunk_range_doc,
            $impl_type
        );
    };
    ($address_name:ident,
     $address_doc:expr,
     $address_range_name:ident,
     $address_range_doc:expr,
     $chunk_name:ident,
     $chunk_doc:expr,
     $chunk_range_name:ident,
     $chunk_range_doc:expr,
     $impl_type:ident,
     true
    ) => {
        implement_address!(
            $address_name,
            $address_doc,
            $address_range_name,
            $address_range_doc,
            $chunk_name,
            $chunk_doc,
            $chunk_range_name,
            $chunk_range_doc,
            $impl_type
        );

        impl $address_name {
            #[doc = concat!("Converts the provided [`", stringify!($address_name), "`] to its [`Address`] representation")]
            pub const fn to_address(self) -> Address {
                Address::new(self.value())
            }
        }

        impl $address_range_name {
            #[doc = concat!("Converts the provided [`", stringify!($address_range_name), "`] to its [`AddressRange`] representation")]
            pub const fn to_address_range(self) -> AddressRange {
                AddressRange::new(self.start().to_address(), self.count())
            }
        }

        impl $chunk_name {
            #[doc = concat!("Converts the provided [`", stringify!($chunk_name), "`] to its [`AddressChunk`] representation")]
            pub const fn to_address_chunk(self) -> AddressChunk {
                AddressChunk::new(self.number())
            }
        }

        impl $chunk_range_name {
            #[doc = concat!("Converts the provided [`", stringify!($chunk_range_name), "`] to its [`AddressChunkRange`] representation")]
            pub const fn to_address_chunk_range(self) -> AddressChunkRange {
                AddressChunkRange::new(self.start().to_address_chunk(), self.count())
            }
        }
    };
}

implement_address!(
    Address,
    "A generic address in a generic address space.",
    AddressRange,
    "A range of [`Address`]es.",
    AddressChunk,
    "A chunk of addresses in the generic address space aligned to a chunk-sized boundary.",
    AddressChunkRange,
    "A range of contiguous [`AddressChunk`]s in a generic address space.",
    u64,
    false
);
implement_address!(
    PhysicalAddress,
    "An address in the physical address space.",
    PhysicalAddressRange,
    "A range of [`PhysicalAddress`]es.",
    Frame,
    "A chunk of physical memory aligned to a chunk-sized boundary",
    FrameRange,
    "A range of contiguous [`Frame`]s in the physical address space.",
    u64,
    true
);
implement_address!(
    VirtualAddress,
    "An address in the virtual address space.",
    VirtualAddressRange,
    "A range of [`VirtualAddress`]es.",
    Page,
    "A chunk of virtual memory aligned to a chunk-sized boundary.",
    PageRange,
    "A range of contiguous [`Page`]s in the virtual address space.",
    usize,
    false
);
implement_address!(
    VirtualAddressExternal,
    "An address in an external virtual address space.",
    VirtalAddressRangeExternal,
    "A range of [`VirtualAddressExternal`]s.",
    PageExternal,
    "A chunk of external virtual memory aligned to a chunk-sized boundary.",
    PageRangeExternal,
    "A range of contiguous [`PageExternal`]s in an external virtual address space.",
    u64,
    true
);

impl Address {
    /// Returns `true` if this [`Address`] is valid in the address space described by the provided
    /// [`AddressSpaceDescriptor`].
    pub const fn is_valid(self, descriptor: &AddressSpaceDescriptor) -> bool {
        descriptor.is_valid(self.value())
    }
}

impl AddressRange {
    /// Returns `true` if this [`AddressRange`] is valid in the address space described by the
    /// provided [`AddressSpaceDescriptor`].
    pub const fn is_valid(self, descriptor: &AddressSpaceDescriptor) -> bool {
        descriptor.is_valid_range(self.start().value(), self.end_inclusive().value())
    }
}

impl AddressChunk {
    /// Returns `true` if this [`AddressChunk`] is valid in the address space described by the
    /// provided [`AddressSpaceDescriptor`].
    pub const fn is_valid(self, chunk_size: u64, descriptor: &AddressSpaceDescriptor) -> bool {
        descriptor.is_valid_range(
            self.start_address(chunk_size).value(),
            self.end_address_inclusive(chunk_size).value(),
        )
    }
}

impl AddressChunkRange {
    /// Returns `true` if this [`AddressChunkRange`] is valid in the address space described by the
    /// provided [`AddressSpaceDescriptor`].
    pub const fn is_valid(self, chunk_size: u64, descriptor: &AddressSpaceDescriptor) -> bool {
        descriptor.is_valid_range(
            self.start().start_address(chunk_size).value(),
            self.end_inclusive()
                .end_address_inclusive(chunk_size)
                .value(),
        )
    }
}

impl VirtualAddress {
    /// Converts the provided [`VirtualAddress`] to its [`Address`] representation.
    #[cfg(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    ))]
    pub const fn to_address(self) -> Address {
        use conversion::usize_to_u64;

        Address::new(usize_to_u64(self.value()))
    }
}

impl VirtualAddressRange {
    /// Converts the provided [`VirtualAddressRange`] to its [`AddressRange`] representation.
    #[cfg(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    ))]
    pub const fn to_address_range(self) -> AddressRange {
        use conversion::usize_to_u64;

        AddressRange::new(self.start().to_address(), usize_to_u64(self.count()))
    }
}

impl Page {
    /// Converts the provided [`Page`] to its [`AddressChunk`] representation.
    #[cfg(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    ))]
    pub const fn to_address_chunk(self) -> AddressChunk {
        use conversion::usize_to_u64;

        AddressChunk::new(usize_to_u64(self.number()))
    }
}

impl PageRange {
    /// Converts the provided [`PageRange`] to its [`AddressChunkRange`] representation.
    #[cfg(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64"
    ))]
    pub const fn to_address_chunk_range(self) -> AddressChunkRange {
        use conversion::usize_to_u64;

        AddressChunkRange::new(self.start().to_address_chunk(), usize_to_u64(self.count()))
    }
}

#[cfg(test)]
mod test {
    use super::{AddressChunk, AddressChunkRange, AddressSpaceDescriptor};

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
    fn x86_32_legacy_32bit_physical() {
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

    #[test]
    fn empty_chunk_range_behavior() {
        let range = AddressChunkRange::empty();

        assert!(range.is_empty());
        assert_eq!(range.count(), 0);

        assert!(!range.contains(AddressChunk::new(0)));
    }

    #[test]
    fn split_at_boundaries() {
        let range = AddressChunkRange::new(AddressChunk::new(10), 5);

        let (lower, upper) = range.split_at(AddressChunk::new(10)).unwrap();
        assert!(lower.is_empty());
        assert_eq!(upper.count(), 5);

        let (lower, upper) = range.split_at(AddressChunk::new(15)).unwrap();
        assert_eq!(lower.count(), 5);
        assert!(upper.is_empty());
    }

    #[test]
    fn merge_contiguous_ranges() {
        let a = AddressChunkRange::new(AddressChunk::new(0), 5);
        let b = AddressChunkRange::new(AddressChunk::new(5), 3);

        let merged = a.merge(b).unwrap();
        assert_eq!(merged.start().number(), 0);
        assert_eq!(merged.count(), 8);
    }

    #[test]
    fn intersection_non_overlap() {
        let a = AddressChunkRange::new(AddressChunk::new(0), 4);
        let b = AddressChunkRange::new(AddressChunk::new(5), 3);

        let inter = a.intersection(b);
        assert!(inter.is_empty());
    }

    #[test]
    fn partition_other_strictly_below() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(0), 5);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert_eq!(lower, other_r);
        assert!(overlap.is_empty());
        assert!(upper.is_empty());
    }

    #[test]
    fn partition_other_strictly_above() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(25), 5);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert!(overlap.is_empty());
        assert_eq!(upper, other_r);
    }

    #[test]
    fn partition_exact_match() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(10), 10);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert_eq!(overlap, other_r);
        assert!(upper.is_empty());
    }

    #[test]
    fn partition_partial_overlap_lower() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(5), 10);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert_eq!(lower, AddressChunkRange::new(AddressChunk::new(5), 5));
        assert_eq!(overlap, AddressChunkRange::new(AddressChunk::new(10), 5));
        assert!(upper.is_empty());
    }

    #[test]
    fn partition_partial_overlap_upper() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(15), 10);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert_eq!(overlap, AddressChunkRange::new(AddressChunk::new(15), 5));
        assert_eq!(upper, AddressChunkRange::new(AddressChunk::new(20), 5));
    }

    #[test]
    fn partition_other_contains_self() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(5), 20);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert_eq!(lower, AddressChunkRange::new(AddressChunk::new(5), 5));
        assert_eq!(overlap, self_r);
        assert_eq!(upper, AddressChunkRange::new(AddressChunk::new(20), 5));
    }

    #[test]
    fn partition_self_contains_other() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 20);
        let other_r = AddressChunkRange::new(AddressChunk::new(15), 5);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert_eq!(overlap, other_r);
        assert!(upper.is_empty());
    }

    #[test]
    fn partition_touching_lower_boundary() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(0), 10);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert_eq!(lower, other_r);
        assert!(overlap.is_empty());
        assert!(upper.is_empty());
    }

    #[test]
    fn partition_touching_upper_boundary() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(20), 10);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert!(overlap.is_empty());
        assert_eq!(upper, other_r);
    }

    #[test]
    fn partition_empty_other() {
        let self_r = AddressChunkRange::new(AddressChunk::new(10), 10);
        let other_r = AddressChunkRange::new(AddressChunk::new(15), 0);

        let (lower, overlap, upper) = other_r.partition(self_r);

        assert!(lower.is_empty());
        assert!(overlap.is_empty());
        assert!(upper.is_empty());
    }
}

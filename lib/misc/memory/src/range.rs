//! Useful functionality for interacting with ranges.

/// Implements functions for interacting with ranges.
macro_rules! implement_range_functions {
    ($data_type:ident,
     $contains_inclusive:ident,
     $contains_exclusive:ident,
     $contains_base_count:ident,
     $overlaps_inclusive:ident,
     $overlaps_exclusive:ident,
     $overlaps_base_count:ident,
     $merge_inclusive:ident,
     $merge_exclusive:ident,
     $merge_base_count:ident,
     $intersection_inclusive:ident,
     $intersection_exclusive:ident,
     $intersection_base_count:ident,
     $partition_inclusive:ident,
     $partition_exclusive:ident,
     $partition_base_count:ident
    ) => {
        /// Returns `true` if the range contains `value`.
        #[inline]
        #[must_use]
        pub const fn $contains_inclusive(
            start: $data_type,
            end: $data_type,
            value: $data_type,
        ) -> bool {
            debug_assert!(start <= end);

            start <= value && value <= end
        }

        /// Returns `true` if the range contains `value`.
        #[inline]
        #[must_use]
        pub const fn $contains_exclusive(
            start: $data_type,
            end: $data_type,
            value: $data_type,
        ) -> bool {
            debug_assert!(start <= end);

            start <= value && value < end
        }

        /// Returns `true` if the range contains `value`.
        #[inline]
        #[must_use]
        pub const fn $contains_base_count(
            start: $data_type,
            count: $data_type,
            value: $data_type,
        ) -> bool {
            // The subtraction is safe due to the first check.
            start <= value && value - start < count
        }

        /// Returns `true` if the ranges overlap.
        #[inline]
        #[must_use]
        pub const fn $overlaps_inclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> bool {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            start_0 <= end_1 && start_1 <= end_0
        }

        /// Returns `true` if the ranges overlap.
        #[inline]
        #[must_use]
        pub const fn $overlaps_exclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> bool {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            start_0 != end_0 && start_1 != end_1 && start_0 < end_1 && start_1 < end_0
        }

        /// Returns `true` if the ranges overlap.
        #[inline]
        #[must_use]
        pub const fn $overlaps_base_count(
            start_0: $data_type,
            count_0: $data_type,
            start_1: $data_type,
            count_1: $data_type,
        ) -> bool {
            if count_0 == 0 || count_1 == 0 {
                return false;
            }

            if start_0 <= start_1 {
                start_1 - start_0 < count_0
            } else {
                start_0 - start_1 < count_1
            }
        }

        /// Returns the merged range if the first and the second range are adjacent or overlap. Otherwise,
        /// returns [`None`].
        #[inline]
        #[must_use]
        pub const fn $merge_inclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            let adjacent_or_overlap =
                start_1 <= end_0.saturating_add(1) && start_0 <= end_1.saturating_add(1);

            if adjacent_or_overlap {
                let start = if start_0 <= start_1 { start_0 } else { start_1 };
                let end = if end_0 >= end_1 { end_0 } else { end_1 };

                Some((start, end))
            } else {
                None
            }
        }

        /// Returns the merged range if the first and the second range are adjacent or overlap. Otherwise,
        /// returns [`None`].
        #[inline]
        #[must_use]
        pub const fn $merge_exclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            if start_0 <= end_1 && start_1 <= end_0 {
                let start = if start_0 <= start_1 { start_0 } else { start_1 };
                let end = if end_0 >= end_1 { end_0 } else { end_1 };

                Some((start, end))
            } else {
                None
            }
        }

        /// Returns the merged range if the first and the second range are adjacent or overlap. Otherwise,
        /// returns [`None`].
        #[inline]
        #[must_use]
        pub const fn $merge_base_count(
            mut start_0: $data_type,
            mut count_0: $data_type,
            mut start_1: $data_type,
            mut count_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            if start_0 > start_1 {
                ::core::mem::swap(&mut start_0, &mut start_1);
                ::core::mem::swap(&mut count_0, &mut count_1);
            }

            let difference = start_1 - start_0;
            if difference > count_0 {
                None
            } else {
                // Ranges are overlapping or adjacent.
                let count = if difference.strict_add(count_1) > count_0 {
                    difference.strict_add(count_1)
                } else {
                    count_0
                };

                Some((start_0, count))
            }
        }

        /// Returns the intersection of the first range and the second range.
        ///
        /// If the two ranges do not overlap, then [`None`] is returned.
        #[inline]
        #[must_use]
        pub const fn $intersection_inclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            let intersection_start = if start_0 >= start_1 { start_0 } else { start_1 };
            let intersection_end = if end_0 <= end_1 { end_0 } else { end_1 };

            if intersection_start <= intersection_end {
                Some((intersection_start, intersection_end))
            } else {
                None
            }
        }

        /// Returns the intersection of the first range and the second range.
        ///
        /// If the two ranges do not overlap, then [`None`] is returned.
        #[inline]
        #[must_use]
        pub const fn $intersection_exclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            let intersection_start = if start_0 >= start_1 { start_0 } else { start_1 };
            let intersection_end = if end_0 <= end_1 { end_0 } else { end_1 };

            if intersection_start < intersection_end {
                Some((intersection_start, intersection_end))
            } else {
                None
            }
        }

        /// Returns the intersection of the first range and the second range.
        ///
        /// If the two ranges do not overlap, then [`None`] is returned.
        #[inline]
        #[must_use]
        pub const fn $intersection_base_count(
            start_0: $data_type,
            count_0: $data_type,
            start_1: $data_type,
            count_1: $data_type,
        ) -> Option<($data_type, $data_type)> {
            if count_0 == 0 || count_1 == 0 {
                return None;
            }

            let intersection_start = if start_0 > start_1 { start_0 } else { start_1 };

            let remainder_0 = if intersection_start < start_0 {
                count_0
            } else {
                let difference = intersection_start - start_0;
                count_0.saturating_sub(difference)
            };

            let remainder_1 = if intersection_start < start_1 {
                count_1
            } else {
                let difference = intersection_start - start_1;
                count_1.saturating_sub(difference)
            };

            let intersection_count = if remainder_0 < remainder_1 {
                remainder_0
            } else {
                remainder_1
            };

            if intersection_count != 0 {
                Some((intersection_start, intersection_count))
            } else {
                None
            }
        }

        /// Partitions the first range into three disjoint ranges.
        ///
        /// The returned tuple `(lower, overlap, upper)` is a classification of the first range's
        /// elements according to their position relative to the second range.
        ///
        /// - `lower`   — elements in `self` strictly below `other`
        /// - `overlap` — elements in `self` are contained inside `other`
        /// - `upper`   — elements in `self` strictly above `other`
        #[inline]
        #[must_use]
        pub const fn $partition_inclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> (
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
        ) {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            let lower = if start_0 < start_1 {
                let lower_end = if end_0 < start_1 { end_0 } else { start_1 - 1 };
                Some((start_0, lower_end))
            } else {
                None
            };

            let overlap_start = if start_0 > start_1 { start_0 } else { start_1 };
            let overlap_end = if end_0 < end_1 { end_0 } else { end_1 };

            let overlap = if overlap_start <= overlap_end {
                Some((overlap_start, overlap_end))
            } else {
                None
            };

            let upper = if end_0 > end_1 {
                let upper_start = if start_0 > end_1 { start_0 } else { end_1 + 1 };
                Some((upper_start, end_0))
            } else {
                None
            };

            (lower, overlap, upper)
        }

        /// Partitions the first range into three disjoint ranges.
        ///
        /// The returned tuple `(lower, overlap, upper)` is a classification of the first range's
        /// elements according to their position relative to the second range.
        ///
        /// - `lower`   — elements in `self` strictly below `other`
        /// - `overlap` — elements in `self` are contained inside `other`
        /// - `upper`   — elements in `self` strictly above `other`
        #[inline]
        #[must_use]
        pub const fn $partition_exclusive(
            start_0: $data_type,
            end_0: $data_type,
            start_1: $data_type,
            end_1: $data_type,
        ) -> (
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
        ) {
            debug_assert!(start_0 <= end_0);
            debug_assert!(start_1 <= end_1);

            let lower = if start_0 < start_1 {
                let end_lower = if end_0 < start_1 { end_0 } else { start_1 };
                if start_0 < end_lower {
                    Some((start_0, end_lower))
                } else {
                    None
                }
            } else {
                None
            };

            let overlap_start = if start_0 > start_1 { start_0 } else { start_1 };
            let overlap_end = if end_0 < end_1 { end_0 } else { end_1 };

            let overlap = if overlap_start < overlap_end {
                Some((overlap_start, overlap_end))
            } else {
                None
            };

            let upper = if end_0 > end_1 {
                let start_upper = if start_0 > end_1 { start_0 } else { end_1 };
                if start_upper < end_0 {
                    Some((start_upper, end_0))
                } else {
                    None
                }
            } else {
                None
            };

            (lower, overlap, upper)
        }

        /// Partitions the first range into three disjoint ranges.
        ///
        /// The returned tuple `(lower, overlap, upper)` is a classification of the first range's
        /// elements according to their position relative to the second range.
        ///
        /// - `lower`   — elements in `self` strictly below `other`
        /// - `overlap` — elements in `self` are contained inside `other`
        /// - `upper`   — elements in `self` strictly above `other`
        #[inline]
        #[must_use]
        pub const fn $partition_base_count(
            start_0: $data_type,
            count_0: $data_type,
            start_1: $data_type,
            count_1: $data_type,
        ) -> (
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
            Option<($data_type, $data_type)>,
        ) {
            if count_0 == 0 {
                return (None, None, None);
            }

            let (start_0, count_0, lower) = if start_0 < start_1 {
                let difference = start_1 - start_0;
                let count = if difference <= count_0 {
                    difference
                } else {
                    count_0
                };
                (start_1, count_0 - count, Some((start_0, count)))
            } else {
                (start_0, count_0, None)
            };

            let (start_0, count_0, overlap) = if count_0 != 0 && start_0 >= start_1 {
                let difference = start_0 - start_1;
                if difference < count_1 {
                    let max_count = count_1 - difference;
                    let count = if count_0 <= max_count {
                        count_0
                    } else {
                        max_count
                    };

                    (
                        start_0.strict_add(count),
                        count_0 - count,
                        Some((start_0, count)),
                    )
                } else {
                    (start_0, count_0, None)
                }
            } else {
                (start_0, count_0, None)
            };

            let upper = if count_0 != 0 {
                Some((start_0, count_0))
            } else {
                None
            };

            (lower, overlap, upper)
        }
    };
}

implement_range_functions!(
    u8,
    contains_inclusive_u8,
    contains_exclusive_u8,
    contains_base_count_u8,
    overlaps_inclusive_u8,
    overlaps_exclusive_u8,
    overlaps_base_count_u8,
    merge_inclusive_u8,
    merge_exclusive_u8,
    merge_base_count_u8,
    intersection_inclusive_u8,
    intersection_exclusive_u8,
    intersection_base_count_u8,
    partition_inclusive_u8,
    partition_exclusive_u8,
    partition_base_count_u8
);
implement_range_functions!(
    u16,
    contains_inclusive_u16,
    contains_exclusive_u16,
    contains_base_count_u16,
    overlaps_inclusive_u16,
    overlaps_exclusive_u16,
    overlaps_base_count_u16,
    merge_inclusive_u16,
    merge_exclusive_u16,
    merge_base_count_u16,
    intersection_inclusive_u16,
    intersection_exclusive_u16,
    intersection_base_count_u16,
    partition_inclusive_u16,
    partition_exclusive_u16,
    partition_base_count_u16
);
implement_range_functions!(
    u32,
    contains_inclusive_u32,
    contains_exclusive_u32,
    contains_base_count_u32,
    overlaps_inclusive_u32,
    overlaps_exclusive_u32,
    overlaps_base_count_u32,
    merge_inclusive_u32,
    merge_exclusive_u32,
    merge_base_count_u32,
    intersection_inclusive_u32,
    intersection_exclusive_u32,
    intersection_base_count_u32,
    partition_inclusive_u32,
    partition_exclusive_u32,
    partition_base_count_u32
);
implement_range_functions!(
    u64,
    contains_inclusive_u64,
    contains_exclusive_u64,
    contains_base_count_u64,
    overlaps_inclusive_u64,
    overlaps_exclusive_u64,
    overlaps_base_count_u64,
    merge_inclusive_u64,
    merge_exclusive_u64,
    merge_base_count_u64,
    intersection_inclusive_u64,
    intersection_exclusive_u64,
    intersection_base_count_u64,
    partition_inclusive_u64,
    partition_exclusive_u64,
    partition_base_count_u64
);
implement_range_functions!(
    u128,
    contains_inclusive_u128,
    contains_exclusive_u128,
    contains_base_count_u128,
    overlaps_inclusive_u128,
    overlaps_exclusive_u128,
    overlaps_base_count_u128,
    merge_inclusive_u128,
    merge_exclusive_u128,
    merge_base_count_u128,
    intersection_inclusive_u128,
    intersection_exclusive_u128,
    intersection_base_count_u128,
    partition_inclusive_u128,
    partition_exclusive_u128,
    partition_base_count_u128
);
implement_range_functions!(
    usize,
    contains_inclusive_usize,
    contains_exclusive_usize,
    contains_base_count_usize,
    overlaps_inclusive_usize,
    overlaps_exclusive_usize,
    overlaps_base_count_usize,
    merge_inclusive_usize,
    merge_exclusive_usize,
    merge_base_count_usize,
    intersection_inclusive_usize,
    intersection_exclusive_usize,
    intersection_base_count_usize,
    partition_inclusive_usize,
    partition_exclusive_usize,
    partition_base_count_usize
);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_contains_inclusive() {
        assert!(contains_inclusive_u64(10, 20, 10));
        assert!(contains_inclusive_u64(10, 20, 15));
        assert!(contains_inclusive_u64(10, 20, 20));
        assert!(!contains_inclusive_u64(10, 20, 9));
        assert!(!contains_inclusive_u64(10, 20, 21));

        assert!(contains_inclusive_u64(10, 10, 10));
        assert!(!contains_inclusive_u64(10, 10, 9));
        assert!(!contains_inclusive_u64(10, 10, 11));
    }

    #[test]
    fn test_contains_exclusive() {
        assert!(contains_exclusive_u64(10, 20, 10));
        assert!(contains_exclusive_u64(10, 20, 15));
        assert!(!contains_exclusive_u64(10, 20, 20));
        assert!(!contains_exclusive_u64(10, 20, 9));
        assert!(!contains_exclusive_u64(10, 20, 21));

        assert!(!contains_exclusive_u64(10, 10, 10));
    }

    #[test]
    fn test_contains_base_count() {
        assert!(contains_base_count_u64(10, 10, 10));
        assert!(contains_base_count_u64(10, 10, 15));
        assert!(!contains_base_count_u64(10, 10, 20));
        assert!(!contains_base_count_u64(10, 10, 9));
        assert!(!contains_base_count_u64(10, 10, 21));

        assert!(!contains_base_count_u64(10, 0, 10));
    }

    #[test]
    fn test_overlaps_inclusive() {
        assert!(overlaps_inclusive_u64(10, 20, 10, 20));
        assert!(overlaps_inclusive_u64(10, 20, 15, 25));
        assert!(overlaps_inclusive_u64(10, 20, 5, 15));

        assert!(overlaps_inclusive_u64(10, 20, 12, 18));
        assert!(overlaps_inclusive_u64(12, 18, 10, 20));

        assert!(overlaps_inclusive_u64(10, 20, 20, 30));
        assert!(overlaps_inclusive_u64(20, 30, 10, 20));

        assert!(!overlaps_inclusive_u64(10, 20, 21, 30));
        assert!(!overlaps_inclusive_u64(21, 30, 10, 20));
    }

    #[test]
    fn test_overlaps_exclusive() {
        assert!(overlaps_exclusive_u64(10, 20, 10, 20));

        assert!(overlaps_exclusive_u64(10, 20, 15, 25));
        assert!(overlaps_exclusive_u64(5, 15, 10, 20));

        assert!(overlaps_exclusive_u64(10, 20, 12, 18));
        assert!(overlaps_exclusive_u64(12, 18, 10, 20));

        assert!(!overlaps_exclusive_u64(10, 20, 20, 30));
        assert!(!overlaps_exclusive_u64(20, 30, 10, 20));

        assert!(!overlaps_exclusive_u64(10, 20, 25, 35));

        assert!(!overlaps_exclusive_u64(10, 10, 5, 15));
    }

    #[test]
    fn test_overlaps_base_count() {
        assert!(overlaps_base_count_u64(10, 10, 10, 10));

        assert!(overlaps_base_count_u64(10, 10, 15, 10));
        assert!(overlaps_base_count_u64(5, 10, 10, 10));

        assert!(overlaps_base_count_u64(10, 10, 12, 6));
        assert!(overlaps_base_count_u64(12, 6, 10, 10));

        assert!(!overlaps_base_count_u64(10, 10, 20, 10));
        assert!(!overlaps_base_count_u64(20, 10, 10, 10));

        assert!(!overlaps_base_count_u64(10, 10, 25, 10));

        assert!(!overlaps_base_count_u64(10, 0, 5, 10));
    }

    #[test]
    fn test_merge_inclusive() {
        assert_eq!(merge_inclusive_u64(10, 20, 10, 20), Some((10, 20)));

        assert_eq!(merge_inclusive_u64(10, 20, 15, 25), Some((10, 25)));
        assert_eq!(merge_inclusive_u64(15, 25, 10, 20), Some((10, 25)));

        assert_eq!(merge_inclusive_u64(10, 30, 15, 25), Some((10, 30)));
        assert_eq!(merge_inclusive_u64(15, 25, 10, 30), Some((10, 30)));

        assert_eq!(merge_inclusive_u64(10, 20, 21, 30), Some((10, 30)));
        assert_eq!(merge_inclusive_u64(21, 30, 10, 20), Some((10, 30)));

        assert_eq!(merge_inclusive_u64(10, 20, 22, 30), None);
        assert_eq!(merge_inclusive_u64(22, 30, 10, 20), None);

        assert_eq!(merge_inclusive_u64(5, 5, 6, 6), Some((5, 6)));
        assert_eq!(merge_inclusive_u64(5, 5, 7, 7), None);

        assert_eq!(
            merge_inclusive_u64(0, u64::MAX - 1, u64::MAX, u64::MAX),
            Some((0, u64::MAX))
        );
    }

    #[test]
    fn test_merge_exclusive() {
        assert_eq!(merge_exclusive_u64(4, 8, 4, 8), Some((4, 8)));

        assert_eq!(merge_exclusive_u64(1, 5, 3, 7), Some((1, 7)));
        assert_eq!(merge_exclusive_u64(3, 7, 1, 5), Some((1, 7)));

        assert_eq!(merge_exclusive_u64(1, 5, 5, 10), Some((1, 10)));
        assert_eq!(merge_exclusive_u64(5, 10, 1, 5), Some((1, 10)));

        assert_eq!(merge_exclusive_u64(1, 10, 3, 7), Some((1, 10)));
        assert_eq!(merge_exclusive_u64(3, 7, 1, 10), Some((1, 10)));

        assert_eq!(merge_exclusive_u64(1, 5, 7, 10), None);
        assert_eq!(merge_exclusive_u64(7, 10, 1, 5), None);

        assert_eq!(merge_exclusive_u64(1, 5, 5, 5), Some((1, 5)));
        assert_eq!(merge_exclusive_u64(5, 5, 5, 10), Some((5, 10)));
        assert_eq!(merge_exclusive_u64(5, 5, 5, 5), Some((5, 5)));
    }

    #[test]
    fn test_merge_base_count() {
        assert_eq!(merge_base_count_u64(4, 4, 4, 4), Some((4, 4)));

        assert_eq!(merge_base_count_u64(1, 4, 3, 4), Some((1, 6)));
        assert_eq!(merge_base_count_u64(3, 4, 1, 4), Some((1, 6)));

        assert_eq!(merge_base_count_u64(1, 4, 5, 5), Some((1, 9)));
        assert_eq!(merge_base_count_u64(5, 5, 1, 4), Some((1, 9)));

        assert_eq!(merge_base_count_u64(1, 9, 3, 4), Some((1, 9)));
        assert_eq!(merge_base_count_u64(3, 4, 1, 9), Some((1, 9)));

        assert_eq!(merge_base_count_u64(1, 4, 7, 3), None);
        assert_eq!(merge_base_count_u64(7, 3, 1, 4), None);

        assert_eq!(merge_base_count_u64(1, 4, 5, 0), Some((1, 4)));
        assert_eq!(merge_base_count_u64(5, 0, 5, 5), Some((5, 5)));
        assert_eq!(merge_base_count_u64(5, 0, 5, 0), Some((5, 0)));
    }

    #[test]
    fn test_intersection_inclusive() {
        assert_eq!(intersection_inclusive_u64(0, 10, 15, 25), None);
        assert_eq!(intersection_inclusive_u64(15, 25, 0, 10), None);

        assert_eq!(intersection_inclusive_u64(0, 10, 10, 20), Some((10, 10)));
        assert_eq!(intersection_inclusive_u64(10, 20, 0, 10), Some((10, 10)));

        assert_eq!(intersection_inclusive_u64(0, 15, 10, 25), Some((10, 15)));
        assert_eq!(intersection_inclusive_u64(10, 25, 0, 15), Some((10, 15)));

        assert_eq!(intersection_inclusive_u64(0, 30, 10, 20), Some((10, 20)));
        assert_eq!(intersection_inclusive_u64(10, 20, 0, 30), Some((10, 20)));

        assert_eq!(intersection_inclusive_u64(5, 15, 5, 15), Some((5, 15)));

        assert_eq!(intersection_inclusive_u64(5, 5, 5, 5), Some((5, 5)));
        assert_eq!(intersection_inclusive_u64(5, 5, 0, 10), Some((5, 5)));
        assert_eq!(intersection_inclusive_u64(5, 5, 6, 10), None);

        assert_eq!(
            intersection_inclusive_u64(u64::MAX - 10, u64::MAX, u64::MAX - 5, u64::MAX),
            Some((u64::MAX - 5, u64::MAX))
        );
    }

    #[test]
    fn test_intersection_exclusive() {
        assert_eq!(intersection_exclusive_u64(0, 10, 15, 25), None);
        assert_eq!(intersection_exclusive_u64(15, 25, 0, 10), None);

        assert_eq!(intersection_exclusive_u64(0, 10, 10, 20), None);
        assert_eq!(intersection_exclusive_u64(10, 20, 0, 10), None);

        assert_eq!(intersection_exclusive_u64(0, 15, 10, 25), Some((10, 15)));
        assert_eq!(intersection_exclusive_u64(10, 25, 0, 15), Some((10, 15)));

        assert_eq!(intersection_exclusive_u64(0, 30, 10, 20), Some((10, 20)));
        assert_eq!(intersection_exclusive_u64(10, 20, 0, 30), Some((10, 20)));

        assert_eq!(intersection_exclusive_u64(5, 15, 5, 15), Some((5, 15)));

        assert_eq!(intersection_exclusive_u64(0, 10, 10, 10), None);
        assert_eq!(intersection_exclusive_u64(5, 5, 5, 5), None);

        assert_eq!(
            intersection_exclusive_u64(u64::MAX - 10, u64::MAX, u64::MAX - 5, u64::MAX),
            Some((u64::MAX - 5, u64::MAX))
        );
    }

    #[test]
    fn test_intersection_base_count() {
        assert_eq!(intersection_base_count_u64(0, 10, 15, 10), None);
        assert_eq!(intersection_base_count_u64(15, 10, 0, 10), None);

        assert_eq!(intersection_base_count_u64(0, 10, 10, 10), None);
        assert_eq!(intersection_base_count_u64(10, 10, 0, 10), None);

        assert_eq!(intersection_base_count_u64(0, 15, 10, 15), Some((10, 5)));
        assert_eq!(intersection_base_count_u64(10, 15, 0, 15), Some((10, 5)));

        assert_eq!(intersection_base_count_u64(0, 30, 10, 10), Some((10, 10)));
        assert_eq!(intersection_base_count_u64(10, 10, 0, 30), Some((10, 10)));

        assert_eq!(intersection_base_count_u64(5, 10, 5, 10), Some((5, 10)));

        assert_eq!(intersection_base_count_u64(0, 10, 10, 0), None);
        assert_eq!(intersection_base_count_u64(5, 0, 5, 0), None);

        assert_eq!(
            intersection_base_count_u64(u64::MAX - 10, 10, u64::MAX - 5, 5),
            Some((u64::MAX - 5, 5))
        );
    }

    #[test]
    fn test_partition_inclusive() {
        {
            let (lower, overlap, upper) = partition_inclusive_u64(10, 50, 20, 40);
            assert_eq!(lower, Some((10, 19)));
            assert_eq!(overlap, Some((20, 40)));
            assert_eq!(upper, Some((41, 50)));
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(10, 30, 20, 40);
            assert_eq!(lower, Some((10, 19)));
            assert_eq!(overlap, Some((20, 30)));
            assert_eq!(upper, None);
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(30, 50, 20, 40);
            assert_eq!(lower, None);
            assert_eq!(overlap, Some((30, 40)));
            assert_eq!(upper, Some((41, 50)));
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(25, 35, 20, 40);
            assert_eq!(lower, None);
            assert_eq!(overlap, Some((25, 35)));
            assert_eq!(upper, None);
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(10, 15, 20, 40);
            assert_eq!(lower, Some((10, 15)));
            assert_eq!(overlap, None);
            assert_eq!(upper, None);
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(45, 50, 20, 40);
            assert_eq!(lower, None);
            assert_eq!(overlap, None);
            assert_eq!(upper, Some((45, 50)));
        }

        {
            let (lower, overlap, upper) = partition_inclusive_u64(20, 40, 20, 40);
            assert_eq!(lower, None);
            assert_eq!(overlap, Some((20, 40)));
            assert_eq!(upper, None);
        }
    }

    #[test]
    fn test_partition_exclusive() {
        let s0 = 10;
        let e0 = 20;

        assert_eq!(
            partition_exclusive_u64(s0, e0, 0, 5),
            (None, None, Some((10, 20)))
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 0, 10),
            (None, None, Some((10, 20)))
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 5, 15),
            (None, Some((10, 15)), Some((15, 20)))
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 10, 15),
            (None, Some((10, 15)), Some((15, 20)))
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 12, 18),
            (Some((10, 12)), Some((12, 18)), Some((18, 20)))
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 15, 20),
            (Some((10, 15)), Some((15, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 10, 20),
            (None, Some((10, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 5, 25),
            (None, Some((10, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 10, 25),
            (None, Some((10, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 5, 20),
            (None, Some((10, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 15, 25),
            (Some((10, 15)), Some((15, 20)), None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 20, 30),
            (Some((10, 20)), None, None)
        );

        assert_eq!(
            partition_exclusive_u64(s0, e0, 25, 30),
            (Some((10, 20)), None, None)
        );

        assert_eq!(partition_exclusive_u64(15, 15, 10, 20), (None, None, None));

        assert_eq!(
            partition_exclusive_u64(s0, e0, 15, 15),
            (Some((10, 15)), None, Some((15, 20)))
        );
    }

    #[test]
    fn test_partition_base_count() {
        let s0 = 10;
        let c0 = 10;

        assert_eq!(
            partition_base_count_u64(s0, c0, 0, 5),
            (None, None, Some((10, 10)))
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 0, 10),
            (None, None, Some((10, 10)))
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 5, 10),
            (None, Some((10, 5)), Some((15, 5)))
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 10, 5),
            (None, Some((10, 5)), Some((15, 5)))
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 12, 6),
            (Some((10, 2)), Some((12, 6)), Some((18, 2)))
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 15, 5),
            (Some((10, 5)), Some((15, 5)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 10, 10),
            (None, Some((10, 10)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 5, 20),
            (None, Some((10, 10)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 10, 15),
            (None, Some((10, 10)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 5, 15),
            (None, Some((10, 10)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 15, 10),
            (Some((10, 5)), Some((15, 5)), None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 20, 10),
            (Some((10, 10)), None, None)
        );

        assert_eq!(
            partition_base_count_u64(s0, c0, 25, 5),
            (Some((10, 10)), None, None)
        );

        assert_eq!(partition_base_count_u64(15, 0, 10, 10), (None, None, None));

        assert_eq!(
            partition_base_count_u64(s0, c0, 15, 0),
            (Some((10, 5)), None, Some((15, 5)))
        );
    }
}

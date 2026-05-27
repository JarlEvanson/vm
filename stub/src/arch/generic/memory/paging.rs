//! Definitions, interfaces, and wrappers for architecture-specific code related to paging or
//! virtual memory management.

use memory::AddressSpaceDescriptor;

use crate::platform::{MapError, MappingType, Permissions};

/// A trait representing low-level [`ExternalVirtualAddress`] translation.
///
/// The [`TranslationScheme`] trait provides an abstraction for mapping, translating, and unmapping
/// [`ExternalFrame`]s from an address space.
///
/// # Safety
///
/// Manipulating [`ExternalFrame`]s and [`ExternalPage`]s can have side effects and may violate
/// memory safety if not carefully managed. Implementors must ensure that proper address validation
/// occurrs, the APIs are implemented as specified, and any implementations adhere to the
/// architectural specification if necessary.
pub trait TranslationScheme {
    /// Returns the [`AddressSpaceDescriptor`] that describes the address space that the
    /// [`TranslationScheme`] receives as input.
    fn input_descriptor(&self) -> AddressSpaceDescriptor;

    /// Returns the [`AddressSpaceDescriptor`] that describes the address space that the
    /// [`TranslationScheme`] outputs into.
    fn output_descriptor(&self) -> AddressSpaceDescriptor;

    /// Returns the size, in bytes, of the smallest translation granule.
    fn chunk_size(&self) -> u64;

    /// Maps the provided [`ExternalFrameRange`] into the input address space with the
    /// requested [`Permissions`] and [`MappingType`].
    ///
    /// # Errors
    ///
    /// - [`MapError::FindFreeRegionError`]: Returned if the [`TranslationScheme`] does not have a
    ///   suitable [`ExternalPageRange`] for the requested mapping.
    /// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`][f]s
    ///   that are required to map the requested [`ExternalFrameRange`] into memory.
    ///
    /// [f]: crate::platform::Frame
    fn map(
        &mut self,
        strategy: SearchStrategy,
        output: ExternalFrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<ExternalPageRange, MapError> {
        let Some(free_region) = self.find_free_region(strategy, output) else {
            return Err(MapError::FindFreeRegionError);
        };

        self.map_at(free_region, output, permissions, mapping_type)
            .map(|_| free_region)
    }

    /// Maps the provided [`ExternalFrameRange`] into the input address space at
    /// [`ExternalPageRange`]the requested [`Permissions`] and [`MappingType`].
    ///
    /// # Errors
    ///
    /// - [`MapError::FindFreeRegionError`]: Returned if the requested [`ExternalPageRange`] has
    ///   already been mapped.
    /// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`][f]s
    ///   that are required to map the requested [`ExternalFrameRange`] into memory.
    ///
    /// [f]: crate::platform::Frame
    fn map_at(
        &mut self,
        input: ExternalPageRange,
        output: ExternalFrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<(), MapError>;

    /// Identity maps the provided [`ExternalFrameRange`] into the input address space with the
    /// requested [`Permissions`].
    ///
    /// # Errors
    ///
    /// - [`MapError::FindFreeRegionError`]: Returned if the requested [`ExternalPageRange`] has
    ///   already been mapped.
    /// - [`MapError::FrameAllocation`]: Returned when an error occurs when allocating [`Frame`][f]s
    ///   that are required to map the requested [`ExternalFrameRange`] into memory.
    ///
    /// [f]: crate::platform::Frame
    fn map_identity(
        &mut self,
        output: ExternalFrameRange,
        permissions: Permissions,
    ) -> Result<ExternalPageRange, MapError> {
        let input = ExternalPageRange::new(
            ExternalPage::new(output.start().number()),
            ExternalPage::new(output.end_inclusive().number()),
        );

        self.map_at(input, output, permissions, MappingType::Normal)?;
        Ok(input)
    }

    /// Unmaps the [`ExternalPageRange`] from the [`TranslationScheme`].
    ///
    /// # Safety
    ///
    /// The [`ExternalPageRange`] in the input address space must not be in use.
    unsafe fn unmap(&mut self, input: ExternalPageRange);

    /// Translates the provided [`ExternalVirtualAddress`] in the input address space into its
    /// corresponding [`ExternalPhysicalAddress`] in the output address space and returns its
    /// associated [`Permissions`] and [`MappingType`].
    fn translate(
        &self,
        address: ExternalVirtualAddress,
    ) -> Option<(Permissions, MappingType, ExternalPhysicalAddress)>;

    /// Locates a free region in the input address space suitable to map the `output`
    /// [`ExternalFrameRange`]. The search is carried out according to the provided
    /// [`SearchStrategy`].
    fn find_free_region(
        &self,
        strategy: SearchStrategy,
        output: ExternalFrameRange,
    ) -> Option<ExternalPageRange> {
        if output.is_empty() {
            return None;
        }

        match strategy {
            SearchStrategy::BottomUp => {
                for (start, end) in self.input_descriptor().valid_ranges() {
                    if start >= end {
                        // Skip empty ranges.
                        continue;
                    }

                    let start = ExternalVirtualAddress::new(start);
                    let end = ExternalVirtualAddress::new(end);

                    let start_bound = {
                        let Some(address) = start.checked_align_up(self.chunk_size()) else {
                            continue;
                        };

                        ExternalPage::containing_address(address, self.chunk_size())
                    };
                    let end_bound = {
                        let end_page = ExternalPage::containing_address(end, self.chunk_size());
                        if end_page.end_address_inclusive(self.chunk_size()) <= end {
                            end_page
                        } else {
                            if end_page.number() == 0 {
                                continue;
                            } else {
                                end_page.strict_sub(1)
                            }
                        }
                    };

                    let mut current = start_bound;

                    let mut start_page = current;
                    let mut count = 0;
                    while current <= end_bound {
                        if self
                            .translate(current.start_address(self.chunk_size()))
                            .is_none()
                        {
                            count += 1;
                            if count == 1 {
                                start_page = current;
                            }

                            if count == output.count() {
                                let range = ExternalPageRange::new(start_page, current);
                                return Some(range);
                            }
                        } else {
                            // Successfully translated (which means that this region is not free)
                            // and thus we reset tracked values.
                            count = 0;
                        }
                        current = current.strict_add(1);
                    }
                }

                None
            }
            SearchStrategy::TopDown => {
                'range_loop: for (start, end) in
                    self.input_descriptor().valid_ranges().into_iter().rev()
                {
                    if start >= end {
                        // Skip empty ranges.
                        continue;
                    }

                    let start = ExternalVirtualAddress::new(start);
                    let end = ExternalVirtualAddress::new(end);

                    let start_bound = {
                        let Some(address) = start.checked_align_up(self.chunk_size()) else {
                            continue;
                        };

                        ExternalPage::containing_address(address, self.chunk_size())
                    };
                    let end_bound = {
                        let end_page = ExternalPage::containing_address(end, self.chunk_size());
                        if end_page.end_address_inclusive(self.chunk_size()) <= end {
                            end_page
                        } else {
                            if end_page.number() == 0 {
                                continue;
                            } else {
                                end_page.strict_sub(1)
                            }
                        }
                    };

                    let mut current = end_bound;

                    let mut end_page = current;
                    let mut count = 0;
                    while current >= start_bound {
                        if self
                            .translate(current.start_address(self.chunk_size()))
                            .is_none()
                        {
                            count += 1;
                            if count == 1 {
                                end_page = current;
                            }

                            if count == output.count() {
                                let range = ExternalPageRange::new(current, end_page);
                                return Some(range);
                            }
                        } else {
                            // Successful translation (which means that this region is not free) and
                            // thus we must reset the tracked values.
                            count = 0;
                        }

                        if current.number() == 0 {
                            continue 'range_loop;
                        }
                        current = current.strict_sub(1);
                    }
                }

                None
            }
        }
    }
}

/// Strategies for searching the input address space to locate an unmapped region capable of
/// holding a requested address range.
pub enum SearchStrategy {
    /// Allocates from the bottom (lowest addresses) of the available address space upwards.
    BottomUp,
    /// Allocates from the top (highest addresses) of the available address space downward.
    TopDown,
}

memory::implement_address!(
    ExternalPhysicalAddress,
    "An external physical address.",
    u64
);
memory::implement_address_range!(
    ExternalPhysicalAddress,
    ExternalPhysicalAddressRange,
    "A contiguous range of [`ExternalPhysicalAddress`]es.",
    base_count,
    u64,
    contains_base_count_u64,
    overlaps_base_count_u64,
    merge_base_count_u64,
    intersection_base_count_u64,
    partition_base_count_u64
);
memory::implement_address_chunk!(
    ExternalPhysicalAddress,
    ExternalFrame,
    "A `chunk-size`d contiguous range of [`ExternalPhysicalAddress`]es with `chunk-size` alignment.",
    u64
);
memory::implement_address_chunk_range!(
    ExternalPhysicalAddress,
    ExternalPhysicalAddressRange,
    ExternalFrame,
    ExternalFrameRange,
    "A contiguous range of [`ExternalFrame`]s.",
    base_count,
    u64,
    contains_base_count_u64,
    overlaps_base_count_u64,
    merge_base_count_u64,
    intersection_base_count_u64,
    partition_base_count_u64
);

memory::implement_address!(ExternalVirtualAddress, "An external virtual address.", u64);
memory::implement_address_range!(
    ExternalVirtualAddress,
    ExternalVirtualAddressRange,
    "A contiguous range of [`ExternalVirtualAddress`]es.",
    inclusive,
    u64,
    contains_base_count_u64,
    overlaps_base_count_u64,
    merge_base_count_u64,
    intersection_base_count_u64,
    partition_base_count_u64
);
memory::implement_address_chunk!(
    ExternalVirtualAddress,
    ExternalPage,
    "A `chunk-size`d contiguous range of [`ExternalVirtualAddress`]es with `chunk-size` alignment.",
    u64
);
memory::implement_address_chunk_range!(
    ExternalVirtualAddress,
    ExternalVirtualAddressRange,
    ExternalPage,
    ExternalPageRange,
    "A contiguous range of [`ExternalPage`]s.",
    inclusive,
    u64,
    contains_base_count_u64,
    overlaps_base_count_u64,
    merge_base_count_u64,
    intersection_base_count_u64,
    partition_base_count_u64
);

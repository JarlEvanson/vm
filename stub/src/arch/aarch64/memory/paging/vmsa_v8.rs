//! Implementation of VMSAv8 paging.

use core::mem;

use aarch64::{
    EL, Granule, PhysicalAddressSpaceSize,
    msr::{
        Aarch64MemoryModelFeatureRegister0EL1, CurrentEl, TcrEL1,
        raw::{read_ttbr0_el1, read_ttbr1_el1},
    },
    paging::{AddressSize, vmsa_v8::TranslationDescriptor},
};
use conversion::usize_to_u64;
use memory::AddressSpaceDescriptor;

use crate::{
    arch::generic::memory::paging::{
        ExternalFrameRange, ExternalPageRange, ExternalPhysicalAddress, ExternalVirtualAddress,
        TranslationScheme,
    },
    platform::{
        AllocationPolicy, FrameRange, MapError, MappingType, OutOfMemory, Permissions,
        PhysicalAddress, allocate_physical, read_u64_at, write_u64_at,
    },
};

/// Implementation of [`TranslationScheme`] for `aarch64` address translation.
pub struct VmsaV8TranslationScheme {
    /// The [`Granule`] used for this paging scheme.
    granule: Granule,

    /// If `true`, TTBR0 is enabled.
    ttbr0_enable: bool,
    /// If `true`, TTBR1 is enabled.
    ttbr1_enable: bool,

    /// The physical address of the table pointed to by TTBR0.
    ttbr0: PhysicalAddress,
    /// The physical address of the table pointed to by TTBR1.
    ttbr1: PhysicalAddress,

    /// The size offset of the memory region addressed by TTBR0.
    t0sz: u8,
    /// The size offset of the memory region addressed by TTBR1.
    t1sz: u8,

    /// The output address space size.
    output: PhysicalAddressSpaceSize,
}

impl VmsaV8TranslationScheme {
    /// Creates a new [`VmsaV8TranslationScheme`] with the maximum supported settings.
    pub fn max_supported() -> Option<Self> {
        let el = CurrentEl::get().el();
        let mut scheme = if el == EL::EL1 {
            // SAFETY:
            //
            // Since the program is in [`EL::EL1`], it is safe to read
            // [`VmsaV8MemoryModelFeatureRegister0EL1`].
            let feature_reg = unsafe { Aarch64MemoryModelFeatureRegister0EL1::get() };

            let granule = if feature_reg.granule_64_supported() {
                Granule::Page64KiB
            } else if feature_reg.granule_16_supported() {
                Granule::Page16KiB
            } else if feature_reg.granule_4_supported() {
                Granule::Page4KiB
            } else {
                unreachable!()
            };

            let ttbr0_enable = true;
            let ttbr1_enable = true;

            let t0sz = 16;
            let t1sz = 16;

            let output = feature_reg.physical_address_bits();

            Self {
                granule,

                ttbr0_enable,
                ttbr1_enable,

                ttbr0: PhysicalAddress::zero(),
                ttbr1: PhysicalAddress::zero(),

                t0sz,
                t1sz,

                output,
            }
        } else if el == EL::EL2 {
            todo!("implement EL2")
        } else {
            unreachable!()
        };

        if scheme.ttbr0_enable {
            scheme.ttbr0 = scheme.allocate_zeroed_table()?.start_address();
        }

        if scheme.ttbr1_enable {
            scheme.ttbr1 = scheme.allocate_zeroed_table()?.start_address();
        }

        Some(scheme)
    }

    /// Creates a new [`VmsaV8TranslationScheme`] by taking over the current page tables
    /// referenced by `CR3`.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`VmsaV8TranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        let el = CurrentEl::get().el();
        let mut scheme = if el == EL::EL1 {
            // SAFETY:
            //
            // Since the program is in [`EL::EL1`], it is safe to read
            // [`TcrEL1`].
            let tcr_el1 = unsafe { TcrEL1::get() };
            assert_eq!(
                tcr_el1.translation_granule_0(),
                tcr_el1.translation_granule_1()
            );

            let granule = tcr_el1.translation_granule_0();

            let ttbr0_enable = !tcr_el1.walk_disable_0();
            let ttbr1_enable = !tcr_el1.walk_disable_1();

            let t0sz = tcr_el1.size_offset_0();
            let t1sz = tcr_el1.size_offset_1();

            let output = tcr_el1.ipas();

            Self {
                granule,

                ttbr0_enable,
                ttbr1_enable,

                ttbr0: PhysicalAddress::zero(),
                ttbr1: PhysicalAddress::zero(),

                t0sz,
                t1sz,

                output,
            }
        } else if el == EL::EL2 {
            todo!("implement EL2")
        } else {
            unreachable!()
        };

        if scheme.ttbr0_enabled() {
            // SAFETY:
            //
            // The current mode is EL1 and the MMU is supported.
            scheme.ttbr0 = unsafe { PhysicalAddress::new(read_ttbr0_el1() & 0xFFFF_FFFF_FFFE) }
        }

        if scheme.ttbr1_enabled() {
            // SAFETY:
            //
            // The current mode is EL1 and the MMU is supported.
            scheme.ttbr1 = unsafe { PhysicalAddress::new(read_ttbr1_el1() & 0xFFFF_FFFF_FFFE) }
        }

        Some(scheme)
    }

    /// Returns the size, in bytes, of minimal translation region.
    pub const fn granule(&self) -> Granule {
        self.granule
    }

    /// Returns `true` if the `TTBR0` page tables are in use.
    pub const fn ttbr0_enabled(&self) -> bool {
        self.ttbr0_enable
    }

    /// Returns `true` if the `TTBR1` page tables are in use.
    pub const fn ttbr1_enabled(&self) -> bool {
        self.ttbr1_enable
    }

    /// Returns the location of the table referenced by `TTBR0`.
    pub const fn ttbr0(&self) -> u64 {
        self.ttbr0.value()
    }

    /// Returns the location of the table referenced by `TTBR1`.
    pub const fn ttbr1(&self) -> u64 {
        self.ttbr1.value()
    }

    /// Returns the value of `T0SZ`.
    pub const fn t0sz(&self) -> u8 {
        self.t0sz
    }

    /// Returns the value of `T1SZ`.
    pub const fn t1sz(&self) -> u8 {
        self.t1sz
    }

    /// Returns the size, in bits, of the output address.
    pub const fn ipa(&self) -> PhysicalAddressSpaceSize {
        self.output
    }

    /// Returns the number of entries in each table.
    const fn entries_per_table(&self) -> u32 {
        self.granule.size() / 8
    }

    /// Returns the number of bits used to index each level.
    const fn index_bits_per_level(&self) -> u32 {
        self.entries_per_table().ilog2()
    }

    /// Returns the total number of levels.
    const fn levels(&self, txsz: u8) -> u32 {
        ((64 - txsz) as u32 - self.granule.size().ilog2()).div_ceil(self.index_bits_per_level())
    }

    /// Returns the [`AddressSize`] that corresponds to size of the physical memory region.
    const fn output_size(&self) -> AddressSize {
        match self.output {
            PhysicalAddressSpaceSize::Bits32
            | PhysicalAddressSpaceSize::Bits36
            | PhysicalAddressSpaceSize::Bits40
            | PhysicalAddressSpaceSize::Bits42
            | PhysicalAddressSpaceSize::Bits44
            | PhysicalAddressSpaceSize::Bits48 => AddressSize::Bits48,
            PhysicalAddressSpaceSize::Bits52 => AddressSize::Bits52,
            PhysicalAddressSpaceSize::Bits56 => unimplemented!(),
        }
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<FrameRange> {
        let max_physical = 1u64
            .checked_shl(u32::from(self.output.to_val()))
            .unwrap_or(u64::MAX);
        let range = allocate_physical(
            self.chunk_size(),
            self.chunk_size(),
            AllocationPolicy::InclusiveMax(max_physical),
        )
        .ok()?;

        for i in 0..self.chunk_size() / usize_to_u64(mem::size_of::<u64>()) {
            let address = range.range().start_address().strict_add(i * 8);
            if !write_u64_at(address, 0) {
                return None;
            }
        }

        let frame_range = range.range();
        mem::forget(range);
        Some(frame_range)
    }
}

impl TranslationScheme for VmsaV8TranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        let lower_bits = if self.ttbr0_enable { 64 - self.t0sz } else { 0 };

        let upper_bits = if self.ttbr1_enable { 64 - self.t1sz } else { 0 };

        AddressSpaceDescriptor::bit_range(lower_bits, upper_bits)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(self.output.to_val(), false)
    }

    fn chunk_size(&self) -> u64 {
        u64::from(self.granule.size())
    }

    fn map_at(
        &mut self,
        input: ExternalPageRange,
        output: ExternalFrameRange,
        _permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<(), MapError> {
        // TODO: Implement permissions.
        if mapping_type != MappingType::Normal {
            todo!("implement cacheability modifiers")
        }

        let input_range = input.address_range(self.chunk_size());
        if !self.input_descriptor().is_valid_range(
            input_range.start().value(),
            input_range.end_inclusive().value(),
        ) {
            return Err(MapError::FindFreeRegionError);
        }

        let output_range = output.address_range(self.chunk_size());
        if !self.output_descriptor().is_valid_range(
            output_range.start().value(),
            output_range.end_inclusive().value(),
        ) {
            return Err(MapError::FindFreeRegionError);
        }

        if input.count() != output.count() {
            return Err(MapError::FindFreeRegionError);
        }

        let (ttbr, txsz) = if AddressSpaceDescriptor::new(64 - self.t0sz, false).is_valid_range(
            input_range.start().value(),
            input_range.end_inclusive().value(),
        ) {
            (self.ttbr0, self.t0sz)
        } else {
            // It must have fit in TTBR1, since `input` is valid and the check forces it to be
            // contained within a single range.
            (self.ttbr1, self.t1sz)
        };

        for page in input.iter() {
            if self
                .translate(page.start_address(self.chunk_size()))
                .is_some()
            {
                return Err(MapError::FindFreeRegionError);
            }
        }

        let offset_bits = self.chunk_size().ilog2();
        let bits_per_level = self.index_bits_per_level();
        let address_space_bits = u32::from(64 - txsz);
        let target_bits = address_space_bits - offset_bits;
        let levels = self.levels(txsz);
        for (input_chunk, output_chunk) in input.iter().zip(output.iter()) {
            let input_address = input_chunk.start_address(self.chunk_size()).value();
            let output_address = output_chunk.start_address(self.chunk_size()).value();

            let mut level = 0;
            let mut base_bit = (levels - 1) * bits_per_level;
            let mut table_address = ttbr;
            loop {
                let top_bit = base_bit.saturating_add(bits_per_level).min(target_bits);
                let top_mask = if top_bit == 64 {
                    u64::MAX
                } else {
                    (1u64 << top_bit) - 1
                };
                let base_mask = (1u64 << base_bit) - 1;
                let mask = (top_mask - base_mask) << offset_bits;
                let index = ((input_address & mask) >> base_bit) >> offset_bits;

                let entry_address = table_address.strict_add(index * 8);
                let is_last_level = level == levels - 1;
                if is_last_level {
                    let descriptor = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_page(true)
                        .set_page_block_accessed(true)
                        .set_page_address(self.granule, self.output_size(), output_address);

                    if !write_u64_at(entry_address, 0) {
                        panic!("failed to write to physical memory")
                    }

                    if !write_u64_at(entry_address, descriptor.to_bits()) {
                        panic!("failed to write to physical memory")
                    }
                    break;
                }

                let entry_value =
                    read_u64_at(entry_address).expect("failed to read from physical memory");
                let entry = TranslationDescriptor::from_bits(entry_value);
                if !entry.present() {
                    let new_table = self
                        .allocate_zeroed_table()
                        .ok_or(MapError::FrameAllocation(OutOfMemory))?;
                    let descriptor = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_table(true)
                        .set_table_address(
                            self.granule,
                            self.output_size(),
                            new_table.start_address().value(),
                        );

                    if !write_u64_at(entry_address, 0) {
                        panic!("failed to write to physical memory")
                    }

                    if !write_u64_at(entry_address, descriptor.to_bits()) {
                        panic!("failed to write to physical memory")
                    }

                    table_address = new_table.start_address();
                } else if entry.table() {
                    table_address =
                        PhysicalAddress::new(entry.table_address(self.granule, self.output_size()));
                } else {
                    unimplemented!("implement block splitting")
                }

                base_bit = base_bit.saturating_sub(bits_per_level);
                level += 1;
            }
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, input: ExternalPageRange) {
        let input_range = input.address_range(self.chunk_size());

        assert!(self.input_descriptor().is_valid_range(
            input_range.start().value(),
            input_range.end_inclusive().value()
        ));

        let (ttbr, txsz) = if AddressSpaceDescriptor::new(64 - self.t0sz, false).is_valid_range(
            input_range.start().value(),
            input_range.end_inclusive().value(),
        ) {
            (self.ttbr0, self.t0sz)
        } else {
            // It must have fit in TTBR1, since `input` is valid and the check forces it to be
            // contained within a single range.
            (self.ttbr1, self.t1sz)
        };

        let offset_bits = self.chunk_size().ilog2();
        let bits_per_level = self.index_bits_per_level();
        let address_space_bits = u32::from(64 - txsz);
        let target_bits = address_space_bits - offset_bits;
        let levels = self.levels(txsz);
        for page in input.iter() {
            let input_address = page.start_address(self.chunk_size()).value();

            let mut level = 0;
            let mut base_bit = (levels - 1) * bits_per_level;
            let mut table_address = ttbr;

            loop {
                let top_bit = base_bit.saturating_add(bits_per_level).min(target_bits);
                let top_mask = if top_bit == 64 {
                    u64::MAX
                } else {
                    (1u64 << top_bit) - 1
                };
                let base_mask = (1u64 << base_bit) - 1;
                let mask = (top_mask - base_mask) << offset_bits;
                let index = ((input_address & mask) >> base_bit) >> offset_bits;

                let entry_address = table_address.strict_add(index * 8);
                let is_last_level = level == levels - 1;
                if is_last_level {
                    let descriptor = TranslationDescriptor::non_present();

                    if !write_u64_at(entry_address, descriptor.to_bits()) {
                        panic!("failed to write to physical memory")
                    }
                    break;
                }

                let entry_value =
                    read_u64_at(entry_address).expect("failed to read from physical memory");
                let entry = TranslationDescriptor::from_bits(entry_value);
                if !entry.present() {
                    break;
                } else if entry.table() {
                    table_address =
                        PhysicalAddress::new(entry.table_address(self.granule, self.output_size()));
                } else {
                    unimplemented!("implement block splitting")
                }

                base_bit = base_bit.saturating_sub(bits_per_level);
                level += 1;
            }
        }
    }

    fn translate(
        &self,
        address: ExternalVirtualAddress,
    ) -> Option<(Permissions, MappingType, ExternalPhysicalAddress)> {
        if !self.input_descriptor().is_valid(address.value()) {
            return None;
        }

        let (ttbr, txsz) = {
            let lower_range = AddressSpaceDescriptor::new(64 - self.t0sz, false);

            if self.ttbr0_enable && lower_range.is_valid(address.value()) {
                (self.ttbr0, self.t0sz)
            } else if self.ttbr1_enable {
                (self.ttbr1, self.t1sz)
            } else {
                return None;
            }
        };

        let offset_bits = self.chunk_size().ilog2();
        let bits_per_level = self.index_bits_per_level();
        let address_space_bits = u32::from(64 - txsz);
        let target_bits = address_space_bits - offset_bits;
        let levels = self.levels(txsz);

        let mut level = 0;
        let mut base_bit = (levels - 1) * bits_per_level;
        let mut table_address = ttbr;

        loop {
            let top_bit = base_bit.saturating_add(bits_per_level).min(target_bits);
            let top_mask = if top_bit == 64 {
                u64::MAX
            } else {
                (1u64 << top_bit) - 1
            };
            let base_mask = (1u64 << base_bit) - 1;
            let mask = (top_mask - base_mask) << offset_bits;
            let index = ((address.value() & mask) >> base_bit) >> offset_bits;

            let entry_address = table_address.strict_add(index * 8);
            let entry_value = read_u64_at(entry_address)?;
            let entry = TranslationDescriptor::from_bits(entry_value);
            if !entry.present() {
                return None;
            }

            let is_last_level = level == levels - 1;
            if is_last_level {
                if !entry.page() {
                    return None;
                }

                let output_base = entry.page_address(self.granule, self.output_size());
                let page_offset_mask = u64::from(self.granule.size()) - 1;
                let page_offset = address.value() & page_offset_mask;

                // TODO: Fix permissions and mapping type computation.
                let permissions = Permissions::Read;
                let mapping_type = MappingType::Normal;
                let physical_address = ExternalPhysicalAddress::new(output_base | page_offset);
                return Some((permissions, mapping_type, physical_address));
            } else if entry.table() {
                table_address =
                    PhysicalAddress::new(entry.table_address(self.granule, self.output_size()));
            } else {
                let output_base = entry.block_address(self.granule, self.output_size());

                let page_offset_mask = (1u64 << (base_bit + offset_bits)) - 1;
                let page_offset = address.value() & page_offset_mask;

                // TODO: Fix permissions and mapping type computation.
                let permissions = Permissions::Read;
                let mapping_type = MappingType::Normal;
                let physical_address = ExternalPhysicalAddress::new(output_base | page_offset);
                return Some((permissions, mapping_type, physical_address));
            }

            base_bit = base_bit.saturating_sub(bits_per_level);
            level += 1;
        }
    }
}

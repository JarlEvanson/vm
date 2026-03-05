//! Implementation of [`TranslationScheme`] for `VMSAv8` address translation.

use core::{fmt, mem};

use conversion::usize_to_u64;
use memory::{
    address::{Address, AddressChunkRange, AddressSpaceDescriptor, Frame, PhysicalAddress},
    phys::PhysicalMemorySpace,
    translation::{MapError, MapFlags, TranslationScheme},
};

use crate::{
    common::{EL, Granule, PhysicalAddressSpaceSize},
    msr::{
        Aarch64MemoryModelFeatureRegister0EL1, CurrentEl, TcrEL1,
        raw::{read_ttbr0_el1, read_ttbr1_el1},
    },
    paging::{AddressSize, AllocPhysical, DeallocPhysical, raw::vmsa_v8::TranslationDescriptor},
};

/// Implementation of [`TranslationScheme`] for `VMSAv8` address translation.
#[derive(Debug)]
pub struct VmsaV8Scheme<M: PhysicalMemorySpace> {
    /// The [`Granule`] used for this [`TranslationScheme`].
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

    /// The [`PhysicalMemorySpace`] to which the [`VmsaV8Scheme`] controls access.
    memory: M,

    /// Function to allocate a provided number of physically-contiguous bytes with a provided
    /// alignment.
    alloc_physical: AllocPhysical,

    /// Function to deallocate a range of physically-contiguous bytes
    dealloc_physical: DeallocPhysical,
}

impl<M: PhysicalMemorySpace> VmsaV8Scheme<M> {
    /*
    pub fn new(
        granule: Granule,
        ttbr0: Option<u8>,
        ttbr1: Option<u8>,
        output: PhysicalAddressSpaceSize,
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, VmsaV8Error> {
        let (ttbr0_enable, t0sz) = if let Some(t0sz) = ttbr0 {
            (true, t0sz)
        } else {
            (false, 64)
        };

        let (ttbr1_enable, t1sz) = if let Some(t1sz) = ttbr1 {
            (true, t1sz)
        } else {
            (false, 64)
        };

        let mut scheme = Self {
            granule,

            ttbr0_enable,
            ttbr1_enable,

            ttbr0: PhysicalAddress::zero(),
            ttbr1: PhysicalAddress::zero(),

            t0sz,
            t1sz,

            output,

            memory,
            alloc_physical,
            dealloc_physical,
        };

        if scheme.ttbr0_enable {
            scheme.ttbr0 = scheme
                .allocate_zeroed_table()
                .ok_or(VmsaV8Error::OutOfMemory)?
                .start_address(scheme.chunk_size());
        }

        if scheme.ttbr1_enable {
            scheme.ttbr1 = scheme
                .allocate_zeroed_table()
                .ok_or(VmsaV8Error::OutOfMemory)?
                .start_address(scheme.chunk_size());
        }

        Ok(scheme)
    }
    */

    /// Creates a new [`VmsaV8Scheme`] with the maximum supported settings.
    ///
    /// # Errors
    ///
    /// - [`VmsaV8Error::OutOfMemory`]: Returned if the allocation of the root frames failed.
    pub fn max_supported(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, VmsaV8Error> {
        let el = CurrentEl::get().el();
        let mut scheme = if el == EL::EL1 {
            // SAFETY:
            //
            // Since the program is in [`EL::EL1`], it is safe to read
            // [`Aarch64MemoryModelFeatureRegister0EL1`].
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

                memory,
                alloc_physical,
                dealloc_physical,
            }
        } else if el == EL::EL2 {
            todo!("implement EL2")
        } else {
            unreachable!()
        };

        if scheme.ttbr0_enable {
            scheme.ttbr0 = scheme
                .allocate_zeroed_table()
                .ok_or(VmsaV8Error::OutOfMemory)?
                .start_address(scheme.chunk_size());
        }

        if scheme.ttbr1_enable {
            scheme.ttbr1 = scheme
                .allocate_zeroed_table()
                .ok_or(VmsaV8Error::OutOfMemory)?
                .start_address(scheme.chunk_size());
        }

        Ok(scheme)
    }

    /// Creates a new [`VmsaV8Scheme`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Errors
    ///
    /// - [`VmsaV8Error::OutOfMemory`]: Never returned from this function.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`VmsaV8Scheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, VmsaV8Error> {
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

                memory,
                alloc_physical,
                dealloc_physical,
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

        Ok(scheme)
    }

    /// Returns the size, in bytes, of minimal translation region.
    pub fn granule(&self) -> Granule {
        self.granule
    }

    /// Returns `true` if the `TTBR0` page tables are in use.
    pub fn ttbr0_enabled(&self) -> bool {
        self.ttbr0_enable
    }

    /// Returns `true` if the `TTBR1` page tables are in use.
    pub fn ttbr1_enabled(&self) -> bool {
        self.ttbr1_enable
    }

    /// Returns the location of the table referenced by `TTBR0`.
    pub fn ttbr0(&self) -> u64 {
        self.ttbr0.value()
    }

    /// Returns the location of the table referenced by `TTBR1`.
    pub fn ttbr1(&self) -> u64 {
        self.ttbr1.value()
    }

    /// Returns the value of `T0SZ`.
    pub fn t0sz(&self) -> u8 {
        self.t0sz
    }

    /// Returns the value of `T1SZ`.
    pub fn t1sz(&self) -> u8 {
        self.t1sz
    }

    /// Returns the size, in bits, of the output address.
    pub fn ipa(&self) -> PhysicalAddressSpaceSize {
        self.output
    }

    fn entries_per_table(&self) -> u32 {
        self.granule.size() / 8
    }

    fn index_bits_per_level(&self) -> u32 {
        self.entries_per_table().ilog2()
    }

    fn levels(&self, txsz: u8) -> u32 {
        (u32::from(64 - txsz) - self.granule.size().ilog2()).div_ceil(self.index_bits_per_level())
    }

    fn output_size(&self) -> AddressSize {
        match self.output {
            PhysicalAddressSpaceSize::Bits32
            | PhysicalAddressSpaceSize::Bits36
            | PhysicalAddressSpaceSize::Bits40
            | PhysicalAddressSpaceSize::Bits42
            | PhysicalAddressSpaceSize::Bits44
            | PhysicalAddressSpaceSize::Bits48 => AddressSize::Bits48,
            PhysicalAddressSpaceSize::Bits52 => AddressSize::Bits52,
            PhysicalAddressSpaceSize::Bits56 => unimplemented!("vmsa_v9"),
        }
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<Frame> {
        let range = (self.alloc_physical)(self.chunk_size(), self.chunk_size())?;

        for i in 0..self.chunk_size() / usize_to_u64(mem::size_of::<u64>()) {
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u64_le(range.start().strict_add(i * 8), 0)
                    .ok()?;
            }
        }

        Some(Frame::containing_address(range.start(), self.chunk_size()))
    }
}

// SAFETY:
//
// `VmsaV8Scheme` is implemented in accordance with the `aarch64` specification.
unsafe impl<M: PhysicalMemorySpace> TranslationScheme for VmsaV8Scheme<M> {
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

    unsafe fn map(
        &mut self,
        input: AddressChunkRange,
        output: AddressChunkRange,
        flags: MapFlags,
    ) -> Result<(), MapError> {
        if !input.is_valid(self.chunk_size(), &self.input_descriptor()) {
            return Err(MapError::InvalidRange);
        }

        if !output.is_valid(self.chunk_size(), &self.output_descriptor()) {
            return Err(MapError::InvalidRange);
        }

        if input.count() != output.count() {
            return Err(MapError::MappingMismatch);
        }

        let (ttbr, txsz) = if input.is_valid(
            self.chunk_size(),
            &AddressSpaceDescriptor::new(64 - self.t0sz, false),
        ) {
            (self.ttbr0, self.t0sz)
        } else {
            // It must have fit in TTBR1, since `input` is valid and the check forces it to be
            // contained within a single range.
            (self.ttbr1, self.t1sz)
        };

        if !flags.contains(MapFlags::MAY_OVERWRITE) {
            for input_chunk in input.iter() {
                if self
                    .translate_input(input_chunk.start_address(self.chunk_size()))
                    .is_some()
                {
                    return Err(MapError::OverlapError);
                }
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

                    // SAFETY:
                    //
                    // This page table layout is valid.
                    unsafe {
                        self.memory
                            .write_u64_le(entry_address, descriptor.to_bits())
                            .expect("failed to write to physical memory")
                    };
                    break;
                }

                // SAFETY:
                //
                // This page table layout is valid.
                let entry_value = unsafe {
                    self.memory
                        .read_u64_le(entry_address)
                        .expect("failed to read from physical memory")
                };
                let entry = TranslationDescriptor::from_bits(entry_value);
                if !entry.present() {
                    let new_table = self.allocate_zeroed_table().ok_or(MapError::OutOfMemory)?;
                    let descriptor = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_table(true)
                        .set_table_address(
                            self.granule,
                            self.output_size(),
                            new_table.start_address(self.chunk_size()).value(),
                        );

                    // SAFETY:
                    //
                    // This page table layout is valid.
                    unsafe {
                        self.memory
                            .write_u64_le(entry_address, descriptor.to_bits())
                            .expect("failed to write to physical memory")
                    };

                    table_address = new_table.start_address(self.chunk_size());
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

    unsafe fn unmap(&mut self, input: AddressChunkRange) {
        assert!(input.is_valid(self.chunk_size(), &self.input_descriptor()));
        assert!(!input.is_empty());

        let (ttbr, txsz) = if input.is_valid(
            self.chunk_size(),
            &AddressSpaceDescriptor::new(64 - self.t0sz, false),
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
        for input_chunk in input.iter() {
            let input_address = input_chunk.start_address(self.chunk_size()).value();

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
                    let descriptor = TranslationDescriptor::non_present().set_present(false);

                    // SAFETY:
                    //
                    // This page table layout is valid.
                    unsafe {
                        self.memory
                            .write_u64_le(entry_address, descriptor.to_bits())
                            .expect("failed to write to physical memory")
                    };
                    break;
                }

                // SAFETY:
                //
                // This page table layout is valid.
                let entry_value = unsafe {
                    self.memory
                        .read_u64_le(entry_address)
                        .expect("failed to read from physical memory")
                };
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

    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)> {
        if !input.is_valid(&self.input_descriptor()) {
            return None;
        }

        let (ttbr, txsz) = {
            let lower_range = AddressSpaceDescriptor::new(64 - self.t0sz, false);

            if self.ttbr0_enable && input.is_valid(&lower_range) {
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
            let index = ((input.value() & mask) >> base_bit) >> offset_bits;

            let entry_address = table_address.strict_add(index * 8);
            let entry_value = unsafe { self.memory.read_u64_le(entry_address).ok()? };
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
                let page_offset = input.value() & page_offset_mask;

                let physical_address = Address::new(output_base | page_offset);
                let flags = MapFlags::READ;
                return Some((physical_address, flags));
            } else if entry.table() {
                table_address =
                    PhysicalAddress::new(entry.table_address(self.granule, self.output_size()));
            } else {
                let output_base = entry.block_address(self.granule, self.output_size());

                let page_offset_mask = (1u64 << (base_bit + offset_bits)) - 1;
                let page_offset = input.value() & page_offset_mask;

                let physical = Address::new(output_base | page_offset);

                let flags = MapFlags::READ;
                return Some((physical, flags));
            }

            base_bit = base_bit.saturating_sub(bits_per_level);
            level += 1;
        }
    }
}

/// Various errors that can occur while creating an instance of [`VmsaV8Scheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmsaV8Error {
    /// The allocation of root frames failed.
    OutOfMemory,
}

impl fmt::Display for VmsaV8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating root frames"),
        }
    }
}

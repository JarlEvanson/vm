//! Implementation of 32-bit paging.

use core::mem;

use conversion::usize_to_u64;
use memory::AddressSpaceDescriptor;
use x86::{
    control::{Cr3, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    paging::{
        PagingMode, bits_32::TranslationDescriptor, current_paging_mode, max_supported_paging_mode,
    },
};

use crate::{
    arch::{
        generic::memory::paging::{
            ExternalFrameRange, ExternalPageRange, ExternalPhysicalAddress, ExternalVirtualAddress,
            TranslationScheme,
        },
        x86::memory::physical_bits,
    },
    platform::{
        AllocationPolicy, FrameRange, MapError, MappingType, OutOfMemory, Permissions,
        PhysicalAddress, allocate_physical, read_u32_at, write_u32_at, write_u64_at,
    },
};

/// Implementation of [`TranslationScheme`] for 32-bit paging.
pub struct Bits32TranslationScheme {
    /// Physical address of the page directory.
    physical_address: PhysicalAddress,

    /// Whether 4 MiB pages (PSE) are enabled.
    pse: bool,

    /// Whether 4 MiB pages can access beyond 32-bits.
    _pse36: bool,

    /// The maximum number of physical address bits supported.
    max_bits: u8,
}

impl Bits32TranslationScheme {
    /// Creates a new [`Bits32TranslationScheme`] with the maximum supported settings.
    pub fn max_supported() -> Option<Self> {
        match max_supported_paging_mode() {
            PagingMode::Disabled => return None,
            PagingMode::Bits32 | PagingMode::Pae | PagingMode::Level4 | PagingMode::Level5 => {}
        }

        let (pse, pse36, max_bits) = if supports_cpuid() {
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let pse = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 3)) != 0 };
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let pse36 = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 17)) != 0 };

            (pse, pse36, physical_bits().min(40))
        } else {
            (false, false, 32)
        };

        let mut scheme = Self {
            physical_address: PhysicalAddress::zero(),
            pse,
            _pse36: pse36,
            max_bits,
        };

        scheme.physical_address = scheme.allocate_zeroed_table()?.start_address();
        Some(scheme)
    }

    /// Creates a new [`Bits32TranslationScheme`] by taking over the current page tables referenced
    /// by `CR3`.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`Bits32TranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        if current_paging_mode() != PagingMode::Bits32 {
            return None;
        }

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR4` is safe.
        let cr4 = unsafe { Cr4::get() };
        let pse = cr4.pse();

        let (pse36, max_bits) = if supports_cpuid() {
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let pse36 = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 17)) != 0 && pse };
            (pse36, physical_bits().min(40))
        } else {
            (false, 32)
        };

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR3` is safe.
        let physical_address = unsafe { PhysicalAddress::new(Cr3::get().to_bits() & 0xFFFF_F000) };

        Some(Self {
            physical_address,
            pse,
            _pse36: pse36,
            max_bits,
        })
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`Bits32TranslationScheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Returns `true` if the page tables should be evaluated as if the `PSE` bit is set.
    pub fn pse(&self) -> bool {
        self.pse
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<FrameRange> {
        let range = allocate_physical(
            self.chunk_size(),
            self.chunk_size(),
            AllocationPolicy::InclusiveMax((1u64 << 32) - 1),
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

impl TranslationScheme for Bits32TranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(32, false)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(self.max_bits, false)
    }

    fn chunk_size(&self) -> u64 {
        4096
    }

    fn map_at(
        &mut self,
        input: ExternalPageRange,
        output: ExternalFrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<(), MapError> {
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

        // Validate that the entire required mapping region is unmapped.
        for page in input.iter() {
            if self
                .translate(page.start_address(self.chunk_size()))
                .is_some()
            {
                return Err(MapError::FindFreeRegionError);
            }
        }

        for (input_chunk, output_chunk) in input.iter().zip(output.iter()) {
            let address = input_chunk.start_address(self.chunk_size()).value();

            let pd_index = (address >> 22) & 0x3FF;
            let pt_index = (address >> 12) & 0x3FF;

            let pt_table_address = {
                let pde_address = self.physical_address.strict_add(pd_index * 4);
                let pde_value = read_u32_at(pde_address).expect("failed to read PML2E");
                let mut pde = TranslationDescriptor::from_bits(pde_value);

                if !pde.present() {
                    let frame = self
                        .allocate_zeroed_table()
                        .ok_or(MapError::FrameAllocation(OutOfMemory))?;
                    let start_address = u32::try_from(frame.start_address().value())
                        .expect("physical memory allocation failed");

                    pde = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(start_address);

                    if !write_u32_at(pde_address, pde.to_bits()) {
                        panic!("failed to write to physical memory");
                    }
                }

                PhysicalAddress::new(u64::from(pde.table_address()))
            };

            let writable = permissions.writable();

            let pte_address = pt_table_address.strict_add(pt_index * 4);

            let start_address =
                u32::try_from(output_chunk.start_address(self.chunk_size()).value())
                    .expect("physical memory allocation failed");

            let pte = TranslationDescriptor::non_present()
                .set_present(true)
                .set_writable(writable)
                .set_page_address(start_address);

            if !write_u32_at(pte_address, pte.to_bits()) {
                panic!("failed to write to physical memory");
            }
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, input: ExternalPageRange) {
        assert!(self.input_descriptor().is_valid_range(
            input.start_address(self.chunk_size()).value(),
            input.end_address_inclusive(self.chunk_size()).value()
        ));

        for page in input.iter() {
            let address = page.start_address(self.chunk_size()).value();

            let pd_index = (address >> 22) & 0x3FF;
            let pt_index = (address >> 12) & 0x3FF;

            let pde_address = self.physical_address.strict_add(pd_index * 4);
            let pde_value = read_u32_at(pde_address).expect("failed to read from physical memory");
            let pde = TranslationDescriptor::from_bits(pde_value);

            if !pde.present() {
                continue;
            }

            let pt_table_address = PhysicalAddress::new(u64::from(pde.table_address()));

            let pte_address = pt_table_address.strict_add(pt_index * 4);
            if !write_u32_at(pte_address, TranslationDescriptor::non_present().to_bits()) {
                panic!("failed to write to physical memory");
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

        let value = address.value();
        let pd_index = (value >> 22) & 0x3FF;
        let pt_index = (value >> 12) & 0x3FF;

        let mut writable = true;

        let pde_address = self.physical_address.checked_add(pd_index * 4)?;
        let pde_value = read_u32_at(pde_address)?;
        let pde = TranslationDescriptor::from_bits(pde_value);

        if !pde.present() {
            return None;
        }

        writable &= pde.writable();

        if self.pse && pde.block() {
            let offset = value % (1024 * self.chunk_size());

            let permissions = if writable {
                Permissions::ReadWriteExecute
            } else {
                Permissions::ReadExecute
            };
            // TODO: Fix mapping type computation.
            let mapping_type = MappingType::Normal;
            let address = ExternalPhysicalAddress::new(pde.block_address()).strict_add(offset);

            return Some((permissions, mapping_type, address));
        }

        let pt_table_address = PhysicalAddress::new(u64::from(pde.table_address()));

        let pte_address = pt_table_address.checked_add(pt_index * 4)?;
        let pte_value = read_u32_at(pte_address)?;
        let pte = TranslationDescriptor::from_bits(pte_value);

        if !pte.present() {
            return None;
        }

        writable &= pte.writable();

        let offset = value & 0xFFF;

        let permissions = if writable {
            Permissions::ReadWriteExecute
        } else {
            Permissions::ReadExecute
        };
        // TODO: Fix mapping type computation.
        let mapping_type = MappingType::Normal;
        let address =
            ExternalPhysicalAddress::new(u64::from(pte.page_address())).strict_add(offset);

        Some((permissions, mapping_type, address))
    }
}

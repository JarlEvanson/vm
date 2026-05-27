//! Implementation of `PAE` paging.

use core::mem;

use conversion::usize_to_u64;
use memory::AddressSpaceDescriptor;
use x86::{
    control::Cr3,
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr},
    paging::{
        PagingMode, current_paging_mode, max_supported_paging_mode,
        pae::{PdpteDescriptor, TranslationDescriptor},
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
        PhysicalAddress, allocate_physical, read_u64_at, write_u64_at,
    },
};

/// Implementation of [`TranslationScheme`] for PAE paging.
pub struct PaeTranslationScheme {
    /// The [`PhysicalAddress`] at which the PDPTE is located.
    ///
    /// This is a 32-byte table.
    physical_address: PhysicalAddress,
    /// If `true`, the `NXE` bit should be treated as being set.
    nxe: bool,
}

impl PaeTranslationScheme {
    /// Creates a new [`PaeTranslationScheme`] with the maximum supported settings.
    pub fn max_supported() -> Option<Self> {
        match max_supported_paging_mode() {
            PagingMode::Disabled | PagingMode::Bits32 => return None,
            PagingMode::Pae | PagingMode::Level4 | PagingMode::Level5 => {}
        }

        let nxe = if supports_cpuid() {
            // SAFETY:
            //
            // The CPUID instruction is supported.
            unsafe { (cpuid_unchecked(0x8000_0001, 0).edx & (1 << 20)) != 0 }
        } else {
            false
        };

        let mut scheme = Self {
            physical_address: PhysicalAddress::zero(),
            nxe,
        };

        scheme.physical_address = scheme.allocate_zeroed_table()?.start_address();
        Some(scheme)
    }

    /// Creates a new [`PaeTranslationScheme`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`PaeTranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        let paging_mode = current_paging_mode();
        if paging_mode != PagingMode::Pae {
            return None;
        }

        let nxe = if supports_msr() {
            // SAFETY:
            //
            // The MSR instructions are supported.
            unsafe { (read_msr(0xC000_0080) & (1 << 11)) == (1 << 11) }
        } else {
            false
        };

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR3` is safe.
        let physical_address = unsafe { PhysicalAddress::new(Cr3::get().to_bits() & 0xFFFF_FFE0) };
        Some(Self {
            physical_address,
            nxe,
        })
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`PaeTranslationScheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Returns `true` if the page tables should be evaulated as if the `NXE` bit is set.
    pub fn nxe(&self) -> bool {
        self.nxe
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

impl TranslationScheme for PaeTranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(32, false)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(physical_bits(), false)
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

            let pdpte_index = (address >> 30) & 0b11;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml2_table_address = {
                let pdpte_address = self.physical_address.strict_add(pdpte_index * 8);
                let pdpte_value = read_u64_at(pdpte_address).expect("failed to read PDPTE");
                let mut pdpte = PdpteDescriptor::from_bits(pdpte_value);
                if !pdpte.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::FrameAllocation(OutOfMemory));
                    };

                    pdpte = PdpteDescriptor::non_present()
                        .set_present(true)
                        .set_address(frame.start_address().value());

                    if !write_u64_at(pdpte_address, pdpte.to_bits()) {
                        panic!("failed to write to physical memory")
                    }
                }

                PhysicalAddress::new(pdpte.address())
            };

            let pml1_table_address = {
                let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
                let pml2e_value = read_u64_at(pml2e_address).expect("failed to read PML2E");
                let mut pml2e = TranslationDescriptor::from_bits(pml2e_value);
                if !pml2e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::FrameAllocation(OutOfMemory));
                    };

                    pml2e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address().value());

                    if !write_u64_at(pml2e_address, pml2e.to_bits()) {
                        panic!("failed to write to physical memory")
                    }
                } else if pml2e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml2e.table_address())
            };

            let writable = permissions.writable();
            let xd = self.nxe && !permissions.executable();

            let pml1e_address = pml1_table_address.strict_add(pml1e_index * 8);
            let pml1e = TranslationDescriptor::non_present()
                .set_present(true)
                .set_writable(writable)
                .set_page_address(output_chunk.start_address(self.chunk_size()).value())
                .set_xd(xd);

            if !write_u64_at(pml1e_address, pml1e.to_bits()) {
                panic!("failed to write to physical memory")
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

            let pdpte_index = (address >> 30) & 0b11;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml2_table_address = {
                let pdpte_address = self.physical_address.strict_add(pdpte_index * 8);
                let pdpte_value = read_u64_at(pdpte_address).expect("failed to read PDPTE");
                let pdpte = PdpteDescriptor::from_bits(pdpte_value);
                if !pdpte.present() {
                    continue;
                }

                PhysicalAddress::new(pdpte.address())
            };

            let pml1_table_address = {
                let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
                let pml2e_value = read_u64_at(pml2e_address).expect("failed to read PDPTE");
                let pml2e = TranslationDescriptor::from_bits(pml2e_value);
                if !pml2e.present() {
                    continue;
                } else if pml2e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml2e.table_address())
            };

            let pml1e_address = pml1_table_address.strict_add(pml1e_index * 8);
            let pml1e = TranslationDescriptor::non_present();

            if !write_u64_at(pml1e_address, pml1e.to_bits()) {
                panic!("failed to write to physical memory")
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

        let pdpte_index = (address.value() >> 30) & 0b11;
        let pml2e_index = (address.value() >> 21) & 0x1FF;
        let pml1e_index = (address.value() >> 12) & 0x1FF;

        let mut writable = true;
        let mut executable = true;
        let pml2_table_address = {
            let pdpte_address = self.physical_address.checked_add(pdpte_index * 8)?;
            let pdpte_value = read_u64_at(pdpte_address)?;
            let pdpte = PdpteDescriptor::from_bits(pdpte_value);
            if !pdpte.present() {
                return None;
            }

            PhysicalAddress::new(pdpte.address())
        };

        let pml1_table_address = {
            let pml2e_address = pml2_table_address.checked_add(pml2e_index * 8)?;
            let pml2e_value = read_u64_at(pml2e_address)?;
            let pml2e = TranslationDescriptor::from_bits(pml2e_value);
            if !pml2e.present() {
                return None;
            }

            writable = writable && pml2e.writable();
            executable = executable && (self.nxe && !pml2e.xd());

            if pml2e.block() {
                let offset = address.value() % (512 * self.chunk_size());

                let permissions = match (writable, executable) {
                    (true, true) => Permissions::ReadWriteExecute,
                    (true, false) => Permissions::ReadWrite,
                    (false, true) => Permissions::ReadExecute,
                    (false, false) => Permissions::Read,
                };
                // TODO: Fix mapping type computation.
                let mapping_type = MappingType::Normal;
                let address =
                    ExternalPhysicalAddress::new(pml2e.block_address()).strict_add(offset);

                return Some((permissions, mapping_type, address));
            }

            if pml2e.page_or_table_reserved() != 0 {
                return None;
            }
            PhysicalAddress::new(pml2e.table_address())
        };

        let pml1e_address = pml1_table_address.checked_add(pml1e_index * 8)?;
        let pml1e_value = read_u64_at(pml1e_address)?;
        let pml1e = TranslationDescriptor::from_bits(pml1e_value);
        if !pml1e.present() {
            return None;
        }
        writable = writable && pml1e.writable();
        executable = executable && (self.nxe && !pml1e.xd());

        let offset = address.value() & 0xFFF;

        let permissions = match (writable, executable) {
            (true, true) => Permissions::ReadWriteExecute,
            (true, false) => Permissions::ReadWrite,
            (false, true) => Permissions::ReadExecute,
            (false, false) => Permissions::Read,
        };
        // TODO: Fix mapping type computation.
        let mapping_type = MappingType::Normal;
        let address = ExternalPhysicalAddress::new(pml1e.page_address()).strict_add(offset);

        Some((permissions, mapping_type, address))
    }
}

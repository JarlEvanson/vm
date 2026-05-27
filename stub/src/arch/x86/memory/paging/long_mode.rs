//! Implementation of long mode paging.

use core::mem;

use conversion::usize_to_u64;
use memory::AddressSpaceDescriptor;
use x86::{
    control::Cr3,
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr},
    paging::{
        PagingMode, bits_64::TranslationDescriptor, current_paging_mode, max_supported_paging_mode,
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

/// Implementation of [`TranslationScheme`] for long mode paging.
pub struct LongModeTranslationScheme {
    /// The physical address of the top of the page table.
    physical_address: PhysicalAddress,
    /// If `true`, the `LA57` bit should be treated as being set.
    la57: bool,
    /// If `true`, the `NXE` bit should be treated as being set.
    nxe: bool,
}

impl LongModeTranslationScheme {
    /// Creates a new [`LongModeTranslationScheme`] with the maximum supported settings.
    pub fn max_supported() -> Option<Self> {
        let paging_mode = max_supported_paging_mode();
        if paging_mode != PagingMode::Level4 && paging_mode != PagingMode::Level5 {
            return None;
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
            la57: paging_mode == PagingMode::Level5,
            nxe,
        };

        scheme.physical_address = scheme.allocate_zeroed_table()?.start_address();
        Some(scheme)
    }

    /// Creates a new [`LongModeTranslationScheme`] by taking over the current page tables
    /// referenced by `CR3`.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`LongModeTranslationScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current() -> Option<Self> {
        let paging_mode = current_paging_mode();
        if paging_mode != PagingMode::Level4 && paging_mode != PagingMode::Level5 {
            return None;
        }

        let la57 = paging_mode != PagingMode::Level4;
        let nxe = if supports_msr() {
            // SAFETY:
            //
            // The MSR instructions are supported.
            unsafe { (read_msr(0xC000_0080) & (1 << 11)) != 0 }
        } else {
            false
        };

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR3` is safe.
        let physical_address =
            unsafe { PhysicalAddress::new(Cr3::get().to_bits() & 0x000F_FFFF_FFFF_F000) };

        Some(Self {
            physical_address,
            la57,
            nxe,
        })
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`LongModeTranslationScheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Returns `true` if the page tables should be evaluated as if the `LA57` bit is set.
    pub fn la57(&self) -> bool {
        self.la57
    }

    /// Returns `true` if the page tables should be evaluated as if the `NXE` bit is set.
    pub fn nxe(&self) -> bool {
        self.nxe
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<FrameRange> {
        let max_physical = 1u64
            .checked_shl(u32::from(physical_bits()))
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

impl TranslationScheme for LongModeTranslationScheme {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(if self.la57 { 57 } else { 48 }, true)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(52, false)
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

            let pml5e_index = (address >> 48) & 0x1FF;
            let pml4e_index = (address >> 39) & 0x1FF;
            let pml3e_index = (address >> 30) & 0x1FF;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml4_table_address = if self.la57 {
                let pml5e_address = self.physical_address.strict_add(pml5e_index * 8);
                let pml5e_value = read_u64_at(pml5e_address).expect("failed to read PML5E");
                let mut pml5e = TranslationDescriptor::from_bits(pml5e_value);
                if !pml5e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::FrameAllocation(OutOfMemory));
                    };

                    pml5e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address().value());

                    if !write_u64_at(pml5e_address, pml5e.to_bits()) {
                        panic!("failed to write PML5E")
                    }
                }

                PhysicalAddress::new(pml5e.table_address())
            } else {
                self.physical_address
            };

            let pml3_table_address = {
                let pml4e_address = pml4_table_address.strict_add(pml4e_index * 8);
                let pml4e_value = read_u64_at(pml4e_address).expect("failed to read PML4E");
                let mut pml4e = TranslationDescriptor::from_bits(pml4e_value);
                if !pml4e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::FrameAllocation(OutOfMemory));
                    };

                    pml4e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address().value());

                    if !write_u64_at(pml4e_address, pml4e.to_bits()) {
                        panic!("failed to write PML4E")
                    }
                }

                PhysicalAddress::new(pml4e.table_address())
            };

            let pml2_table_address = {
                let pml3e_address = pml3_table_address.strict_add(pml3e_index * 8);
                let pml3e_value = read_u64_at(pml3e_address).expect("failed to read PML3E");
                let mut pml3e = TranslationDescriptor::from_bits(pml3e_value);
                if !pml3e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::FrameAllocation(OutOfMemory));
                    };

                    pml3e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address().value());

                    if !write_u64_at(pml3e_address, pml3e.to_bits()) {
                        panic!("failed to write PML3E")
                    }
                } else if pml3e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml3e.table_address())
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
                        panic!("failed to write PML2E")
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
                panic!("failed to write PML1E")
            }
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, input: ExternalPageRange) {
        assert!(self.input_descriptor().is_valid_range(
            input.start_address(self.chunk_size()).value(),
            input.end_address_inclusive(self.chunk_size()).value()
        ));

        for chunk in input.iter() {
            let address = chunk.start_address(self.chunk_size()).value();

            let pml5e_index = (address >> 48) & 0x1FF;
            let pml4e_index = (address >> 39) & 0x1FF;
            let pml3e_index = (address >> 30) & 0x1FF;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml4_table_address = if self.la57 {
                let pml5e_address = self.physical_address.strict_add(pml5e_index * 8);
                let pml5e_value = read_u64_at(pml5e_address).expect("failed to read PML5E");
                let pml5e = TranslationDescriptor::from_bits(pml5e_value);
                if !pml5e.present() {
                    continue;
                }

                PhysicalAddress::new(pml5e.table_address())
            } else {
                self.physical_address
            };

            let pml3_table_address = {
                let pml4e_address = pml4_table_address.strict_add(pml4e_index * 8);
                let pml4e_value = read_u64_at(pml4e_address).expect("failed to read PML4E");
                let pml4e = TranslationDescriptor::from_bits(pml4e_value);
                if !pml4e.present() {
                    continue;
                }

                PhysicalAddress::new(pml4e.table_address())
            };

            let pml2_table_address = {
                let pml3e_address = pml3_table_address.strict_add(pml3e_index * 8);
                let pml3e_value = read_u64_at(pml3e_address).expect("failed to read PML3E");
                let pml3e = TranslationDescriptor::from_bits(pml3e_value);
                if !pml3e.present() {
                    continue;
                } else if pml3e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml3e.table_address())
            };

            let pml1_table_address = {
                let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
                let pml2e_value = read_u64_at(pml2e_address).expect("failed to read PML2E");
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
                panic!("failed to write PML1E")
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

        let pml5e_index = (address.value() >> 48) & 0x1FF;
        let pml4e_index = (address.value() >> 39) & 0x1FF;
        let pml3e_index = (address.value() >> 30) & 0x1FF;
        let pml2e_index = (address.value() >> 21) & 0x1FF;
        let pml1e_index = (address.value() >> 12) & 0x1FF;

        let mut writable = true;
        let mut executable = true;
        let pml4_table_address = if self.la57 {
            let pml5e_address = self.physical_address.strict_add(pml5e_index * 8);
            let pml5e_value = read_u64_at(pml5e_address).expect("failed to read PML5E");
            let pml5e = TranslationDescriptor::from_bits(pml5e_value);
            if !pml5e.present() {
                return None;
            }

            writable = writable && pml5e.writable();
            executable = executable && (self.nxe && !pml5e.xd());
            PhysicalAddress::new(pml5e.table_address())
        } else {
            self.physical_address
        };

        let pml3_table_address = {
            let pml4e_address = pml4_table_address.strict_add(pml4e_index * 8);
            let pml4e_value = read_u64_at(pml4e_address).expect("failed to read PML4E");
            let pml4e = TranslationDescriptor::from_bits(pml4e_value);
            if !pml4e.present() {
                return None;
            }

            writable = writable && pml4e.writable();
            executable = executable && (self.nxe && !pml4e.xd());
            PhysicalAddress::new(pml4e.table_address())
        };

        let pml2_table_address = {
            let pml3e_address = pml3_table_address.strict_add(pml3e_index * 8);
            let pml3e_value = read_u64_at(pml3e_address).expect("failed to read PML3E");
            let pml3e = TranslationDescriptor::from_bits(pml3e_value);
            if !pml3e.present() {
                return None;
            }

            writable = writable && pml3e.writable();
            executable = executable && (self.nxe && !pml3e.xd());
            if pml3e.block() {
                let offset = address.value() % (512 * 512 * self.chunk_size());

                let permissions = match (writable, executable) {
                    (true, true) => Permissions::ReadWriteExecute,
                    (true, false) => Permissions::ReadWrite,
                    (false, true) => Permissions::ReadExecute,
                    (false, false) => Permissions::Read,
                };
                // TODO: Fix mapping type computation.
                let mapping_type = MappingType::Normal;
                let address =
                    ExternalPhysicalAddress::new(pml3e.block_pml3_address()).strict_add(offset);

                return Some((permissions, mapping_type, address));
            }

            PhysicalAddress::new(pml3e.table_address())
        };

        let pml1_table_address = {
            let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
            let pml2e_value = read_u64_at(pml2e_address).expect("failed to read PML2E");
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
                    ExternalPhysicalAddress::new(pml2e.block_pml2_address()).strict_add(offset);

                return Some((permissions, mapping_type, address));
            }

            PhysicalAddress::new(pml2e.table_address())
        };

        let pml1e_address = pml1_table_address.checked_add(pml1e_index * 8)?;
        let pml1e_value = read_u64_at(pml1e_address).expect("failed to read PML1E");
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

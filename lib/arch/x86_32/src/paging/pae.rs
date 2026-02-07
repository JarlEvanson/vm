//! Implementation of [`TranslationScheme`] for PAE paging.

use core::{error, fmt, mem};

use conversion::usize_to_u64;
use memory::{
    address::{
        Address, AddressChunkRange, AddressSpaceDescriptor, Frame, PhysicalAddress,
        PhysicalAddressRange,
    },
    phys::PhysicalMemorySpace,
    translation::{MapError, MapFlags, TranslationScheme},
};
use x86_common::{
    control::Cr3,
    cpuid::{cpuid_unchecked, supports_cpuid},
    msr::{read_msr, supports_msr},
    paging::{PagingMode, current_paging_mode, max_supported_paging_mode},
};

use crate::paging::{
    AllocPhysical, DeallocPhysical,
    raw::pae::{PdpteDescriptor, TranslationDescriptor},
};

/// Implmentation of [`TranslationScheme`] for PAE paging.
pub struct PaeScheme<M: PhysicalMemorySpace> {
    /// The [`PhysicalAddress`] at which the PDPTE is located.
    ///
    /// This is a 32-byte table.
    physical_address: PhysicalAddress,
    /// If `true`, the `NXE` bit should be treated as being set.
    nxe: bool,
    /// The [`PhysicalMemorySpace`] to which the [`PaeScheme`] controls access.
    memory: M,

    /// Function to allocate a provided number of physically-contiguous bytes with a provided
    /// alignment.
    alloc_physical: AllocPhysical,

    /// Function to deallocate a range of physically-contiguous bytes
    dealloc_physical: DeallocPhysical,
}

impl<M: PhysicalMemorySpace> PaeScheme<M> {
    /// Creates a new [`PaeScheme`] with the provided flags.
    ///
    /// # Errors
    ///
    /// - [`PaeError::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`PaeError::NotActive`]: Returned if the requested mode is not supported.
    pub fn new(
        nxe: bool,
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, PaeError> {
        let paging_mode = max_supported_paging_mode();
        if !(paging_mode == PagingMode::Pae
            || paging_mode == PagingMode::Level4
            || paging_mode == PagingMode::Level5)
        {
            return Err(PaeError::NotActive);
        }

        let supports_nxe = if supports_cpuid() {
            // SAFETY:
            //
            // The CPUID instruction is supported.
            unsafe { (cpuid_unchecked(0x8000_0001, 0).edx & (1 << 20)) == (1 << 20) }
        } else {
            false
        };

        if nxe && !supports_nxe {
            return Err(PaeError::NotActive);
        }

        let mut pae = Self {
            physical_address: PhysicalAddress::zero(),
            nxe,
            memory,
            alloc_physical,
            dealloc_physical,
        };

        let frame = pae.allocate_zeroed_table().ok_or(PaeError::OutOfMemory)?;
        pae.physical_address = frame.start_address(pae.chunk_size());

        Ok(pae)
    }

    /// Creates a new [`PaeScheme`] in accordance with the the current paging scheme.
    ///
    /// # Errors
    ///
    /// - [`PaeError::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`PaeError::NotActive`]: Returned if the active mode is not `PAE` paging.
    pub fn new_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, PaeError> {
        let paging_mode = current_paging_mode();
        if !(paging_mode == PagingMode::Pae
            || paging_mode == PagingMode::Level4
            || paging_mode == PagingMode::Level5)
        {
            return Err(PaeError::NotActive);
        }

        let nxe = if supports_msr() {
            // SAFETY:
            //
            // The MSR instructions are supported.
            unsafe { (read_msr(0xC000_0080) & (1 << 11)) == (1 << 11) }
        } else {
            false
        };

        Self::new(nxe, memory, alloc_physical, dealloc_physical)
    }

    /// Creates a new [`PaeScheme`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Errors
    ///
    /// - [`PaeError::OutOfMemory`]: Never returned from this function.
    /// - [`PaeError::NotActive`]: Returned if the active mode is not `PAE` paging.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`PaeScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, PaeError> {
        let paging_mode = current_paging_mode();
        if !(paging_mode == PagingMode::Pae
            || paging_mode == PagingMode::Level4
            || paging_mode == PagingMode::Level5)
        {
            return Err(PaeError::NotActive);
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
        let address_space = Self {
            physical_address,
            nxe,
            memory,
            alloc_physical,
            dealloc_physical,
        };
        Ok(address_space)
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`PaeScheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<Frame> {
        let range = (self.alloc_physical)(self.chunk_size(), self.chunk_size())?;

        for i in 0..usize_to_u64(4096 / mem::size_of::<u64>()) {
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u8(range.start().strict_add(i * 8), 0)
                    .ok()?;
            }
        }

        Some(Frame::containing_address(range.start(), self.chunk_size()))
    }

    /// Frees a table, recursing down each allocated entry and freeing the children.
    ///
    /// # Safety
    ///
    /// This page table and all of its children must not be reachable or referenced by the root
    /// page table.
    unsafe fn free_table_recursive(&self, table_physical_address: PhysicalAddress, level: usize) {
        if level == 0 {
            return;
        }

        let entry_count = if level == 3 { 4 } else { 512 };
        for i in 0..entry_count {
            let entry_address = table_physical_address.strict_add(i * 8);
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let entry_value = unsafe {
                self.memory
                    .read_u64_le(entry_address)
                    .expect("failed to read page entry")
            };
            let entry = TranslationDescriptor::from_bits(entry_value);

            if !entry.present() {
                continue;
            }

            if level == 2 && entry.block() {
                continue;
            }

            let child_table = entry.table_address();
            // SAFETY:
            //
            // This invariants of this function ensure that this operation is safe.
            unsafe { self.free_table_recursive(PhysicalAddress::new(child_table), level - 1) }
        }

        // SAFETY:
        //
        // This invariants of this function ensure that this operation is safe.
        (self.dealloc_physical)(PhysicalAddressRange::new(
            table_physical_address,
            self.chunk_size(),
        ))
    }
}

// SAFETY:
//
// The PAE paging implementation was implemented according to the Intel specification.
unsafe impl<M: PhysicalMemorySpace> TranslationScheme for PaeScheme<M> {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(32, false)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(52, false)
    }

    fn chunk_size(&self) -> u64 {
        4096
    }

    unsafe fn map(
        &mut self,
        input: AddressChunkRange,
        output: AddressChunkRange,
        flags: MapFlags,
    ) -> Result<(), MapError> {
        if !input.is_valid(self.chunk_size(), &self.input_descriptor())
            || !output.is_valid(self.chunk_size(), &self.output_descriptor())
            || input.is_empty()
            || output.is_empty()
        {
            return Err(MapError::InvalidRange);
        }

        if input.count() != output.count() {
            return Err(MapError::MappingMismatch);
        }

        if flags.contains(MapFlags::MAY_OVERWRITE) {
            // Validate the the entire required mapping region is unmapped.
            for chunk in input.iter() {
                if self
                    .translate_input(chunk.start_address(self.chunk_size()))
                    .is_some()
                {
                    return Err(MapError::OverlapError);
                }
            }
        }

        for (input_chunk, output_chunk) in input.iter().zip(output.iter()) {
            let address = input_chunk.start_address(self.chunk_size()).value();

            let pdpte_index = (address >> 30) & 0b11;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml2_table_address = {
                let pdpte_address = self.physical_address.strict_add(pdpte_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pdpte_value = unsafe {
                    self.memory
                        .read_u64_le(pdpte_address)
                        .map_err(|_| todo!())?
                };
                let mut pdpte = PdpteDescriptor::from_bits(pdpte_value);
                if !pdpte.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::OutOfMemory);
                    };

                    pdpte = PdpteDescriptor::non_present()
                        .set_present(true)
                        .set_address(frame.start_address(self.chunk_size()).value());

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u64_le(pdpte_address, pdpte.to_bits())
                            .map_err(|_| todo!())?
                    };
                }

                PhysicalAddress::new(pdpte.address())
            };

            let pml1_table_address = {
                let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml2e_value = unsafe {
                    self.memory
                        .read_u64_le(pml2e_address)
                        .map_err(|_| todo!())?
                };
                let mut pml2e = TranslationDescriptor::from_bits(pml2e_value);
                if !pml2e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::OutOfMemory);
                    };

                    pml2e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address(self.chunk_size()).value());

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u64_le(pml2e_address, pml2e.to_bits())
                            .map_err(|_| todo!())?
                    };
                } else if pml2e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml2e.table_address())
            };

            let present = flags.contains(MapFlags::READ)
                | flags.contains(MapFlags::WRITE)
                | flags.contains(MapFlags::EXEC);
            let writable = flags.contains(MapFlags::WRITE);
            let xd = self.nxe && !flags.contains(MapFlags::EXEC);

            let pml1e_address = pml1_table_address.strict_add(pml1e_index * 8);
            let pml1e = TranslationDescriptor::non_present()
                .set_present(present)
                .set_writable(writable)
                .set_page_address(output_chunk.start_address(self.chunk_size()).value())
                .set_xd(xd);

            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u64_le(pml1e_address, pml1e.to_bits())
                    .map_err(|_| todo!())?
            };
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, input: AddressChunkRange) {
        assert!(input.is_valid(self.chunk_size(), &self.input_descriptor()));
        assert!(!input.is_empty(), "unmapping empty regions is not allowed");

        for chunk in input.iter() {
            let address = chunk.start_address(self.chunk_size()).value();

            let pdpte_index = (address >> 30) & 0b11;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml2_table_address = {
                let pdpte_address = self.physical_address.strict_add(pdpte_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pdpte_value = unsafe {
                    self.memory
                        .read_u64_le(pdpte_address)
                        .expect("failed to read PDPTE")
                };
                let pdpte = PdpteDescriptor::from_bits(pdpte_value);
                if !pdpte.present() {
                    continue;
                }

                PhysicalAddress::new(pdpte.address())
            };

            let pml1_table_address = {
                let pml2e_address = pml2_table_address.strict_add(pml2e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml2e_value = unsafe {
                    self.memory
                        .read_u64_le(pml2e_address)
                        .expect("failed to read PML2E")
                };
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

            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u64_le(pml1e_address, pml1e.to_bits())
                    .expect("failed to write PML1E")
            };
        }
    }

    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)> {
        if !input.is_valid(&self.input_descriptor()) {
            // Check if the given [`Address`] is valid for the address space.
            return None;
        }

        let pdpte_index = (input.value() >> 30) & 0b11;
        let pml2e_index = (input.value() >> 21) & 0x1FF;
        let pml1e_index = (input.value() >> 12) & 0x1FF;

        let mut writable = true;
        let mut executable = true;
        let pml2_table_address = {
            let pdpte_address = self.physical_address.checked_add(pdpte_index * 8)?;
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pdpte_value = unsafe { self.memory.read_u64_le(pdpte_address).ok()? };
            let pdpte = PdpteDescriptor::from_bits(pdpte_value);
            if !pdpte.present() {
                return None;
            }

            PhysicalAddress::new(pdpte.address())
        };

        let pml1_table_address = {
            let pml2e_address = pml2_table_address.checked_add(pml2e_index * 8)?;
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pml2e_value = unsafe { self.memory.read_u64_le(pml2e_address).ok()? };
            let pml2e = TranslationDescriptor::from_bits(pml2e_value);
            if !pml2e.present() {
                return None;
            }

            writable = writable && pml2e.writable();
            executable = executable && (self.nxe && !pml2e.xd());

            if pml2e.block() {
                let mut flags = MapFlags::READ;
                if writable {
                    flags |= MapFlags::WRITE;
                }

                if executable {
                    flags |= MapFlags::EXEC;
                }

                let offset = input.value() % (512 * self.chunk_size());
                return Some((
                    Address::new(pml2e.block_address()).strict_add(offset),
                    flags,
                ));
            }

            if pml2e.page_or_table_reserved() != 0 {
                return None;
            }
            PhysicalAddress::new(pml2e.table_address())
        };

        let pml1e_address = pml1_table_address.checked_add(pml1e_index * 8)?;
        // SAFETY:
        //
        // The invariants provided by this structure ensure that the requested read is aligned
        // and occurs on RAM.
        let pml1e_value = unsafe { self.memory.read_u64_le(pml1e_address).ok()? };
        let pml1e = TranslationDescriptor::from_bits(pml1e_value);
        if !pml1e.present() {
            return None;
        }
        writable = writable && pml1e.writable();
        executable = executable && (self.nxe && !pml1e.xd());

        let mut flags = MapFlags::READ;
        if writable {
            flags |= MapFlags::WRITE;
        }

        if executable {
            flags |= MapFlags::EXEC;
        }

        let offset = input.value() & 0xFFF;
        Some((Address::new(pml1e.page_address()).strict_add(offset), flags))
    }
}

impl<M: PhysicalMemorySpace> Drop for PaeScheme<M> {
    fn drop(&mut self) {
        // SAFETY:
        //
        // These page tables are under the exclusive control of this [`PaeScheme`] and thus
        // can be deallocated freely.
        unsafe { self.free_table_recursive(self.physical_address, 3) }
    }
}

/// Various errors that can occur when creating a [`PaeScheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaeError {
    /// An error occurrred while allocating the root page table.
    OutOfMemory,
    /// The requested mode is not active.
    NotActive,
}

impl fmt::Display for PaeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating page table memory"),
            Self::NotActive => f.pad("PAE paging is not active"),
        }
    }
}

impl error::Error for PaeError {}

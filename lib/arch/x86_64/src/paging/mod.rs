//! Implementation of [`TranslationScheme`] for 4-level and 5-level paging.

use core::{error, fmt};

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
    paging::{self, PagingMode, current_paging_mode},
};

use crate::paging::raw::TranslationDescriptor;

pub mod raw;

/// Allocates `byte_count` bytes with an alignment of `alignment` in physical memory.
pub type AllocPhysical = fn(byte_count: u64, alignment: u64) -> Option<PhysicalAddressRange>;
/// Deallocates the provided [`PhysicalAddressRange`].
pub type DeallocPhysical = fn(address: PhysicalAddressRange);

/// Implementation of [`TranslationScheme`] for `x86_64` with 4-level or 5-level paging.
pub struct LongModeScheme<M: PhysicalMemorySpace> {
    /// The physical address of the top of the page table.
    physical_address: PhysicalAddress,
    /// If `true`, the `LA57` bit should be treated as being set.
    la57: bool,
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

impl<M: PhysicalMemorySpace> LongModeScheme<M> {
    /// Creates a new [`LongModeScheme`] with the provided flags.
    ///
    /// # Errors
    ///
    /// - [`LongModeError::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`LongModeError::NotActive`]: Returned if the requested mode is not supported.
    pub fn new(
        la57: bool,
        nxe: bool,
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, LongModeError> {
        let paging_mode = paging::max_supported_paging_mode();
        match paging_mode {
            PagingMode::Level5 => {}
            PagingMode::Level4 if !la57 => {}
            _ => return Err(LongModeError::NotActive),
        }

        let supports_nxe = if supports_cpuid() {
            // SAFETY:
            //
            // The CPUID instruction is supported.
            unsafe { (cpuid_unchecked(0x8000_0001, 0).edx & (1 << 20)) != 0 }
        } else {
            false
        };

        if nxe && !supports_nxe {
            return Err(LongModeError::NotActive);
        }

        let mut scheme = Self {
            physical_address: PhysicalAddress::zero(),
            la57,
            nxe,
            memory,
            alloc_physical,
            dealloc_physical,
        };

        let frame = scheme
            .allocate_zeroed_table()
            .ok_or(LongModeError::OutOfMemory)?;
        scheme.physical_address = frame.start_address(scheme.chunk_size());

        Ok(scheme)
    }

    /// Creates a new [`LongModeScheme`] with the current configuration of the CPU.
    ///
    /// # Errors
    ///
    /// - [`LongModeError::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`LongModeError::NotActive`]: Returned if the active mode is not 4-level or 5-level
    ///   paging.
    pub fn new_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, LongModeError> {
        let paging_mode = current_paging_mode();
        if paging_mode != PagingMode::Level4 && paging_mode != PagingMode::Level5 {
            return Err(LongModeError::NotActive);
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

        Self::new(la57, nxe, memory, alloc_physical, dealloc_physical)
    }

    /// Creates a new [`LongModeScheme`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Errors
    ///
    /// - [`LongModeError::OutOfMemory`]: Never returned from this function.
    /// - [`LongModeError::NotActive`]: Returned if the active mode is not 4-level or 5-level
    ///   paging.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`LongModeScheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, LongModeError> {
        let paging_mode = current_paging_mode();
        if paging_mode != PagingMode::Level4 && paging_mode != PagingMode::Level5 {
            return Err(LongModeError::NotActive);
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

        Ok(Self {
            physical_address,
            la57,
            nxe,
            memory,
            alloc_physical,
            dealloc_physical,
        })
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`LongModeScheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<Frame> {
        let range = (self.alloc_physical)(self.chunk_size(), self.chunk_size())?;

        for i in 0..usize_to_u64(4096 / core::mem::size_of::<u64>()) {
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

    /// Frees a table, recursing down each allocated entry and freeing the children.
    ///
    /// # Safety
    ///
    /// This page table and all of its children must not be reachable or referenced by the root
    /// page table.
    unsafe fn free_table_recursive(&self, table: PhysicalAddress, level: usize) {
        if level == 0 {
            return;
        }

        for i in 0..512u64 {
            let entry_addr = table.strict_add(i * 8);
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let value = unsafe { self.memory.read_u64_le(entry_addr).ok() };
            let Some(bits) = value else { continue };
            let entry = TranslationDescriptor::from_bits(bits);

            if !entry.present() {
                continue;
            }

            if (level == 2 || level == 3) && entry.block() {
                continue;
            }

            // SAFETY:
            //
            // This invariants of this function ensure that this operation is safe.
            unsafe {
                self.free_table_recursive(PhysicalAddress::new(entry.table_address()), level - 1)
            }
        }

        (self.dealloc_physical)(PhysicalAddressRange::new(table, self.chunk_size()));
    }
}

// SAFETY:
//
// The long mode paging implementation was implemented according to the Intel specification.
unsafe impl<M: PhysicalMemorySpace> TranslationScheme for LongModeScheme<M> {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(if self.la57 { 57 } else { 48 }, true)
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

        for chunk in input.iter() {
            if self
                .translate_input(chunk.start_address(self.chunk_size()))
                .is_some()
            {
                return Err(MapError::OverlapError);
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
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml5e_value = unsafe {
                    self.memory
                        .read_u64_le(pml5e_address)
                        .map_err(|_| todo!())?
                };
                let mut pml5e = TranslationDescriptor::from_bits(pml5e_value);
                if !pml5e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::OutOfMemory);
                    };

                    pml5e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address(self.chunk_size()).value());

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u64_le(pml5e_address, pml5e.to_bits())
                            .map_err(|_| todo!())?
                    };
                }

                PhysicalAddress::new(pml5e.table_address())
            } else {
                self.physical_address
            };

            let pml3_table_address = {
                let pml4e_address = pml4_table_address.strict_add(pml4e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml4e_value = unsafe {
                    self.memory
                        .read_u64_le(pml4e_address)
                        .map_err(|_| todo!())?
                };
                let mut pml4e = TranslationDescriptor::from_bits(pml4e_value);
                if !pml4e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::OutOfMemory);
                    };

                    pml4e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address(self.chunk_size()).value());

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u64_le(pml4e_address, pml4e.to_bits())
                            .map_err(|_| todo!())?
                    };
                }

                PhysicalAddress::new(pml4e.table_address())
            };

            let pml2_table_address = {
                let pml3e_address = pml3_table_address.strict_add(pml3e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml3e_value = unsafe {
                    self.memory
                        .read_u64_le(pml3e_address)
                        .map_err(|_| todo!())?
                };
                let mut pml3e = TranslationDescriptor::from_bits(pml3e_value);
                if !pml3e.present() {
                    let Some(frame) = self.allocate_zeroed_table() else {
                        return Err(MapError::OutOfMemory);
                    };

                    pml3e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(frame.start_address(self.chunk_size()).value());

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u64_le(pml3e_address, pml3e.to_bits())
                            .map_err(|_| todo!())?
                    };
                } else if pml3e.block() {
                    todo!("implement block page splitting")
                }

                PhysicalAddress::new(pml3e.table_address())
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
        assert!(!input.is_empty());

        for chunk in input.iter() {
            let address = chunk.start_address(self.chunk_size()).value();

            let pml5e_index = (address >> 48) & 0x1FF;
            let pml4e_index = (address >> 39) & 0x1FF;
            let pml3e_index = (address >> 30) & 0x1FF;
            let pml2e_index = (address >> 21) & 0x1FF;
            let pml1e_index = (address >> 12) & 0x1FF;

            let pml4_table_address = if self.la57 {
                let pml5e_address = self.physical_address.strict_add(pml5e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml5e_value = unsafe {
                    self.memory
                        .read_u64_le(pml5e_address)
                        .expect("failed to read PML5E")
                };
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
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml4e_value = unsafe {
                    self.memory
                        .read_u64_le(pml4e_address)
                        .expect("failed to read PML4E")
                };
                let pml4e = TranslationDescriptor::from_bits(pml4e_value);
                if !pml4e.present() {
                    continue;
                }

                PhysicalAddress::new(pml4e.table_address())
            };

            let pml2_table_address = {
                let pml3e_address = pml3_table_address.strict_add(pml3e_index * 8);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pml3e_value = unsafe {
                    self.memory
                        .read_u64_le(pml3e_address)
                        .expect("failed to read PML3E")
                };
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
            return None;
        }

        let pml5e_index = (input.value() >> 48) & 0x1FF;
        let pml4e_index = (input.value() >> 39) & 0x1FF;
        let pml3e_index = (input.value() >> 30) & 0x1FF;
        let pml2e_index = (input.value() >> 21) & 0x1FF;
        let pml1e_index = (input.value() >> 12) & 0x1FF;

        let mut writable = true;
        let mut executable = true;
        let pml4_table_address = if self.la57 {
            let pml5e_address = self.physical_address.strict_add(pml5e_index * 8);
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pml5e_value = unsafe {
                self.memory
                    .read_u64_le(pml5e_address)
                    .expect("failed to read PML5E")
            };
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
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pml4e_value = unsafe {
                self.memory
                    .read_u64_le(pml4e_address)
                    .expect("failed to read PML4E")
            };
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
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pml3e_value = unsafe {
                self.memory
                    .read_u64_le(pml3e_address)
                    .expect("failed to read PML3E")
            };
            let pml3e = TranslationDescriptor::from_bits(pml3e_value);
            if !pml3e.present() {
                return None;
            }

            writable = writable && pml3e.writable();
            executable = executable && (self.nxe && !pml3e.xd());
            if pml3e.block() {
                let mut flags = MapFlags::READ;
                if writable {
                    flags |= MapFlags::WRITE;
                }

                if executable {
                    flags |= MapFlags::EXEC;
                }

                let offset = input.value() % (512 * 512 * self.chunk_size());
                return Some((
                    Address::new(pml3e.block_pml3_address()).strict_add(offset),
                    flags,
                ));
            }

            PhysicalAddress::new(pml3e.table_address())
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
                    Address::new(pml2e.block_pml2_address()).strict_add(offset),
                    flags,
                ));
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

impl<M: PhysicalMemorySpace> Drop for LongModeScheme<M> {
    fn drop(&mut self) {
        let levels = if self.la57 { 5 } else { 4 };

        // SAFETY:
        //
        // These page tables are under the exclusive control of this [`LongModeScheme`] and
        // thus can be deallocated freely.
        unsafe {
            self.free_table_recursive(self.physical_address, levels);
        }
    }
}

/// Various errors that can occur when creating a [`LongModeScheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongModeError {
    /// An error occurrred while allocating the root page table.
    OutOfMemory,
    /// The requested mode is not active.
    NotActive,
}

impl fmt::Display for LongModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating page table memory"),
            Self::NotActive => f.pad("4-level and 5-level paging are not active"),
        }
    }
}

impl error::Error for LongModeError {}

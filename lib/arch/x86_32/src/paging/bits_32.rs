//! Implementation of [`TranslationScheme`] for x86 32-bit paging.

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
    control::{Cr3, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    paging::{PagingMode, current_paging_mode},
};

use crate::paging::{AllocPhysical, DeallocPhysical, raw::bits_32::TranslationDescriptor};

/// Implementation of [`TranslationScheme`] for 32-bit paging.
pub struct Bits32Scheme<M: PhysicalMemorySpace> {
    /// Physical address of the page directory.
    physical_address: PhysicalAddress,

    /// Whether 4 MiB pages (PSE) are enabled.
    pse: bool,

    /// Whether PSE-36 is supported.
    pse36: bool,

    /// Physical memory abstraction.
    memory: M,

    /// Physical allocation function.
    alloc_physical: AllocPhysical,

    /// Physical deallocation function.
    dealloc_physical: DeallocPhysical,
}

impl<M: PhysicalMemorySpace> Bits32Scheme<M> {
    /// Creates a new [`Bits32Scheme`] with the provided flags.
    ///
    /// # Errors
    ///
    /// - [`Bits32Error::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`Bits32Error::NotActive`]: Returned if the requested mode is not supported.
    pub fn new(
        pse: bool,
        pse36: bool,
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, Bits32Error> {
        if current_paging_mode() != PagingMode::Bits32 {
            return Err(Bits32Error::NotActive);
        }

        let (pse_supported, pse36_supported) = if supports_cpuid() {
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let pse = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 3)) != 0 };
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let pse36 = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 16)) != 0 };
            (pse, pse36)
        } else {
            (false, false)
        };

        if (pse && !pse_supported) || (pse36 && !pse36_supported) {
            return Err(Bits32Error::NotActive);
        }

        let mut scheme = Self {
            physical_address: PhysicalAddress::zero(),
            pse,
            pse36,
            memory,
            alloc_physical,
            dealloc_physical,
        };

        let frame = scheme
            .allocate_zeroed_table()
            .ok_or(Bits32Error::OutOfMemory)?;
        scheme.physical_address = frame.start_address(scheme.chunk_size());

        Ok(scheme)
    }

    /// Creates a new [`Bits32Scheme`] in accordance with the the current paging scheme.
    ///
    /// # Errors
    ///
    /// - [`Bits32Error::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`Bits32Error::NotActive`]: Returned if the active mode is not 32-bit paging.
    pub fn new_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, Bits32Error> {
        if current_paging_mode() != PagingMode::Bits32 {
            return Err(Bits32Error::NotActive);
        }

        let (pse, pse36_supported) = if supports_cpuid() {
            // SAFETY:
            //
            // This code runs in ring 0 and thus it is safe to load `CR4`.
            let cr4 = unsafe { Cr4::get() };

            // SAFETY:
            //
            // The CPUID instruction is supported.
            let pse36_supported = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 16)) == (1 << 16) };

            (cr4.pse(), cr4.pse() && pse36_supported)
        } else {
            (false, false)
        };

        Self::new(
            pse,
            pse36_supported,
            memory,
            alloc_physical,
            dealloc_physical,
        )
    }

    /// Creates a new [`Bits32Scheme`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Errors
    ///
    /// - [`Bits32Error::OutOfMemory`]: Never returned from this function.
    /// - [`Bits32Error::NotActive`]: Returned if the active mode is not 4-level or 5-level
    ///   paging.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`Bits32Scheme`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current(
        memory: M,
        alloc_physical: AllocPhysical,
        dealloc_physical: DeallocPhysical,
    ) -> Result<Self, Bits32Error> {
        if current_paging_mode() != PagingMode::Bits32 {
            return Err(Bits32Error::NotActive);
        }

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR4` is safe.
        let cr4 = unsafe { Cr4::get() };
        let pse = cr4.pse();

        let pse36 = if supports_cpuid() {
            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 16)) != 0 && pse }
        } else {
            false
        };

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR3` is safe.
        let physical_address = unsafe { PhysicalAddress::new(Cr3::get().to_bits() & 0xFFFF_F000) };

        Ok(Self {
            physical_address,
            pse,
            pse36,
            memory,
            alloc_physical,
            dealloc_physical,
        })
    }

    /// Returns the required value of the `CR3` register in order to utilize this [`Bits32Scheme`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(&mut self) -> Option<Frame> {
        let range = (self.alloc_physical)(self.chunk_size(), self.chunk_size())?;

        for i in 0..usize_to_u64(4096 / mem::size_of::<u32>()) {
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u32_le(range.start().strict_add(i * 4), 0)
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

        for i in 0..1024 {
            let entry_address = table_physical_address.strict_add(i * 4);
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let entry_value = unsafe {
                self.memory
                    .read_u32_le(entry_address)
                    .expect("failed to read page entry")
            };
            let entry = TranslationDescriptor::from_bits(entry_value);

            if !entry.present() {
                continue;
            }

            if level == 2 && entry.block() {
                continue;
            }

            let child = PhysicalAddress::new(u64::from(entry.table_address()));

            // SAFETY:
            //
            // This invariants of this function ensure that this operation is safe.
            unsafe { self.free_table_recursive(child, level - 1) }
        }

        (self.dealloc_physical)(PhysicalAddressRange::new(
            table_physical_address,
            self.chunk_size(),
        ));
    }
}

// SAFETY:
//
// The 32-bit paging implementation was implemented according to the Intel specification.
unsafe impl<M: PhysicalMemorySpace> TranslationScheme for Bits32Scheme<M> {
    fn input_descriptor(&self) -> AddressSpaceDescriptor {
        AddressSpaceDescriptor::new(32, false)
    }

    fn output_descriptor(&self) -> AddressSpaceDescriptor {
        if self.pse36 {
            AddressSpaceDescriptor::new(36, false)
        } else {
            AddressSpaceDescriptor::new(32, false)
        }
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

        for (input_chunk, output_chunk) in input.iter().zip(output.iter()) {
            let address = input_chunk.start_address(self.chunk_size()).value();

            let pd_index = (address >> 22) & 0x3FF;
            let pt_index = (address >> 12) & 0x3FF;

            let pt_table_address = {
                let pde_address = self.physical_address.strict_add(pd_index * 4);
                // SAFETY:
                //
                // The invariants provided by this structure ensure that the requested read is aligned
                // and occurs on RAM.
                let pde_value = unsafe {
                    self.memory
                        .read_u32_le(pde_address)
                        .expect("failed to read PML2E")
                };
                let mut pde = TranslationDescriptor::from_bits(pde_value);

                if !pde.present() {
                    let frame = self.allocate_zeroed_table().ok_or(MapError::OutOfMemory)?;
                    let start_address =
                        u32::try_from(frame.start_address(self.chunk_size()).value())
                            .expect("physical memory allocation failed");

                    pde = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(start_address);

                    // SAFETY:
                    //
                    // The invariants provided by this structure ensure that the requested read is aligned
                    // and occurs on RAM.
                    unsafe {
                        self.memory
                            .write_u32_le(pde_address, pde.to_bits())
                            .expect("failed to write PML2E")
                    }
                }

                PhysicalAddress::new(u64::from(pde.table_address()))
            };

            let present = flags.contains(MapFlags::READ)
                | flags.contains(MapFlags::WRITE)
                | flags.contains(MapFlags::EXEC);

            let writable = flags.contains(MapFlags::WRITE);

            let pte_address = pt_table_address.strict_add(pt_index * 4);

            let start_address =
                u32::try_from(output_chunk.start_address(self.chunk_size()).value())
                    .expect("physical memory allocation failed");

            let pte = TranslationDescriptor::non_present()
                .set_present(present)
                .set_writable(writable)
                .set_page_address(start_address);

            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u32_le(pte_address, pte.to_bits())
                    .expect("failed to write PML1E")
            }
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, input: AddressChunkRange) {
        assert!(input.is_valid(self.chunk_size(), &self.input_descriptor()));
        assert!(!input.is_empty());

        for chunk in input.iter() {
            let address = chunk.start_address(self.chunk_size()).value();

            let pd_index = (address >> 22) & 0x3FF;
            let pt_index = (address >> 12) & 0x3FF;

            let pde_address = self.physical_address.strict_add(pd_index * 4);
            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            let pde_value = unsafe {
                self.memory
                    .read_u32_le(pde_address)
                    .expect("failed to read PML2E")
            };
            let pde = TranslationDescriptor::from_bits(pde_value);

            if !pde.present() {
                continue;
            }

            let pt_table_address = PhysicalAddress::new(u64::from(pde.table_address()));

            let pte_address = pt_table_address.strict_add(pt_index * 4);

            // SAFETY:
            //
            // The invariants provided by this structure ensure that the requested read is aligned
            // and occurs on RAM.
            unsafe {
                self.memory
                    .write_u32_le(pte_address, TranslationDescriptor::non_present().to_bits())
                    .expect("failed to write PML1E")
            }
        }
    }

    fn translate_input(&self, input: Address) -> Option<(Address, MapFlags)> {
        if !input.is_valid(&self.input_descriptor()) {
            return None;
        }

        let value = input.value();
        let pd_index = (value >> 22) & 0x3FF;
        let pt_index = (value >> 12) & 0x3FF;

        let mut writable = true;

        let pde_address = self.physical_address.checked_add(pd_index * 4)?;
        // SAFETY:
        //
        // The invariants provided by this structure ensure that the requested read is aligned
        // and occurs on RAM.
        let pde_value = unsafe { self.memory.read_u32_le(pde_address).ok()? };
        let pde = TranslationDescriptor::from_bits(pde_value);

        if !pde.present() {
            return None;
        }

        writable &= pde.writable();

        if self.pse && pde.block() {
            let offset = value % (1024 * self.chunk_size());
            let mut flags = MapFlags::READ;
            if writable {
                flags |= MapFlags::WRITE;
            }
            return Some((Address::new(pde.block_address()).strict_add(offset), flags));
        }

        let pt_table_address = PhysicalAddress::new(u64::from(pde.table_address()));

        let pte_address = pt_table_address.checked_add(pt_index * 4)?;
        // SAFETY:
        //
        // The invariants provided by this structure ensure that the requested read is aligned
        // and occurs on RAM.
        let pte_value = unsafe { self.memory.read_u32_le(pte_address).ok()? };
        let pte = TranslationDescriptor::from_bits(pte_value);

        if !pte.present() {
            return None;
        }

        writable &= pte.writable();

        let offset = value & 0xFFF;

        let mut flags = MapFlags::READ;
        if writable {
            flags |= MapFlags::WRITE;
        }

        Some((
            Address::new(u64::from(pte.page_address())).strict_add(offset),
            flags,
        ))
    }
}

impl<M: PhysicalMemorySpace> Drop for Bits32Scheme<M> {
    fn drop(&mut self) {
        // SAFETY:
        //
        // These page tables are under the exclusive control of this [`Bits32Scheme`] and thus can
        // be deallocated freely.
        unsafe { self.free_table_recursive(self.physical_address, 2) }
    }
}

/// Various errors that can occur when creating a [`Bits32Scheme`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bits32Error {
    /// An error occurrred while allocating the root page table.
    OutOfMemory,
    /// The requested mode is not active.
    NotActive,
}

impl fmt::Display for Bits32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "error allocating page table memory"),
            Self::NotActive => f.pad("32-bit paging is not active"),
        }
    }
}

impl error::Error for Bits32Error {}

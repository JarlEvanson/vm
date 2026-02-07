//! Implementation of 32-bit paging implementation.

use core::{error, fmt, mem};

use x86_32::paging::bits_32::TranslationDescriptor;
use x86_common::{
    control::{Cr3, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    paging::{PagingMode, current_paging_mode},
};

use crate::{
    arch::generic::address_space::{AddressSpace, MapError, NoMapping, NotFound, ProtectionFlags},
    platform::{
        AllocationPolicy, Frame, FrameAllocation, FrameRange, OutOfMemory, PhysicalAddress,
        allocate_frames_aligned, deallocate_frames, frame_size,
    },
    util::usize_to_u64,
};

/// A function that reads a value from the given `address`.
type ReadU32 = fn(address: PhysicalAddress) -> u32;
/// A function that writes `value` to the given `address`.
type WriteU32 = fn(address: PhysicalAddress, value: u32);

/// Implementation of [`AddressSpace`] for `x86_32` with 32-bit paging.
pub struct Bits32AddressSpace {
    /// The physical address of the top of the page table.
    physical_address: PhysicalAddress,
    /// If `true`, the `PSE` bit should be treated as being set.
    pse: bool,
    /// If `true`, the `PSE-36` bit should be treated as being set.
    pse36: bool,
    /// Function to read a [`u32`] from a specified physical address.
    read: ReadU32,
    /// Function to write a [`u32`] to a specified physical address.
    write: WriteU32,
}

impl Bits32AddressSpace {
    /// Creates a new [`Bits32AddressSpace`] with the provided flags.
    ///
    /// # Errors
    ///
    /// - [`Bits32Error::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`Bits32Error::NotActive`]: Returned if the requested mode is not supported.
    pub fn new(
        pse: bool,
        pse36: bool,
        read: ReadU32,
        write: WriteU32,
    ) -> Result<Self, Bits32Error> {
        let (pse_supported, pse36_supported) = if supports_cpuid() {
            // SAFETY:
            //
            // The CPUID instruction is supported.
            let pse = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 3)) == (1 << 3) };

            // SAFETY:
            //
            // The CPUID instruction is supported.
            let pse36 = unsafe { (cpuid_unchecked(0x1, 0).edx & (1 << 16)) == (1 << 16) };

            (pse, pse36)
        } else {
            (false, false)
        };

        if (pse && !pse_supported) || (pse36 && !pse36_supported) {
            return Err(Bits32Error::NotActive);
        }

        let frame_allocation = Self::allocate_zeroed_table(write)?;
        let physical_address = frame_allocation.range().start().start_address();

        // Forget about the [`FrameAllocation`] to prevent it from dropping.
        mem::forget(frame_allocation);
        Ok(Self {
            physical_address,
            pse,
            pse36,
            read,
            write,
        })
    }

    /// Creates a new [`Bits32AddressSpace`] in accordance with the the current paging scheme.
    ///
    /// # Errors
    ///
    /// - [`Bits32Error::OutOfMemory`]: Returned if the allocation of the root frame failed.
    /// - [`Bits32Error::NotActive`]: Returned if the active mode is not 32-bit paging.
    pub fn new_current(read: ReadU32, write: WriteU32) -> Result<Self, Bits32Error> {
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

        Self::new(pse, pse36_supported, read, write)
    }

    /// Creates a new [`Bits32AddressSpace`] by taking over the current page tables referenced by
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
    /// For the lifetime of this object, the newly created [`Bits32AddressSpace`] must have
    /// exclusive control over the memory making up the page tables.
    pub unsafe fn active_current(read: ReadU32, write: WriteU32) -> Result<Self, Bits32Error> {
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

        // SAFETY:
        //
        // The system is in ring 0 and thus reading `CR3` is safe.
        let physical_address = unsafe { PhysicalAddress::new(Cr3::get().to_bits() & 0xFFFF_FFFF) };
        let address_space = Self {
            physical_address,
            pse,
            pse36: pse36_supported,
            read,
            write,
        };
        Ok(address_space)
    }

    /// Returns the required value of the `CR3` register in order to utilize this
    /// [`Bits32AddressSpace`].
    pub fn cr3(&self) -> u64 {
        self.physical_address.value()
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(write: WriteU32) -> Result<FrameAllocation, OutOfMemory> {
        let policy = AllocationPolicy::Below(u64::from(u32::MAX));
        let frame_allocation = allocate_frames_aligned(Self::frames_in_4_kib(), 4096, policy)?;

        for i in 0..usize_to_u64(4096 / mem::size_of::<u32>()) {
            write(
                frame_allocation.range().start().start_address().add(i * 4),
                0,
            );
        }
        Ok(frame_allocation)
    }

    /// Returns the number of frames in 4 KiB.
    fn frames_in_4_kib() -> u64 {
        frame_size().div_ceil(4096)
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
            let entry_address = table_physical_address.add(i * 4);
            let entry = TranslationDescriptor::from_bits((self.read)(entry_address));

            if !entry.present() {
                continue;
            }

            if level == 2 && entry.block() {
                continue;
            }

            let child_table = u64::from(entry.table_address());
            // SAFETY:
            //
            // This invariants of this function ensure that this operation is safe.
            unsafe { self.free_table_recursive(PhysicalAddress::new(child_table), level - 1) }
        }

        // SAFETY:
        //
        // This invariants of this function ensure that this operation is safe.
        unsafe {
            deallocate_frames(FrameRange::new(
                Frame::containing_address(table_physical_address),
                Self::frames_in_4_kib(),
            ))
        }
    }
}

impl AddressSpace for Bits32AddressSpace {
    fn page_size(&self) -> u64 {
        4096
    }

    fn max_virtual_address(&self) -> u64 {
        u64::from(u32::MAX)
    }

    fn max_physical_address(&self) -> u64 {
        if self.pse36 {
            (1 << 36) - 1
        } else {
            (1 << 32) - 1
        }
    }

    #[expect(clippy::as_conversions)]
    fn map(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        count: u64,
        protection: ProtectionFlags,
    ) -> Result<(), MapError> {
        if !virtual_address.is_multiple_of(4096) || !physical_address.is_multiple_of(4096) {
            return Err(MapError::AlignmentError);
        }

        let Some(requested_mapping_size) = count.checked_mul(4096u64) else {
            return Err(MapError::InvalidSize);
        };

        let physical_end_address = physical_address
            .checked_add(requested_mapping_size)
            .ok_or(MapError::WrapAroundError)?;
        if physical_end_address > self.max_physical_address() {
            return Err(MapError::WrapAroundError);
        }

        let virtual_end_address = virtual_address
            .checked_add(requested_mapping_size)
            .ok_or(MapError::WrapAroundError)?;
        if virtual_end_address > self.max_virtual_address() {
            return Err(MapError::WrapAroundError);
        }

        // Validate the the entire required mapping region is unmapped.
        for index in 0..count {
            let offset = index.strict_mul(self.page_size());
            let virtual_address = virtual_address.strict_add(offset);
            if self.translate_virt(virtual_address).is_ok() {
                return Err(MapError::OverlapError);
            }
        }

        for index in 0..count {
            let offset = index.strict_mul(self.page_size());
            let physical_address = physical_address.strict_add(offset);
            let virtual_address = virtual_address.strict_add(offset);

            let pml2e_index = (virtual_address >> 22) & 0x3FF;
            let pml1e_index = (virtual_address >> 12) & 0x3FF;

            let pml1_table_address = {
                let pml2e_address = self.physical_address.add(pml2e_index * 4);
                let pml2e_value = (self.read)(pml2e_address);
                let mut pml2e = TranslationDescriptor::from_bits(pml2e_value);
                if !pml2e.present() {
                    let Ok(frame_allocation) = Self::allocate_zeroed_table(self.write) else {
                        return Err(MapError::AllocationError);
                    };

                    pml2e = TranslationDescriptor::non_present()
                        .set_present(true)
                        .set_writable(true)
                        .set_table_address(
                            frame_allocation.range().start().start_address().value() as u32,
                        );
                    (self.write)(pml2e_address, pml2e.to_bits());
                    mem::forget(frame_allocation);
                }

                if pml2e.block() {
                    unreachable!("overlapping pml2 mapping has already been checked")
                }

                PhysicalAddress::new(u64::from(pml2e.table_address()))
            };

            let present = protection.readable() || protection.writable() || protection.executable();
            let writable = protection.writable();

            let pml1e_address = pml1_table_address.add(pml1e_index * 4);
            let pml1e = TranslationDescriptor::non_present()
                .set_present(present)
                .set_writable(writable)
                .set_page_address(physical_address as u32);
            (self.write)(pml1e_address, pml1e.to_bits());
        }

        Ok(())
    }

    unsafe fn unmap(&mut self, virtual_address: u64, count: u64) {
        #[cfg(debug_assertions)]
        {
            // Validate that the entire required unmapping region was already mapped.
            for index in 0..count {
                let offset = index.strict_mul(self.page_size());
                let virtual_address = virtual_address.strict_add(offset);
                assert!(self.translate_virt(virtual_address).is_ok());
            }
        }

        for index in 0..count {
            let offset = index.strict_mul(self.page_size());
            let virtual_address = virtual_address.strict_add(offset);

            let pml2e_index = (virtual_address >> 22) & 0x3FF;
            let pml1e_index = (virtual_address >> 12) & 0x3FF;

            let pml1_table_address = {
                let pml2e_address = self.physical_address.add(pml2e_index * 4);
                let pml2e_value = (self.read)(pml2e_address);
                let pml2e = TranslationDescriptor::from_bits(pml2e_value);
                if pml2e.block() {
                    todo!("implement unmapping pml2 mappings");
                }

                PhysicalAddress::new(u64::from(pml2e.table_address()))
            };

            let pml1e_address = pml1_table_address.add(pml1e_index * 4);
            (self.write)(
                pml1e_address,
                TranslationDescriptor::non_present().to_bits(),
            );
        }
    }

    fn find_region(&self, count: u64) -> Result<u64, NotFound> {
        let max_va = self.max_virtual_address();

        let mut current = self.page_size();
        while count
            .checked_mul(self.page_size())
            .and_then(|size| current.checked_add(size))
            .is_some_and(|max_region_va| max_region_va <= max_va)
        {
            let mut free = true;
            for i in 0..count {
                if self.translate_virt(current + i * self.page_size()).is_ok() {
                    free = false;
                    current += (i + 1) * self.page_size();
                    break;
                }
            }
            if free {
                return Ok(current);
            }
        }

        Err(NotFound)
    }

    fn translate_virt(&self, virtual_address: u64) -> Result<(u64, ProtectionFlags), NoMapping> {
        if virtual_address > u64::from(u32::MAX) {
            return Err(NoMapping);
        }

        let pml2e_index = (virtual_address >> 22) & 0x3FF;
        let pml1e_index = (virtual_address >> 12) & 0x3FF;

        let mut flags = ProtectionFlags::READ | ProtectionFlags::WRITE | ProtectionFlags::EXEC;
        let pml1_table_address = {
            let pml2e_address = self.physical_address.add(pml2e_index * 4);
            let pml2e_value = (self.read)(pml2e_address);
            let pml2e = TranslationDescriptor::from_bits(pml2e_value);
            if !pml2e.present() {
                return Err(NoMapping);
            }

            flags = flags.set_writable(flags.writable() && pml2e.writable());
            if self.pse && pml2e.block() {
                let offset = virtual_address % (1024 * self.page_size());
                return Ok((pml2e.block_address() + offset, flags));
            }

            PhysicalAddress::new(u64::from(pml2e.table_address()))
        };

        let pml1e_address = pml1_table_address.add(pml1e_index * 4);
        let pml1e_value = (self.read)(pml1e_address);
        let pml1e = TranslationDescriptor::from_bits(pml1e_value);
        if !pml1e.present() {
            return Err(NoMapping);
        }

        flags = flags.set_writable(flags.writable() && pml1e.writable());

        let offset = virtual_address % self.page_size();
        Ok((u64::from(pml1e.page_address()) + offset, flags))
    }
}

impl Drop for Bits32AddressSpace {
    fn drop(&mut self) {
        // SAFETY:
        //
        // These page tables are under the exclusive control of this [`PaeAddressSpace`] and thus
        // can be deallocated freely.
        unsafe { self.free_table_recursive(self.physical_address, 2) }
    }
}

/// Various errors that can occur when creating a [`Bits32AddressSpace`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bits32Error {
    /// An error occurrred while allocating the root page table.
    OutOfMemory(OutOfMemory),
    /// The requested mode is not active.
    NotActive,
}

impl From<OutOfMemory> for Bits32Error {
    fn from(value: OutOfMemory) -> Self {
        Self::OutOfMemory(value)
    }
}

impl fmt::Display for Bits32Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory(error) => write!(f, "error allocating page table memory: {error}"),
            Self::NotActive => f.pad("32-bit paging is not active"),
        }
    }
}

impl error::Error for Bits32Error {}

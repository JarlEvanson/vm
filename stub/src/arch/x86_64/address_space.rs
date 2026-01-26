//! Implementation of [`AddressSpace`] for `x86_64`.

use core::{arch, matches, mem, ops::ControlFlow};

use x86_64::paging::TranslationDescriptor;
use x86_common::{
    msr::read_msr,
    paging::{PagingMode, current_paging_mode},
};

use crate::{
    arch::generic::address_space::{AddressSpace, MapError, NoMapping, NotFound, ProtectionFlags},
    platform::{
        AllocationPolicy, FrameAllocation, OutOfMemory, allocate_frames_aligned, frame_size,
    },
    util::usize_to_u64,
};

/// A function that reads a value from the given `address`.
type ReadU64 = fn(address: u64) -> u64;
/// A function that writes `value` to the given `address`.
type WriteU64 = fn(address: u64, value: u64);

/// Implementation of [`AddressSpace`] for `x86_64` with 4-level or 5-level paging.
pub struct X86_64AddressSpace {
    /// The physical address of the top of the page table.
    physical_address: u64,
    /// If `true`, the LA57 bit is set (5-level paging is enabled).
    la57: bool,
    /// If `true`, the NXE bit is set (execute-disable enabled).
    nxe: bool,
    /// Function to read a [`u64`] from a specified physical address.
    read: ReadU64,
    /// Function to write a [`u64`] to a specified physical address.
    write: WriteU64,
}

impl X86_64AddressSpace {
    /// Creates a new [`X86_64AddressSpace`] with the following flags.
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the allocation of the root page failed.
    pub fn new(la57: bool, nxe: bool, read: ReadU64, write: WriteU64) -> Result<Self, OutOfMemory> {
        let frame_allocation = Self::allocate_zeroed_table(write)?;
        let physical_address = frame_allocation.physical_address();

        // Forget about the [`FrameAllocation`] to prevent it from dropping.
        mem::forget(frame_allocation);
        Ok(Self {
            physical_address,
            la57,
            nxe,
            read,
            write,
        })
    }

    /// Creates a new [`X86_64AddressSpace`] in accordance with the the current paging scheme.
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the allocation of the root page failed.
    ///
    /// # Panics
    ///
    /// Panics if the current paging mode is not 4-level or 5-level or if the processor does not
    /// support MSR interactions.
    pub fn new_current(read: ReadU64, write: WriteU64) -> Result<Self, OutOfMemory> {
        let paging_mode = current_paging_mode();
        assert!(matches!(
            paging_mode,
            PagingMode::Level4 | PagingMode::Level5
        ));
        let la57 = paging_mode != PagingMode::Level4;

        assert!(x86_common::msr::supports_msr());
        // SAFETY:
        //
        // The MSR instructions are supported.
        let nxe = unsafe { read_msr(0xC0000080) } & (1 << 11) == (1 << 11);

        Self::new(la57, nxe, read, write)
    }

    /// Creates a new [`X86_64AddressSpace`] by taking over the current page tables referenced by
    /// `CR3`.
    ///
    /// # Safety
    ///
    /// For the lifetime of this object, the newly created [`X86_64AddressSpace`] must have
    /// exclusive control over the memory making up the page tables.
    ///
    /// # Panics
    ///
    /// Panics if the current paging mode is not 4-level or 5-level or if the processor does not
    /// support MSR interactions.
    pub unsafe fn active_current(read: ReadU64, write: WriteU64) -> Self {
        let paging_mode = current_paging_mode();
        assert!(matches!(
            paging_mode,
            PagingMode::Level4 | PagingMode::Level5
        ));
        let la57 = paging_mode != PagingMode::Level4;

        assert!(x86_common::msr::supports_msr());
        // SAFETY:
        //
        // The MSR instructions are supported.
        let nxe = unsafe { read_msr(0xC0000080) } & (1 << 11) == (1 << 11);

        let cr3: u64;
        unsafe { arch::asm!("mov {}, cr3", lateout(reg) cr3) }

        Self {
            physical_address: cr3 & 0x000F_FFFF_FFFF_F000,
            la57,
            nxe,
            read,
            write,
        }
    }

    /// Returns the value to load into the CR3 register to utilize this [`AddressSpace`].
    pub fn cr3(&self) -> u64 {
        self.physical_address
    }

    /// Maps the physical region beginning at `physical_address` and extending `count *
    /// Self::page_size()` bytes into the virtual address space at `virtual_address` with the
    /// specified [`ProtectionFlags`].
    fn map_unchecked(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        count: u64,
        protection: ProtectionFlags,
    ) -> Result<(), MapError> {
        let not_present_handler =
            |_: ReadU64, write: WriteU64, entry_address: u64| -> ControlFlow<MapError> {
                let Ok(frame_allocation) = Self::allocate_zeroed_table(write) else {
                    return ControlFlow::Break(MapError::AllocationError);
                };
                let physical_address = frame_allocation.physical_address();

                mem::forget(frame_allocation);
                write(
                    entry_address,
                    TranslationDescriptor::new_table(physical_address)
                        .set_writable(true)
                        .to_bits(),
                );

                ControlFlow::Continue(())
            };

        for index in 0..count {
            let map_address = virtual_address + index * 4096u64;
            let mapped_address = physical_address + index * 4096u64;

            let pml5_index = (map_address >> 48) & 0x1FF;
            let pml4_index = (map_address >> 39) & 0x1FF;
            let pml3_index = (map_address >> 30) & 0x1FF;
            let pml2_index = (map_address >> 21) & 0x1FF;
            let pml1_index = (map_address >> 12) & 0x1FF;

            let pml4_table = if self.la57 {
                next_level(
                    self.read,
                    self.write,
                    self.physical_address,
                    pml5_index,
                    not_present_handler,
                )?
            } else {
                self.physical_address
            };

            let pml3_table = next_level(
                self.read,
                self.write,
                pml4_table,
                pml4_index,
                not_present_handler,
            )?;
            let pml2_table = next_level(
                self.read,
                self.write,
                pml3_table,
                pml3_index,
                not_present_handler,
            )?;
            let pml1_table = next_level(
                self.read,
                self.write,
                pml2_table,
                pml2_index,
                not_present_handler,
            )?;
            (self.write)(
                pml1_table + 8 * pml1_index,
                TranslationDescriptor::new_page(mapped_address)
                    .set_present(protection.readable())
                    .set_writable(protection.writable())
                    .set_xd(self.nxe && !protection.executable())
                    .to_bits(),
            );
        }

        Ok(())
    }

    /// Allocates a zeroed page table.
    fn allocate_zeroed_table(write: WriteU64) -> Result<FrameAllocation, OutOfMemory> {
        let frame_allocation =
            allocate_frames_aligned(Self::frames_in_4_kib(), 4096, AllocationPolicy::Any)?;

        for i in 0..usize_to_u64(4096 / mem::size_of::<u64>()) {
            write(frame_allocation.physical_address() + i * 8, 0);
        }
        Ok(frame_allocation)
    }

    /// Returns the number of frames in 4 KiB.
    fn frames_in_4_kib() -> u64 {
        frame_size().div_ceil(4096)
    }
}

impl AddressSpace for X86_64AddressSpace {
    fn page_size(&self) -> u64 {
        4096
    }

    fn max_virtual_address(&self) -> u64 {
        u64::MAX
    }

    fn max_physical_address(&self) -> u64 {
        (1u64 << 52) - 1
    }

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

        if physical_address
            .checked_add(requested_mapping_size)
            .is_none_or(|max_address| max_address > self.max_physical_address())
        {
            return Err(MapError::WrapAroundError);
        }

        let virtual_end_address = virtual_address
            .checked_add(requested_mapping_size)
            .ok_or(MapError::WrapAroundError)?;
        if virtual_end_address > self.max_virtual_address() {
            return Err(MapError::WrapAroundError);
        }

        let virtual_region = (virtual_address, virtual_end_address);
        let invalid_region = virtual_region_bounds(self.la57);
        if virtual_region.0 <= invalid_region.1 && invalid_region.0 <= virtual_region.1 {
            return Err(MapError::InvalidAddress);
        }

        self.map_unchecked(virtual_address, physical_address, count, protection)
    }

    unsafe fn unmap(&mut self, virtual_address: u64, count: u64) {
        for index in 0..count {
            let virtual_address = virtual_address + index * 4096;

            let pml5_index = (virtual_address >> 48) & 0x1FF;
            let pml4_index = (virtual_address >> 39) & 0x1FF;
            let pml3_index = (virtual_address >> 30) & 0x1FF;
            let pml2_index = (virtual_address >> 21) & 0x1FF;
            let pml1_index = (virtual_address >> 12) & 0x1FF;

            let pml4_table = if self.la57 {
                match next_level(
                    self.read,
                    self.write,
                    self.physical_address,
                    pml5_index,
                    |_, _, _| ControlFlow::Break(()),
                ) {
                    Ok(addr) => addr,
                    Err(_) => continue, // nothing mapped, skip.
                }
            } else {
                self.physical_address
            };

            let pml3_table =
                match next_level(self.read, self.write, pml4_table, pml4_index, |_, _, _| {
                    ControlFlow::Break(())
                }) {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };
            let pml2_table =
                match next_level(self.read, self.write, pml3_table, pml3_index, |_, _, _| {
                    ControlFlow::Break(())
                }) {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };
            let pml1_table =
                match next_level(self.read, self.write, pml2_table, pml2_index, |_, _, _| {
                    ControlFlow::Break(())
                }) {
                    Ok(addr) => addr,
                    Err(_) => continue,
                };

            (self.write)(
                pml1_table + 8 * pml1_index,
                TranslationDescriptor::non_present().to_bits(),
            );
        }

        todo!()
    }

    fn find_region(&self, count: u64) -> Result<u64, NotFound> {
        let max_va = self.max_virtual_address();
        let mut current = self.page_size();

        while current + count * 4096 <= max_va {
            let mut free = true;
            for i in 0..count {
                if self.translate_virt(current + i * 4096).is_ok() {
                    free = false;
                    current += (i + 1) * 4096;
                    break;
                }
            }
            if free {
                return Ok(current);
            }
        }

        Err(NotFound)
    }

    fn translate_virt(&self, virtual_address: u64) -> Result<u64, NoMapping> {
        let not_present_handler = |_, _, _| ControlFlow::Break(NoMapping);

        let pml5_index = (virtual_address >> 48) & 0x1FF;
        let pml4_index = (virtual_address >> 39) & 0x1FF;
        let pml3_index = (virtual_address >> 30) & 0x1FF;
        let pml2_index = (virtual_address >> 21) & 0x1FF;
        let pml1_index = (virtual_address >> 12) & 0x1FF;
        let offset = virtual_address & 0xFFF;

        let pml4_table = if self.la57 {
            next_level(
                self.read,
                self.write,
                self.physical_address,
                pml5_index,
                not_present_handler,
            )?
        } else {
            self.physical_address
        };

        let pml3_table = next_level(
            self.read,
            self.write,
            pml4_table,
            pml4_index,
            not_present_handler,
        )?;
        let pml2_table = next_level(
            self.read,
            self.write,
            pml3_table,
            pml3_index,
            not_present_handler,
        )?;
        let pml1_table = next_level(
            self.read,
            self.write,
            pml2_table,
            pml2_index,
            not_present_handler,
        )?;

        let entry = TranslationDescriptor::from_bits((self.read)(pml1_table + 8 * pml1_index));
        if !entry.present() {
            return Err(NoMapping);
        }

        Ok(entry.page_address() + offset)
    }
}

/// Descends to the next level of the page table tree.
fn next_level<E>(
    read: ReadU64,
    write: WriteU64,
    table_address: u64,
    index: u64,
    not_present_handler: fn(ReadU64, WriteU64, u64) -> ControlFlow<E>,
) -> Result<u64, E> {
    let entry_address = table_address + 8 * index;
    let mut entry = TranslationDescriptor::from_bits(read(entry_address));
    if !entry.present() {
        match not_present_handler(read, write, entry_address) {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(err) => return Err(err),
        }
    }

    entry = TranslationDescriptor::from_bits(read(entry_address));
    Ok(entry.table_address())
}

/// Returns the bounds of the invalid region in the middle of the address space.
const fn virtual_region_bounds(level_5: bool) -> (u64, u64) {
    if level_5 {
        (0x0100_0000_0000_0000, 0xFEFF_FFFF_FFFF_FFFF)
    } else {
        (0x0000_8000_0000_0000, 0xFFFF_7FFF_FFFF_FFFF)
    }
}

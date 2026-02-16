//! Long mode paging implementation for `x86_32` and `x86_64` use and debugging.

use x86_64::paging::TranslationDescriptor;

use crate::{
    arch::generic::memory::{phys::PhysicalMemory, virt::FindFreeRegionError},
    memory::phys::structs::PhysicalAddress,
};

/// Implementation of various useful operations of an `x86_64` long mode page table structure.
pub struct LongModeTable<M: PhysicalMemory> {
    /// The `CR3` value associated with the given [`LongModeTable`].
    cr3: u64,
    /// Whether the table should be interpreted as a 5-level table.
    la57: bool,
    /// The [`PhysicalMemory`] provider.
    provider: M,
}

impl<M: PhysicalMemory> LongModeTable<M> {
    /// Constructs a new [`LongModeTable`] from the provided arguments.
    pub const fn new(cr3: u64, la57: bool, provider: M) -> Self {
        Self {
            cr3,
            la57,
            provider,
        }
    }

    /// Returns the virtual address at the start of a range of free virtual memory in the the
    /// virtual address space represented by this [`LongModeTable`].
    ///
    pub fn find_free_region(&self, count: u64) -> Result<u64, FindFreeRegionError<M::Error>> {
        let top_table_address = self.cr3 & 0x000f_ffff_ffff_f000;

        let mut checked_count = 0;
        let (end_5_index, end_4_index, end_3_index, end_2_index, end_1_index) = 'l: {
            for pml5e_index in 0..512 {
                if pml5e_index != 0 && !self.la57 {
                    break;
                }

                let pml4_table = if self.la57 {
                    let pml5e_address = PhysicalAddress::new(top_table_address + pml5e_index * 8);

                    // SAFETY:
                    //
                    // The page table was constructed with the proper physical memory ranges and
                    // thus it is safe to access.
                    let pml5e_value = unsafe {
                        self.provider
                            .read_u64_le(pml5e_address)
                            .map_err(FindFreeRegionError::MemoryError)?
                    };
                    let pml5e = TranslationDescriptor::from_bits(pml5e_value);
                    if !pml5e.present() {
                        checked_count += 512 * 512 * 512 * 512;
                        if checked_count >= count {
                            break 'l (pml5e_index, 512, 512, 512, 512);
                        }
                        continue;
                    }

                    pml5e.table_address()
                } else {
                    top_table_address
                };

                for pml4e_index in 0..512 {
                    let pml3_table = {
                        let pml4e_address = PhysicalAddress::new(pml4_table + pml4e_index * 8);

                        // SAFETY:
                        //
                        // The page table was constructed with the proper physical memory ranges and
                        // thus it is safe to access.
                        let pml4e_value = unsafe {
                            self.provider
                                .read_u64_le(pml4e_address)
                                .map_err(FindFreeRegionError::MemoryError)?
                        };
                        let pml4e = TranslationDescriptor::from_bits(pml4e_value);
                        if !pml4e.present() {
                            checked_count += 512 * 512 * 512;
                            if checked_count >= count {
                                break 'l (pml5e_index, pml4e_index, 512, 512, 512);
                            }
                            continue;
                        }

                        pml4e.table_address()
                    };

                    for pml3e_index in 0..512 {
                        let pml2_table = {
                            let pml3e_address = PhysicalAddress::new(pml3_table + pml3e_index * 8);

                            // SAFETY:
                            //
                            // The page table was constructed with the proper physical memory ranges and
                            // thus it is safe to access.
                            let pml3e_value = unsafe {
                                self.provider
                                    .read_u64_le(pml3e_address)
                                    .map_err(FindFreeRegionError::MemoryError)?
                            };
                            let pml3e = TranslationDescriptor::from_bits(pml3e_value);
                            if !pml3e.present() {
                                checked_count += 512 * 512;
                                if checked_count >= count {
                                    break 'l (pml5e_index, pml4e_index, pml3e_index, 512, 512);
                                }
                                continue;
                            } else if pml3e.block() {
                                checked_count = 0;
                                continue;
                            }

                            pml3e.table_address()
                        };

                        for pml2e_index in 0..512 {
                            let pml1_table = {
                                let pml2e_address =
                                    PhysicalAddress::new(pml2_table + pml2e_index * 8);

                                // SAFETY:
                                //
                                // The page table was constructed with the proper physical memory ranges and
                                // thus it is safe to access.
                                let pml2e_value = unsafe {
                                    self.provider
                                        .read_u64_le(pml2e_address)
                                        .map_err(FindFreeRegionError::MemoryError)?
                                };
                                let pml2e = TranslationDescriptor::from_bits(pml2e_value);
                                if !pml2e.present() {
                                    checked_count += 512;
                                    if checked_count >= count {
                                        break 'l (
                                            pml5e_index,
                                            pml4e_index,
                                            pml3e_index,
                                            pml2e_index,
                                            512,
                                        );
                                    }
                                    continue;
                                } else if pml2e.block() {
                                    checked_count = 0;
                                    continue;
                                }

                                pml2e.table_address()
                            };

                            for pml1e_index in 0..512 {
                                if pml5e_index == 0
                                    && pml4e_index == 0
                                    && pml3e_index == 0
                                    && pml2e_index == 0
                                    && pml1e_index == 0
                                {
                                    continue;
                                }

                                let pml1e_address =
                                    PhysicalAddress::new(pml1_table + pml1e_index * 8);

                                // SAFETY:
                                //
                                // The page table was constructed with the proper physical memory ranges and
                                // thus it is safe to access.
                                let pml1e_value = unsafe {
                                    self.provider
                                        .read_u64_le(pml1e_address)
                                        .map_err(FindFreeRegionError::MemoryError)?
                                };
                                let pml1e = TranslationDescriptor::from_bits(pml1e_value);
                                if !pml1e.present() {
                                    checked_count += 1;
                                    if checked_count >= count {
                                        break 'l (
                                            pml5e_index,
                                            pml4e_index,
                                            pml3e_index,
                                            pml2e_index,
                                            pml1e_index + 1,
                                        );
                                    }
                                } else {
                                    checked_count = 0;
                                }
                            }
                        }
                    }
                }
            }

            unreachable!("find_free_region must not iterate over entire address space")
        };

        let end_address = (end_5_index << 48)
            + (end_4_index << 39)
            + (end_3_index << 30)
            + (end_2_index << 21)
            + (end_1_index << 12);

        let start_address = end_address - count * 4096;

        Ok(start_address)
    }
}

//! Virtual memory support for the Linux `x86_64` boot protocol.

use conversion::{u64_to_usize, u64_to_usize_strict, usize_to_u64};
use sync::{ControlledModificationCell, Spinlock};
use x86::paging::{
    PagingMode, bits_64::TranslationDescriptor, current_paging_mode, tlb::invalidate_page,
};

use crate::{
    arch::{
        generic::memory::paging::{
            ExternalFrame, ExternalFrameRange, ExternalPage, ExternalPageRange,
            ExternalPhysicalAddress, ExternalVirtualAddress, SearchStrategy, TranslationScheme,
        },
        memory::{ArchTranslationScheme, physical_bits},
    },
    platform::{
        FrameRange, MapError, MappingType, Page, PageRange, Permissions, PhysicalAddress,
        VirtualAddress, VirtualMemoryManager, frame_size, linux::x86_64::LinuxImpl, page_size,
    },
};

/// The address of the temporary mapping page.
const TEMP_PAGE_ADDRESS: usize = usize::MAX - 4096 * 2 + 1;
/// The PML1 table used to implement the temp mapping.
static TEMP_TABLE: Table = Table(ControlledModificationCell::new(
    [TranslationDescriptor::non_present(); 512],
));

/// The [`TranslationScheme`] used to implement page mapping and unmapping.
static TRANSLATION_SCHEME: Spinlock<Option<ArchTranslationScheme>> = Spinlock::new(None);

impl VirtualMemoryManager for LinuxImpl {
    fn max_physical_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(1u64 << physical_bits()).strict_sub(1)
    }

    fn map(
        &self,
        frames: FrameRange,
        permissions: Permissions,
        mapping_type: MappingType,
    ) -> Result<PageRange, MapError> {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let frame_count = if usize_to_u64(page_size()) >= frame_size() {
            frames
                .count()
                .div_ceil(usize_to_u64(page_size()) / frame_size())
        } else {
            frames
                .count()
                .strict_mul(frame_size() / usize_to_u64(page_size()))
        };

        let frames = ExternalFrameRange::new(
            ExternalFrame::containing_address(
                ExternalPhysicalAddress::new(frames.start_address().value()),
                scheme.chunk_size(),
            ),
            frame_count,
        );
        let pages = scheme.map(SearchStrategy::TopDown, frames, permissions, mapping_type)?;

        let range = PageRange::new(
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.start_address(scheme.chunk_size()).value(),
            ))),
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.end_address_inclusive(scheme.chunk_size()).value(),
            ))),
        );

        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }

        Ok(range)
    }

    fn map_identity(
        &self,
        frames: FrameRange,
        permissions: Permissions,
    ) -> Result<PageRange, MapError> {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let frame_count = if usize_to_u64(page_size()) >= frame_size() {
            frames
                .count()
                .div_ceil(usize_to_u64(page_size()) / frame_size())
        } else {
            frames
                .count()
                .strict_mul(frame_size() / usize_to_u64(page_size()))
        };

        let frames = ExternalFrameRange::new(
            ExternalFrame::containing_address(
                ExternalPhysicalAddress::new(frames.start_address().value()),
                scheme.chunk_size(),
            ),
            frame_count,
        );
        let pages = scheme.map_identity(frames, permissions)?;

        let range = PageRange::new(
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.start_address(scheme.chunk_size()).value(),
            ))),
            Page::containing_address(VirtualAddress::new(u64_to_usize_strict(
                pages.end_address_inclusive(scheme.chunk_size()).value(),
            ))),
        );

        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }

        Ok(range)
    }

    fn map_temporary(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
        let pml1_index = (TEMP_PAGE_ADDRESS >> 12) & 0x1FF;
        let new_descriptor = TEMP_TABLE.get()[pml1_index].set_page_address(address.value());
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        unsafe { TEMP_TABLE.get_mut()[pml1_index] = new_descriptor }
        invalidate_page(TEMP_PAGE_ADDRESS);

        let offset = u64_to_usize(address.value() % 4096);
        Some(VirtualAddress::new(TEMP_PAGE_ADDRESS + offset))
    }

    fn translate_virtual(
        &self,
        address: VirtualAddress,
    ) -> Option<(Permissions, MappingType, PhysicalAddress)> {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let virtual_address_u64 = usize_to_u64(address.value());
        scheme
            .translate(ExternalVirtualAddress::new(virtual_address_u64))
            .map(|(permissions, mapping_type, address)| {
                (
                    permissions,
                    mapping_type,
                    PhysicalAddress::new(address.value()),
                )
            })
    }

    unsafe fn unmap(&self, range: PageRange) {
        let mut scheme = TRANSLATION_SCHEME.lock();
        let scheme = scheme
            .as_mut()
            .expect("virtual memory management initialization failed");

        let input = ExternalPageRange::new(
            ExternalPage::containing_address(
                ExternalVirtualAddress::new(usize_to_u64(range.start_address().value())),
                scheme.chunk_size(),
            ),
            ExternalPage::containing_address(
                ExternalVirtualAddress::new(usize_to_u64(range.end_address_inclusive().value())),
                scheme.chunk_size(),
            ),
        );

        // SAFETY:
        //
        // The invariants of [`VirtualMemoryManager::unmap()`] fulfill the invariants of
        // [`ArchTranslationScheme::unmap()`].
        unsafe { scheme.unmap(input) }

        for page in range.iter() {
            x86::paging::tlb::invalidate_page(page.start_address().value());
        }
    }
}

/// Initializes the self-controlled [`TranslationScheme`].
pub fn setup_initial_mappings(
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
) {
    static PML5_TABLE: Table = Table(ControlledModificationCell::new(
        [TranslationDescriptor::non_present(); 512],
    ));
    static PML4_TABLES: [Table; 3] = [const {
        Table(ControlledModificationCell::new(
            [TranslationDescriptor::non_present(); 512],
        ))
    }; 3];
    static PML3_TABLES: [Table; 3] = [const {
        Table(ControlledModificationCell::new(
            [TranslationDescriptor::non_present(); 512],
        ))
    }; 3];
    static PML2_TABLES: [Table; 3] = [const {
        Table(ControlledModificationCell::new(
            [TranslationDescriptor::non_present(); 512],
        ))
    }; 3];
    static PML1_TABLES: [Table; 512] = [const {
        Table(ControlledModificationCell::new(
            [TranslationDescriptor::non_present(); 512],
        ))
    }; 512];

    let mut next_pml4_table = 0;
    let mut next_pml3_table = 0;
    let mut next_pml2_table = 0;
    let mut next_pml1_table = 0;

    let enable_pml5 = match current_paging_mode() {
        PagingMode::Disabled | PagingMode::Bits32 | PagingMode::Pae => unreachable!(),
        PagingMode::Level4 => false,
        PagingMode::Level5 => true,
    };

    let ranges = [
        (image_start, image_size),
        (stack_start, stack_size),
        (usize_to_u64(TEMP_PAGE_ADDRESS), 4096),
    ];
    for (index, (start_address, size)) in ranges.into_iter().enumerate() {
        let start = PhysicalAddress::new(start_address).align_down(4096);
        let end = PhysicalAddress::new(start_address)
            .strict_add(size)
            .strict_align_up(4096);

        let mut current_address = start;
        while current_address < end {
            let va = u64_to_usize(current_address.value());

            let pml5_index = if enable_pml5 { (va >> 48) & 0x1FF } else { 0 };
            let pml4_index = (va >> 39) & 0x1FF;
            let pml3_index = (va >> 30) & 0x1FF;
            let pml2_index = (va >> 21) & 0x1FF;
            let pml1_index = (va >> 12) & 0x1FF;

            // SAFETY:
            //
            // This code ensures that all table operations are safe.
            #[expect(clippy::multiple_unsafe_ops_per_block)]
            unsafe {
                if !PML5_TABLE.get()[pml5_index].present() {
                    assert!(
                        next_pml4_table < PML4_TABLES.len(),
                        "Exceeded available static PML4 tables"
                    );
                    let address = PML4_TABLES[next_pml4_table].get().as_ptr() as u64;
                    PML5_TABLE.get_mut()[pml5_index] =
                        TranslationDescriptor::new_table(address).set_writable(true);
                    next_pml4_table += 1;
                }

                let pml4_table_address = PML5_TABLE.get()[pml5_index].table_address();
                let pml4_table = &mut *(pml4_table_address as *mut [TranslationDescriptor; 512]);

                if !pml4_table[pml4_index].present() {
                    assert!(
                        next_pml3_table < PML3_TABLES.len(),
                        "Exceeded available static PML3 tables"
                    );
                    let address = PML3_TABLES[next_pml3_table].get().as_ptr() as u64;
                    pml4_table[pml4_index] =
                        TranslationDescriptor::new_table(address).set_writable(true);
                    next_pml3_table += 1;
                }

                let pml3_table_address = pml4_table[pml4_index].table_address();
                let pml3_table = &mut *(pml3_table_address as *mut [TranslationDescriptor; 512]);

                if !pml3_table[pml3_index].present() {
                    assert!(
                        next_pml2_table < PML2_TABLES.len(),
                        "Exceeded available static PML2 tables"
                    );
                    let address = PML2_TABLES[next_pml2_table].get().as_ptr() as u64;
                    pml3_table[pml3_index] =
                        TranslationDescriptor::new_table(address).set_writable(true);
                    next_pml2_table += 1;
                }

                let pml2_table_address = pml3_table[pml3_index].table_address();
                let pml2_table = &mut *(pml2_table_address as *mut [TranslationDescriptor; 512]);

                if !pml2_table[pml2_index].present() {
                    if index == ranges.len() - 1 {
                        let address = TEMP_TABLE.get().as_ptr() as u64;
                        pml2_table[pml2_index] =
                            TranslationDescriptor::new_table(address).set_writable(true);
                    } else {
                        assert!(
                            next_pml1_table < PML1_TABLES.len(),
                            "Exceeded available static PML1 tables"
                        );
                        let address = PML1_TABLES[next_pml1_table].get().as_ptr() as u64;
                        pml2_table[pml2_index] =
                            TranslationDescriptor::new_table(address).set_writable(true);
                        next_pml1_table += 1;
                    }
                }

                let pml1_table_address = pml2_table[pml2_index].table_address();
                let pml1_table = &mut *(pml1_table_address as *mut [TranslationDescriptor; 512]);

                pml1_table[pml1_index] =
                    TranslationDescriptor::new_page(current_address.align_down(4096).value())
                        .set_writable(true);
            }

            current_address = current_address.strict_add(4096);
        }
    }

    let root_table_address = match current_paging_mode() {
        PagingMode::Disabled | PagingMode::Bits32 | PagingMode::Pae => unreachable!(),
        PagingMode::Level4 => PML4_TABLES[0].get().as_ptr() as u64,
        PagingMode::Level5 => PML5_TABLE.get().as_ptr() as u64,
    };

    // SAFETY:
    //
    // The invariants of [`setup_initial_mappings()`] ensures that this operation is safe.
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) root_table_address, options(nostack, preserves_flags))
    }

    let mut scheme = TRANSLATION_SCHEME.lock();

    // SAFETY:
    //
    // This takeover of [`ArchAddressSpace`] is only performed once and this program has exclusive
    // control over the system.
    let current_scheme = unsafe {
        ArchTranslationScheme::active_current().expect("failed to initialize address space")
    };
    *scheme = Some(current_scheme);

    drop(scheme);
}

/// 4096-byte aligned `x86_64` page table.
#[repr(align(4096))]
struct Table(ControlledModificationCell<[TranslationDescriptor; 512]>);

impl core::ops::Deref for Table {
    type Target = ControlledModificationCell<[TranslationDescriptor; 512]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

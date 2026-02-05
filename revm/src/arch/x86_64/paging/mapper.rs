//! Implementation of `revm`'s page mapper.

use sync::Spinlock;
use x86_64::paging::TranslationDescriptor;
use x86_common::paging::{PagingMode, current_paging_mode};

use crate::util::{u64_to_usize, usize_to_u64};

/// The singular instance of `revm`'s page mapper.
static MAPPER: Spinlock<Mapper> = Spinlock::new(Mapper {
    la_57: false,
    mapper_physical_address: 0,

    top_level: PageTable::new(),

    internal_pml4: PageTable::new(),
    internal_pml3: PageTable::new(),
    internal_pml2: PageTable::new(),
    internal_pml1: PageTable::new(),

    last_internal_pml4_index: 0,
    last_internal_pml3_index: 0,
    last_internal_pml2_index: 0,
    last_internal_pml1_index: 0,

    external_pml4: PageTable::new(),
    external_pml3: PageTable::new(),
    external_pml2: PageTable::new(),
    external_pml1: PageTable::new(),

    last_external_pml4_index: 0,
    last_external_pml3_index: 0,
    last_external_pml2_index: 0,
    last_external_pml1_index: 0,

    scratch: PageTable::new(),
});

/// Initializes the page mapper with the physical address of the start of the image.
///
/// # Safety
///
/// `image_physical_address` and `image_virtual_address` must be the actual addresses associated
/// with the image and the image must be physically contiguous.
pub unsafe fn initialize_mapper(image_physical_address: u64, image_virtual_address: u64) {
    let mut mapper = MAPPER.lock();
    let mapper_virtual_address = usize_to_u64((&raw const *mapper).addr());
    let mapper_offset = mapper_virtual_address.strict_sub(image_virtual_address);

    let paging_mode = current_paging_mode();
    assert!(paging_mode == PagingMode::Level4 || paging_mode == PagingMode::Level5);

    mapper.la_57 = paging_mode != PagingMode::Level4;
    mapper.mapper_physical_address = mapper_offset.strict_add(image_physical_address);
}

/// Maps the physical memory region of `size` bytes that starts at `physical_address` into the
/// `revm` address at `virtual_address`.
pub fn map_at(physical_address: u64, virtual_address: usize, size: usize) {}

fn map_internal(
    physical_address: u64,
    virtual_address: usize,
    size: usize,
    map: fn(u64) -> *mut u8,
    unmap: fn(*mut u8),
) {
    assert!(physical_address % 4096 == usize_to_u64(virtual_address) % 4096);
    assert_ne!(size, 0);

    let max_physical_address = physical_address.strict_add(usize_to_u64(size));
    assert!(max_physical_address <= (1 << 52));

    let max_virtual_address = virtual_address.strict_add(size);

    let mut pml5_index = (virtual_address >> 48) & 0x1FF;
    let mut pml4_index = (virtual_address >> 39) & 0x1FF;
    let mut pml3_index = (virtual_address >> 30) & 0x1FF;
    let mut pml2_index = (virtual_address >> 21) & 0x1FF;
    let mut pml1_index = (virtual_address >> 12) & 0x1FF;

    let end_pml5_index = (max_virtual_address.strict_sub(1) >> 48) & 0x1FF;
    let end_pml4_index = (max_virtual_address.strict_sub(1) >> 39) & 0x1FF;
    let end_pml3_index = (max_virtual_address.strict_sub(1) >> 30) & 0x1FF;
    let end_pml2_index = (max_virtual_address.strict_sub(1) >> 21) & 0x1FF;
    let end_pml1_index = (max_virtual_address.strict_sub(1) >> 12) & 0x1FF;

    let mapper = MAPPER.lock();
    loop {
        if mapper.la_57 {}

        if pml5_index == end_pml5_index
            && pml4_index == end_pml4_index
            && pml3_index == end_pml3_index
            && pml2_index == end_pml2_index
            && pml1_index == end_pml1_index
        {
            break;
        }

        pml1_index += 1;
        if pml1_index == 512 {
            pml1_index = 0;
            pml2_index += 1;
            if pml2_index == 512 {
                pml2_index = 0;
                pml3_index += 1;
                if pml3_index == 512 {
                    pml3_index = 0;
                    pml4_index += 1;
                    if pml4_index == 512 {
                        pml4_index = 0;
                        pml5_index += 1;
                    }
                }
            }
        }
    }
}

/// Maps the physical memory region inside which `physical_address` is contained into `revm`'s
/// address space.
///
/// This function is destructive: any call to this function will invalidate all previous temporary
/// mappings produced by [`map_temporary()`] and its derivatives.
pub fn map_temporary(physical_address: u64) -> (*mut u8, usize) {
    let mapper = MAPPER.lock();

    todo!()
}

struct Mapper {
    la_57: bool,
    mapper_physical_address: u64,

    top_level: PageTable,

    internal_pml4: PageTable,
    internal_pml3: PageTable,
    internal_pml2: PageTable,
    internal_pml1: PageTable,

    last_internal_pml4_index: usize,
    last_internal_pml3_index: usize,
    last_internal_pml2_index: usize,
    last_internal_pml1_index: usize,

    external_pml4: PageTable,
    external_pml3: PageTable,
    external_pml2: PageTable,
    external_pml1: PageTable,

    last_external_pml4_index: usize,
    last_external_pml3_index: usize,
    last_external_pml2_index: usize,
    last_external_pml1_index: usize,

    // This will never be touched but instead serves to provide a known-good virtual page that
    // internal temporary mappings can use.
    scratch: PageTable,
}

impl Mapper {}

#[repr(C, align(4096))]
struct PageTable([TranslationDescriptor; 512]);

impl PageTable {
    pub const fn new() -> Self {
        Self([TranslationDescriptor::non_present(); 512])
    }
}

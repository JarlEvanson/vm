//! `x86_64`-specific paging functionality.

#[cfg(target_arch = "x86_64")]
use core::convert::Infallible;

#[cfg(target_arch = "x86_64")]
use crate::{arch::generic::memory::virt::FindFreeRegionError, memory::virt::structs::PageRange};

/// Returns a [`PageRange`] representing a range of free virtual [`Page`][p]s in the active address
/// space.
///
/// [p]: crate::memory::virt::structs::Page
#[cfg(target_arch = "x86_64")]
#[expect(clippy::missing_errors_doc)]
pub fn find_free_region(count: usize) -> Result<PageRange, FindFreeRegionError<Infallible>> {
    use x86_common::{
        control::Cr3,
        paging::{PagingMode, current_paging_mode},
    };

    use crate::{
        arch::x86_common::paging::long_mode::LongModeTable,
        memory::phys::BareMetalMemory,
        memory::virt::structs::{Page, VirtualAddress},
        util::{u64_to_usize, usize_to_u64},
    };

    // SAFETY:
    //
    // The executable operates in Ring 0 and thus it is safe to read `CR3`.
    let cr3 = unsafe { Cr3::get() };
    let la57 = current_paging_mode() == PagingMode::Level5;

    let start_address = LongModeTable::new(cr3.to_bits(), la57, BareMetalMemory)
        .find_free_region(usize_to_u64(count))?;

    let start_page = Page::containing_address(VirtualAddress::new(u64_to_usize(start_address)));
    Ok(PageRange::new(start_page, count))
}

//! Definition and implementation of a managed physical memory region.

use core::{mem, ptr::NonNull};

use conversion::usize_to_u64;
use sync::Spinlock;

use crate::memory::phys::PhysicalAddress;

/// The head of the [`ManagedRegion`] list.
static MANAGED_REGIONS: Spinlock<Option<&'static ManagedRegion>> = Spinlock::new(None);

#[derive(Debug)]
pub struct ManagedRegion {
    /// The address of the next [`ManagedRegion`].
    next: Option<NonNull<ManagedRegion>>,

    /// The [`PhysicalAddress`] at the base of this [`ManagedRegion`].
    physical_address: PhysicalAddress,
}

impl ManagedRegion {
    /// The number of bytes that each [`ManagedRegion`] controls.
    const REGION_SIZE: u64 = 2 * 1024 * 1024;
    /// The number of bits required to be mapped in each [`ManagedRegion`].
    const REQUIRED_MAPPING_SIZE: u64 = usize_to_u64(mem::size_of::<ManagedRegion>())
        + Self::REGION_SIZE.div_ceil(4096).div_ceil(8);
}

// SAFETY:
//
// TODO:
unsafe impl Send for ManagedRegion {}
// SAFETY:
//
// TODO:
unsafe impl Sync for ManagedRegion {}

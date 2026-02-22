//! Architecture-specific code.

use std::mem;

use crate::platform::{AllocationPolicy, StubPhysicalMemory, allocate_frames_aligned};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_common;

pub mod generic;

use memory::address::AddressRange;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use x86_common::paging::X86CommonSchemeError as ArchSchemeError;

/// Implementation of [`TranslationScheme`][memory::translation::TranslationScheme`] for the
/// current architecture.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub type ArchScheme = x86_common::paging::X86CommonScheme<StubPhysicalMemory>;

fn alloc_physical(count: u64, alignment: u64) -> Option<AddressRange> {
    allocate_frames_aligned(
        count.div_ceil(frame_size()),
        alignment,
        AllocationPolicy::Any,
    )
    .ok()
    .map(|allocation| {
        mem::forget(allocation);
    })
}

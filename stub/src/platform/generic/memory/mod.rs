//! Definitions and interfaces that platforms utilize to provide memory services for use by the
//! rest of the executable.

use core::sync::atomic::{AtomicBool, Ordering};

use sync::ControlledModificationCell;

mod alloc;
mod mem_structs;
mod phys;
mod virt;

pub use alloc::*;
pub use mem_structs::{
    Frame, FrameRange, Page, PageRange, PhysicalAddress, PhysicalAddressRange, VirtualAddress,
    VirtualAddressRange,
};
pub use phys::*;
pub use virt::*;

/// The current [`MemoryConfig`].
static MEMORY_CONFIG: ControlledModificationCell<Option<MemoryConfig>> =
    ControlledModificationCell::new(None);

/// Initializes the memory configuration of the system.
pub(in crate::platform) fn initialize_memory_config(
    frame_size: u64,
    phys_addr_bits: u8,
    page_size: usize,
) {
    /// Guard to prevent modification of the [`MemoryConfig`] after the initial call.
    static MEMORY_CONFIGURATION_GUARD: AtomicBool = AtomicBool::new(false);

    assert!(
        !MEMORY_CONFIGURATION_GUARD.swap(true, Ordering::Relaxed),
        "memory configuration must only be set once"
    );

    assert!(frame_size.is_power_of_two());
    assert!(phys_addr_bits <= 64);
    assert!(page_size.is_power_of_two());

    // SAFETY:
    //
    // All functions that acquire an immutable reference to `MEMORY_CONFIG` do not call functions
    // that modify `MEMORY_CONFIG`, while all functions that acquire a mutable reference to
    // `MEMORY_CONFIG` are leaf functions.
    let memory_config = unsafe { MEMORY_CONFIG.get_mut() };
    *memory_config = Some(MemoryConfig {
        frame_size,
        phys_addr_bits,

        page_size,
    });
}

/// Returns the active [`MemoryConfig`].
fn memory_config() -> &'static MemoryConfig {
    MEMORY_CONFIG
        .get()
        .as_ref()
        .expect("the memory configuration must be initialized before any memory call")
}

/// Returns the size, in bytes, of a [`Frame`].
pub fn frame_size() -> u64 {
    memory_config().frame_size
}

/// Returns the number of bits that makes up an addressable physical address.
///
/// This can depend on the paging mode.
pub fn physical_address_bits() -> u8 {
    memory_config().phys_addr_bits
}

/// Returns the maximum addressable [`PhysicalAddress`].
///
/// This can depend on the paging mode.
pub fn maximum_physical_address() -> PhysicalAddress {
    if physical_address_bits() == 64 {
        PhysicalAddress::new(u64::MAX)
    } else {
        PhysicalAddress::new((1u64 << physical_address_bits()) - 1)
    }
}

/// Returns the size, in bytes, of a [`Page`].
pub fn page_size() -> usize {
    memory_config().page_size
}

/// The memory configuration of the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MemoryConfig {
    /// The size, in bytes, of the basic unit of physical memory allocation.
    frame_size: u64,
    /// The number of bits that make up an addressable physical address.
    ///
    /// This can depend on the paging mode.
    phys_addr_bits: u8,

    /// The size, in bytes, of the basic unit of virtual memory allocation.
    page_size: usize,
}

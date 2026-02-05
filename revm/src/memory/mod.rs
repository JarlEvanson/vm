//! Definitions and implementations of virtual and physical memory management APIs for `revm`.

use core::{
    error, fmt,
    sync::atomic::{AtomicU64, Ordering},
};

pub mod phys;
pub mod virt;

static PAGE_FRAME_SIZE: AtomicU64 = AtomicU64::new(0);

/// Initializes the memory management subsystem for `revm`.
pub fn initialize_memory_management(page_frame_size: u64) {
    assert!(
        page_frame_size.is_power_of_two(),
        "page frame sizes must be a power of two"
    );
    assert_eq!(
        PAGE_FRAME_SIZE.load(Ordering::Relaxed),
        0,
        "page frame size must only be set once"
    );

    PAGE_FRAME_SIZE.store(page_frame_size, Ordering::Relaxed);
}

/// Returns the size, in bytes, of a page and frame.
pub fn page_frame_size() -> u64 {
    PAGE_FRAME_SIZE.load(Ordering::Relaxed)
}

/// Indicates that there were no frame regions that were free and complied with the provided flags.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("out of memory")
    }
}

impl error::Error for OutOfMemory {}

//! Definitions and implementations of interfaces provided by architecture-specific code related to
//! virtual memory.

use core::{error, fmt};

/// Various errors that can occur while searching for a free [`PageRange`][pr].
///
/// [pr]: crate::memory::virt::structs::PageRange
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindFreeRegionError<E> {
    /// A suitable [`PageRange`][pr] could not be found.
    ///
    /// [pr]: crate::memory::virt::structs::PageRange
    NotFound,
    /// An error ocurred while attempting to access physical memory.
    MemoryError(E),
}

impl<E: fmt::Display> fmt::Display for FindFreeRegionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "free region could not be located"),
            Self::MemoryError(error) => write!(f, "error while reading physical memory: {error}"),
        }
    }
}

impl<E: error::Error + 'static> error::Error for FindFreeRegionError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::NotFound => None,
            Self::MemoryError(error) => Some(error),
        }
    }
}

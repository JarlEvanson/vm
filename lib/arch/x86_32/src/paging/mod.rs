//! Implementation of 32-bit paging and PAE paging structures.

use memory::address::PhysicalAddressRange;

pub mod bits_32;
pub mod pae;
pub mod raw;

/// Allocates `byte_count` bytes with an alignment of `alignment` in physical memory.
pub type AllocPhysical = fn(byte_count: u64, alignment: u64) -> Option<PhysicalAddressRange>;
/// Deallocates the provided [`PhysicalAddressRange`].
pub type DeallocPhysical = fn(address: PhysicalAddressRange);

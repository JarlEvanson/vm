//! Implementation of various paging structures for `aarch64`.

use memory::address::PhysicalAddressRange;

pub mod raw;
pub mod vmsa_v8;

/// Allocates `byte_count` bytes with an alignment of `alignment` in physical memory.
pub type AllocPhysical = fn(byte_count: u64, alignment: u64) -> Option<PhysicalAddressRange>;
/// Deallocates the provided [`PhysicalAddressRange`].
pub type DeallocPhysical = fn(address: PhysicalAddressRange);

/// The maximum number of relevant bits in an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddressSize {
    /// The maximum number of relevant bits in an address is 48 bits.
    Bits48,
    /// The maximum number of relevant bits in an address is 52 bits.
    Bits52,
}

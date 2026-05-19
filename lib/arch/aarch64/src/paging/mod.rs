//! Definitions related to paging structures for `aarch64`.

pub mod vmsa_v8;

/// The maximum number of relevant bits in an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddressSize {
    /// The maximum number of relevant bits in an address is 48 bits.
    Bits48,
    /// The maximum number of relevant bits in an address is 52 bits.
    Bits52,
}

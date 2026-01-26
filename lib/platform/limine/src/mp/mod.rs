//! Definitions of [`MpRequest`] and architecture specific responses.

use core::{ffi::c_void, ops};

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

pub mod aarch64;
pub mod riscv64;
pub mod x86_64;

/// Magic numbers identifying the request as a [`MpRequest`].
pub const MP_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x95a67b819a1b857e,
    0xa0b61b723b6a73e0,
];

/// Request for the other processors to be initialized and waiting on a spinloop.
#[repr(C)]
#[derive(Debug)]
pub struct MpRequest {
    /// Location storing [`MP_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`MpRequest`] structure.
    pub revision: u64,
    /// A pointer to the response structure for this [`MpRequest`].
    pub response: *mut c_void,
    /// Various flags.
    pub flags: MpRequestFlags,
}

// SAFETY:
//
// [`MpRequest`] does not interact with threads in any manner.
unsafe impl Send for MpRequest {}
// SAFETY:
//
// [`MpRequest`] does not interact with threads in any manner.
unsafe impl Sync for MpRequest {}

/// Flags that influence the behavior of the [`MpRequest`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpRequestFlags(u64);

impl MpRequestFlags {
    /// Default flags.
    pub const DEFAULT: Self = Self(0);

    /// Enable X2APIC if possible.
    pub const X86_64_ENABLE_X2APIC: Self = Self(1);
}

impl ops::BitOr for MpRequestFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for MpRequestFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl ops::BitAnd for MpRequestFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for MpRequestFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl ops::BitXor for MpRequestFlags {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for MpRequestFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl ops::Not for MpRequestFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

//! Definitions of `x86_64` responses.

use core::ops;

/// Response indicating that other processors have been initialized.
///
/// Provides access to the bootstrap processor ID and information about the processor on the
/// system.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpResponse {
    /// The revision of the [`MpResponse`] structure.
    pub revision: u64,
    /// Flags indicating various multiprocessing specific situations.
    pub flags: MpResponseFlags,
    /// The local APIC ID of the bootstrap processor.
    pub bsp_lapic_id: u32,
    /// The number of CPUs that are present.
    pub cpu_count: u64,
    /// A pointer to an array of [`MpResponse::cpu_count`] pointers to [`MpInfo`] structures
    pub cpus: u64,
}

// SAFETY:
//
// [`MpResponse`] does not interact with threads in any manner.
unsafe impl Send for MpResponse {}
// SAFETY:
//
// [`MpResponse`] does not interact with threads in any manner.
unsafe impl Sync for MpResponse {}

/// Various flags that indicate the state that the system is in.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpResponseFlags(u32);

impl MpResponseFlags {
    /// Default flags.
    pub const DEFAULT: Self = Self(0);

    /// Enable X2APIC if possible.
    pub const ENABLED_X2APIC: Self = Self(1);
}

impl ops::BitOr for MpResponseFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for MpResponseFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl ops::BitAnd for MpResponseFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for MpResponseFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl ops::BitXor for MpResponseFlags {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for MpResponseFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl ops::Not for MpResponseFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Information regarding a single CPU.
///
/// This also provides the jump field.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpInfo {
    /// ACPI processor UID as specified by the MADT.
    pub processor_id: u32,
    /// Local APIC ID of the processor as specified by the MADT.
    pub lapic_id: u32,
    /// Reserved field.
    pub _reserved: u64,
    /// An atomic write to this field causes the parked CPU to jump to the written address on a
    /// stack.
    ///
    /// A pointer to the [`MpInfo`] struct associated with the CPU is passed in RDI.
    pub goto_address: u64,
    /// A field free for use.
    pub extra_argument: u64,
}

//! Definitions of `aarch64` responses.

/// Response indicating that other processors on the system have been initialized.
///
/// Provides access to the bootstrap processor ID and information about the processor on the
/// system.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpResponse {
    /// The revision of the [`MpResponse`] structure.
    pub revision: u64,
    /// Always zero.
    pub flags: u64,
    /// MPIDR of the bootstrap processor (as read from MPIDR_EL1).
    pub bsp_mpidr: u64,
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

/// Information for a single CPU.
///
/// This also provides a jump field.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MpInfo {
    /// ACPI processor UID as specified by the MADT.
    pub processor_id: u32,
    /// Reserved.
    pub _reserved_1: u32,
    /// THe MPIDR of the processor.
    pub mpidr: u64,
    /// Reserved.
    pub _reserved_2: u64,
    /// An atomic write to this field causes the parked CPU to jump to the written address on a
    /// stack.
    ///
    /// A pointer to the [`MpInfo`] struct associated with the CPU is passed in X0.
    pub goto_address: u64,
    /// A field free for use.
    pub extra_argument: u64,
}

//! Definitions of [`PagingModeRequestV0`], [`PagingModeRequestV1`], and [`PagingModeResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`PagingModeRequestV0`] or [`PagingModeRequestV1`].
pub const PAGING_MODE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x95c1a0edab0944cb,
    0xa4e5cb3842f7488a,
];

/// Request determines the preferred [`PagingMode`].
#[repr(C)]
#[derive(Debug)]
pub struct PagingModeRequestV0 {
    /// Location storing [`PAGING_MODE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`PagingModeRequestV0`] structure.
    pub revision: u64,
    /// A pointer to the [`PagingModeResponse`] structure for this [`PagingModeRequestV0`].
    pub response: u64,
    /// The preferred [`PagingMode`] by the OS.
    pub mode: PagingMode,
}

// SAFETY:
//
// [`PagingModeRequest`] does not interact with threads in any manner.
unsafe impl Send for PagingModeRequestV0 {}
// SAFETY:
//
// [`PagingModeRequest`] does not interact with threads in any manner.
unsafe impl Sync for PagingModeRequestV0 {}

/// Request determines the preferred [`PagingMode`] and the minimum and maximum supported
/// [`PagingMode`]s.
#[repr(C)]
#[derive(Debug)]
pub struct PagingModeRequestV1 {
    /// Definition of the base revision's structure.
    pub base_revision: PagingModeRequestV0,

    /// The maximum [`PagingMode`] that the OS supports.
    pub max_mode: PagingMode,
    /// The minimum [`PagingMode`] that the OS supports.
    pub min_mode: PagingMode,
}

// SAFETY:
//
// [`PagingModeRequest`] does not interact with threads in any manner.
unsafe impl Send for PagingModeRequestV1 {}
// SAFETY:
//
// [`PagingModeRequest`] does not interact with threads in any manner.
unsafe impl Sync for PagingModeRequestV1 {}

/// Response to a [`PagingModeRequestV0`] or [`PagingModeRequestV1`].
#[repr(C)]
#[derive(Debug)]
pub struct PagingModeResponse {
    /// The revision of the [`PagingModeResponse`] structure.
    pub revision: u64,
    /// The [`PagingMode`] enabled by the bootloader.
    pub mode: PagingMode,
}

// SAFETY:
//
// [`PagingModeResponse`] does not interact with threads in any manner.
unsafe impl Send for PagingModeResponse {}
// SAFETY:
//
// [`PagingModeResponse`] does not interact with threads in any manner.
unsafe impl Sync for PagingModeResponse {}

/// A paging mode for a supported architecture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PagingMode(pub u64);

impl PagingMode {
    /// `x86_64` 4-level paging.
    pub const X86_64_LVL_4: Self = Self(0);
    /// `x86_64` 5-level paging.
    pub const X86_64_LVL_5: Self = Self(1);

    /// `aarch64` 4-level paging.
    pub const AARCH64_LVL_4: Self = Self(0);
    /// `aarch64` 5-level paging.
    pub const AARCH64_LVL_5: Self = Self(1);

    /// `riscv64` 39-bit paging.
    pub const RISCV64_SV39: Self = Self(0);
    /// `riscv64` 48-bit paging.
    pub const RISCV64_SV48: Self = Self(1);
    /// `riscv64` 57-bit paging.
    pub const RISCV64_SV57: Self = Self(2);

    /// `loongarch64` 4-level paging.
    pub const LOONGARCH64_LVL_4: Self = Self(0);
}

//! Definitions of [`ModuleRequestV0`], [`ModuleRequestV1`] and [`ModuleResponse`].

use core::{ffi::c_char, ops};

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1, executable::File};

/// Magic numbers identifying the request as an [`ModuleRequestV0`] or [`ModuleRequestV1`]].
pub const MODULE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x3e7e279702be32af,
    0xca1c4f3bd1280cee,
];

/// Request for loaded modules.
#[repr(C)]
#[derive(Debug)]
pub struct ModuleRequestV0 {
    /// Location storing [`MODULE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`ModuleRequestV0`] or [`ModuleRequestV1`] structure.
    pub revision: u64,
    /// A pointer to the [`ModuleResponse`] structure for this [`ModuleRequestV0`] or [`ModuleRequestV1`].
    pub response: *mut ModuleResponse,
}

// SAFETY:
//
// [`ModuleRequestV0`] does not interact with threads in any manner.
unsafe impl Send for ModuleRequestV0 {}
// SAFETY:
//
// [`ModuleRequestV0`] does not interact with threads in any manner.
unsafe impl Sync for ModuleRequestV0 {}

/// Request for loaded modules.
///
/// Also allows specifying a number of internal modules to be loaded.
#[repr(C)]
#[derive(Debug)]
pub struct ModuleRequestV1 {
    /// Definition of the base revision's structure.
    pub base_revision: ModuleRequestV0,

    /// The number of [`InternalModule`]s passed by the executable.
    pub internal_module_count: u64,
    /// A pointer to an array of [`ModuleRequestV1::internal_module_count`] pointers to
    /// [`InternalModule`] structures.
    pub internal_modules: *mut *mut InternalModule,
}

// SAFETY:
//
// [`ModuleRequestV1`] does not interact with threads in any manner.
unsafe impl Send for ModuleRequestV1 {}
// SAFETY:
//
// [`ModuleRequestV1`] does not interact with threads in any manner.
unsafe impl Sync for ModuleRequestV1 {}

/// A module specified by the executable to be loaded.
#[repr(C)]
#[derive(Debug)]
pub struct InternalModule {
    /// The path to the module to loaded.
    ///
    /// This path is relative to the location of the executable.
    pub path: *const c_char,
    /// THe command line for the given module.
    pub command_line: *const c_char,
    /// Flags changing module loading behavior.
    pub flags: InternalModuleFlags,
}

// SAFETY:
//
// [`InternalModule`] does not interact with threads in any manner.
unsafe impl Send for InternalModule {}
// SAFETY:
//
// [`InternalModule`] does not interact with threads in any manner.
unsafe impl Sync for InternalModule {}

/// Flags that influence the behavior of an [`InternalModule`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalModuleFlags(u64);

impl InternalModuleFlags {
    /// Default flags.
    pub const DEFAULT: Self = Self(0);

    /// Fail if the requested module cannot be found.
    pub const REQUIRED: Self = Self(1);

    /// The module is GZ-compressed and should be decompressed by the bootloader.
    ///
    /// This is honored if the response is revision 2 or greater.
    #[deprecated]
    pub const COMPRESSED: Self = Self(2);
}

impl ops::BitOr for InternalModuleFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for InternalModuleFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl ops::BitAnd for InternalModuleFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for InternalModuleFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl ops::BitXor for InternalModuleFlags {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for InternalModuleFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl ops::Not for InternalModuleFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Response to an [`ModuleRequestV0`] or [`ModuleRequestV1`].
#[repr(C)]
#[derive(Debug)]
pub struct ModuleResponse {
    /// The revision of the [`ModuleRequestV0`] or [`ModuleRequestV1`].
    pub revision: u64,
    /// The number of modules present.
    pub module_count: u64,
    /// A pointer to an array of pointers to [`File`] structures specifying information about
    /// loaded modules.
    pub modules: *mut *mut File,
}

// SAFETY:
//
// [`ModuleResponse`] does not interact with threads in any manner.
unsafe impl Send for ModuleResponse {}
// SAFETY:
//
// [`ModuleResponse`] does not interact with threads in any manner.
unsafe impl Sync for ModuleResponse {}

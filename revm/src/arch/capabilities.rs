//! Architectural capability detection.
//!
//! This is a centralized mechanism for testing whether various architectural features or
//! capabilities are supported.

use core::fmt;

use sync::ControlledModificationCell;

use crate::arch::arch_impl;

/// The system [`ArchCapabilities`].
static ARCH_FEATURE_SUPPORT: ControlledModificationCell<ArchCapabilities> =
    ControlledModificationCell::new(ArchCapabilities(
        arch_impl::capabilities::ArchCapabilities::initial(),
    ));

/// Initialize the architectural capabilities.
///
/// # Safety
///
/// This must only be called a single time, as soon as possible.
pub unsafe fn initialize_arch_capability_support() {
    let arch_capability_support = arch_impl::capabilities::ArchCapabilities::new();

    // SAFETY:
    //
    // Since this function is only called a single time, as soon as possible, it is safe to mutate
    // this variable.
    unsafe { *ARCH_FEATURE_SUPPORT.get_mut() = ArchCapabilities(arch_capability_support) };
}

/// Returns the [`ArchCapabilities`] for the system.
pub fn arch_capability_support() -> &'static ArchCapabilities {
    ARCH_FEATURE_SUPPORT.get()
}

/// Validates that this processor's [`ArchCapabilities`] is the same as the stored [`ArchCapabilities`].
pub fn validate_arch_capabilities_match() -> bool {
    let stored_arch_capability_support = arch_capability_support();
    let arch_capability_support = arch_impl::capabilities::ArchCapabilities::new();

    arch_capability_support == stored_arch_capability_support.0
}

/// The architectural capabilities that are constant across cores.
pub struct ArchCapabilities(arch_impl::capabilities::ArchCapabilities);

impl core::ops::Deref for ArchCapabilities {
    type Target = arch_impl::capabilities::ArchCapabilities;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for ArchCapabilities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Debug for ArchCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <arch_impl::capabilities::ArchCapabilities as fmt::Debug>::fmt(&self.0, f)
    }
}

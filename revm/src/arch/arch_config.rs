//! Architectural configuration data.

use core::{error, fmt};

use sync::ControlledModificationCell;

use crate::arch::arch_impl;

/// The system [`ArchConfig`].
static ARCH_CONFIG: ControlledModificationCell<ArchConfig> =
    ControlledModificationCell::new(ArchConfig(arch_impl::arch_config::ArchConfig::initial()));

/// Initialize the architectural configuration.
///
/// # Errors
///
/// [`ArchConfigError`] is returned if there does not exist an architectural configuration of the
/// sytem that is supported by `revm`.
///
/// # Safety
///
/// This must only be called a single time, as soon as possible.
pub unsafe fn initialize_arch_config() -> Result<(), ArchConfigError> {
    let arch_config = arch_impl::arch_config::ArchConfig::new()?;

    // SAFETY:
    //
    // Since this function is only called a single time, as soon as possible, it is safe to mutate
    // this variable.
    unsafe { *ARCH_CONFIG.get_mut() = ArchConfig(arch_config) };

    Ok(())
}

/// Returns the [`ArchConfig`] for the system.
pub fn arch_config() -> &'static ArchConfig {
    ARCH_CONFIG.get()
}

/// Validates that this processor's [`ArchConfig`] is the same as the stored [`ArchConfig`].
pub fn validate_same() -> bool {
    let stored_arch_config = arch_config();
    #[expect(irrefutable_let_patterns)]
    let Ok(arch_config) = arch_impl::arch_config::ArchConfig::new() else {
        return false;
    };

    arch_config == stored_arch_config.0
}

/// The architectural  configuration that is constant across cores.
pub struct ArchConfig(arch_impl::arch_config::ArchConfig);

impl fmt::Debug for ArchConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <arch_impl::arch_config::ArchConfig as fmt::Debug>::fmt(&self.0, f)
    }
}

/// Various errors that may occur while gathering [`ArchConfig`] data.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ArchConfigError(arch_impl::arch_config::ArchConfigError);

impl From<arch_impl::arch_config::ArchConfigError> for ArchConfigError {
    fn from(value: arch_impl::arch_config::ArchConfigError) -> Self {
        Self(value)
    }
}

impl fmt::Debug for ArchConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <arch_impl::arch_config::ArchConfigError as fmt::Debug>::fmt(&self.0, f)
    }
}

impl fmt::Display for ArchConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <arch_impl::arch_config::ArchConfigError as fmt::Display>::fmt(&self.0, f)
    }
}

impl error::Error for ArchConfigError {
    fn cause(&self) -> Option<&dyn error::Error> {
        #[expect(deprecated)]
        <arch_impl::arch_config::ArchConfigError as error::Error>::cause(&self.0)
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        <arch_impl::arch_config::ArchConfigError as error::Error>::source(&self.0)
    }

    fn description(&self) -> &str {
        #[expect(deprecated)]
        <arch_impl::arch_config::ArchConfigError as error::Error>::description(&self.0)
    }
}

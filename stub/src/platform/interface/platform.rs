//! Boot platform introspection.

use core::sync::atomic::{AtomicU8, Ordering};

/// The [`BootProtocol`] with which this executable was booted.
static BOOT_PROTOCOL: AtomicU8 = AtomicU8::new(0xFF);

/// Sets the [`BootProtocol`] with which this executable was booted.
///
/// This is used to centralize introspection about the platform capabilities and restrictions.
///
/// # Safety
///
/// `protocol` must correspond with the [`BootProtocol`] with which this executable was actually
/// booted.
pub(in crate::platform) unsafe fn set_protocol(protocol: BootProtocol) {
    BOOT_PROTOCOL.store(protocol.to_u8(), Ordering::Relaxed);
}

/// Returns the [`BootProtocol`] with which this executable was booted.
pub fn get_protocol() -> BootProtocol {
    BootProtocol::from_u8(BOOT_PROTOCOL.load(Ordering::Relaxed))
}

/// The various boot protocols that can exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootProtocol {
    /// The binary was booted using the Linux boot protocol for this architecture.
    #[cfg(CONFIG_STUB_PLATFORM_LINUX)]
    Linux,

    /// The binary was booted from a UEFI firmware implementation.
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    Uefi,
}

impl BootProtocol {
    /// Converts the provided [`BootProtocol`] into a raw representation.
    fn to_u8(self) -> u8 {
        match self {
            #[cfg(CONFIG_STUB_PLATFORM_LINUX)]
            Self::Linux => 0,
            #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
            Self::Uefi => 1,
        }
    }

    /// Converts the provided `val` into its associated [`BootProtocol`].
    fn from_u8(val: u8) -> Self {
        match val {
            #[cfg(CONFIG_STUB_PLATFORM_LINUX)]
            0 => Self::Linux,
            #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
            1 => Self::Uefi,
            _ => unreachable!(),
        }
    }
}

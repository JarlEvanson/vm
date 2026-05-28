//! Support for booting using the Linux boot protocol.

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86")]
mod i686;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::linux_main;
#[cfg(target_arch = "x86")]
pub use i686::linux_main;
#[cfg(target_arch = "x86_64")]
pub use x86_64::linux_main;

//! # `revm`
//!
//! `revm` is a platform for hardware probing, remote debugging, and black-box reverse engineering.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod log;
pub mod stub_protocol;
pub mod util;

/// Generic handler for panics.
#[panic_handler]
#[cfg(not(test))]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

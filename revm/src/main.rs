//! # `revm`
//!
//! `revm` is a platform for hardware probing, remote debugging, and black-box reverse engineering.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

/// Generic handler for panics.
#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

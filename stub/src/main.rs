//! The first stage loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod platform;

/// Generic handler for panics.
#[panic_handler]
#[cfg(not(test))]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

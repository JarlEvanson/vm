//! The loader for the `revm` platform.
#![no_std]
#![no_main]

pub mod platform;

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

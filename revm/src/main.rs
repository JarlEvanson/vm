//! # `revm`
//!
//! `revm` is a platform for hardware probing, remote debugging, and black-box reverse engineering.
#![no_std]
#![no_main]

mod stub_protocol;

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

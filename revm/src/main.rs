//! # `revm`
//!
//! `revm` is a platform for hardware probing, remote debugging, and black-box reverse engineering.
#![no_std]
#![no_main]

mod log;
mod stub_protocol;
mod util;

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

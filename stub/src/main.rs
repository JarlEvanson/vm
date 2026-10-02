//! The loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod cmd_line;
mod driver;
mod platform;

/// Generic handler for panics.
#[cfg(not(test))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    loop {}
}

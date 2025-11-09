//! The first stage loader for the `revm` platform.

#![no_std]
#![no_main]

use crate::boot::Context;

mod boot;

fn stub_main(context: &mut Context) {
    todo!()
}

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

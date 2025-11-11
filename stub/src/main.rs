//! The first stage loader for the `revm` platform.

#![no_std]
#![no_main]

use sync::Spinlock;

use crate::boot::Context;

mod boot;
mod graphics;

fn stub_main(context: &mut Context) {
    todo!()
}

static PANIC_FUNC: Spinlock<fn(&core::panic::PanicInfo) -> !> = Spinlock::new(fallback);

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    PANIC_FUNC.lock()(info)
}

fn fallback(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

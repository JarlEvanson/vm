//! The loader for the `revm` platform.
#![no_std]
#![no_main]

use sync::Spinlock;

pub mod arch;
pub mod platform;
pub mod util;

/// The platform-specific panic handler function.
static PANIC_HANDLER: Spinlock<fn(&core::panic::PanicInfo) -> !> = Spinlock::new(fallback);

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    let lock = PANIC_HANDLER.lock();
    let handler = *lock;
    drop(lock);

    handler(info)
}

/// The panic handler function utilized if no other panic handler is assigned.
fn fallback(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

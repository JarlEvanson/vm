//! The first stage loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use sync::Spinlock;

pub mod platform;

/// The platform-specific panic handler function.
static PANIC_FUNC: Spinlock<fn(&core::panic::PanicInfo) -> !> = Spinlock::new(fallback);

/// Generic handler for panics.
#[panic_handler]
#[cfg(not(test))]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    PANIC_FUNC.lock()(info)
}

/// The panic handler function utilized if no other panic handler is assigned.
fn fallback(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

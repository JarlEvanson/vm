//! The loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod cmd_line;
mod driver;
mod graph;
mod platform;
mod util;

define_node!(
    TEST_NODE,
    "Test",
    active,
    type = (),
    required = [],
    required_by = [],
    wanted = [],
    wanted_by = []
);

/// Generic handler for panics.
#[cfg(not(test))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    loop {}
}

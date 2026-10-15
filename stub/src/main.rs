//! The loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub mod graph;
pub mod platform;
pub mod util;

define_node!(
    C,
    "C",
    type = (),
    None,
    active = true,
    extended_lifetime = false,
    required_enabled = [B],
    required_disabled = [B],
    required_by_enabled = [],
    required_by_disabled = [],
);

static A: graph::GraphNode<()> = make_node!(
    "A",
    None,
    active = true,
    extended_lifetime = false,
    required_enabled = [B],
    required_disabled = [B],
    required_by_enabled = [],
    required_by_disabled = [],
);

static B: graph::GraphNode<()> = make_node!(
    "B",
    None,
    active = true,
    extended_lifetime = false,
    required_enabled = [],
    required_disabled = [],
    required_by_enabled = [],
    required_by_disabled = [],
);

/// Generic handler for panics.
#[cfg(not(test))]
#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

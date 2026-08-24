//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Other support modules.

#[cfg(CONFIG_STUB_SELF_FRAME_ALLOCATOR)]
mod frame_allocator;
#[cfg(CONFIG_STUB_GRAPHICS)]
mod graphics;
#[cfg(CONFIG_STUB_CUSTOM_HEAP_ALLOCATOR)]
mod heap_allocator;
mod interface;
mod relocate;

pub use interface::*;

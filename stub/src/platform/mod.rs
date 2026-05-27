//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Other support modules.

mod frame_allocator;
mod generic;
mod graphics;
mod heap_allocator;

pub use generic::*;

//! Definitions and interfaces that platforms utilize to provide services for use by the rest of
//! the executable.

mod memory;
mod platform_tables;

pub use memory::*;
pub use platform_tables::*;

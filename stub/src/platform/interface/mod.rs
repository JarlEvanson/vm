//! Definitions and interfaces that platforms utilize to provide services for use by the rest of
//! the executable.

mod logging;
mod memory;
mod processor;
mod tables;

pub use logging::*;
pub use memory::*;
pub use processor::*;
pub use tables::*;

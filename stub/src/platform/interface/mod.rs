//! Definitions and interfaces that platforms utilize to provide services for use by the rest of
//! the executable.

mod logging;
mod platform;

pub use logging::*;
pub use platform::*;

//! Collection of supported boot protocols and utilities for carrying out boot operations.

mod context;
mod relocation;

pub use context::{AllocationPolicy, Context, FailedMapping, NotFound, OutOfMemory};

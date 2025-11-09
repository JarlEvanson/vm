//! Collection of supported boot protocols and utilities for carrying out boot operations.

mod context;
mod relocation;

mod uefi;

pub use context::{AllocationPolicy, Context, FailedMapping, NotFound, OutOfMemory};

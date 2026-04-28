//! Library of various synchronization methods.

#![no_std]

mod controlled_modification;
mod spinlock;

pub use controlled_modification::ControlledModificationCell;
pub use spinlock::{RawSpinlock, Spinlock, SpinlockAcquisitionError, SpinlockGuard};

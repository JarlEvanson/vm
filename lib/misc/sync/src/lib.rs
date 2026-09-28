//! Library of various synchronization methods.

#![no_std]

mod controlled_modification;
mod rwspinlock;
mod spinlock;

pub use controlled_modification::ControlledModificationCell;
pub use rwspinlock::{
    RawRwSpinlock, RwSpinlock, RwSpinlockReadAcquisitionError, RwSpinlockReadGuard,
    RwSpinlockWriteAcquisitionError, RwSpinlockWriteGuard,
};
pub use spinlock::{RawSpinlock, Spinlock, SpinlockAcquisitionError, SpinlockGuard};

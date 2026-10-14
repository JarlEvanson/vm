//! Various miscellaneous utility structures and functions.

use core::ptr::NonNull;

/// A [`Send`] and [`Sync`] [`NonNull`] used for directly exposed pointer types.
pub struct SendSyncNonNull<T: Send + Sync>(pub NonNull<T>);

// SAFETY:
//
// All interactions with [`SendSyncNonNull`] require `unsafe` operations to violate safety
// constraints.
unsafe impl<T: Send + Sync> Send for SendSyncNonNull<T> {}
// SAFETY:
//
// All interactions with [`SendSyncNonNull`] require `unsafe` operations to violate safety
// constraints.
unsafe impl<T: Send + Sync> Sync for SendSyncNonNull<T> {}

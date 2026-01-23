//! The `pe` crate provides an interface for reading PE files.
//!
//! # Capabilities
//!
//! ## Works in `no_std` environments
//!
//! This crate provides a PE file parsing interface which does not allocate or use any `std`
//! features, so it can be used in `no_std` contexts such as bootloaders, kernels, or hypervisors.
//!
//! ## Zero-Alloc parsing
//!
//! This crate implements parsing in such a manner that avoids heap allocations. PE structures are
//! lazily parsing with iterators or tables that only parse the requested structure when required.
//!
//! ## Uses no unsafe code
//!
//! This crate contains zero unsafe blocks of code.

#![no_std]

pub mod raw;

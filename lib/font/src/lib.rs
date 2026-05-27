//! Defines the implementation and interfaces of the `revm-stub` font structures.
//!
//! Includes both read-only and writable interfaces.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod font_map;
pub mod glyph;

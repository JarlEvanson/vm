//! Implementation of `compiler-builtins` for `revm` and `revm-stub`.

#![cfg_attr(not(test), allow(internal_features))]
#![cfg_attr(not(test), feature(compiler_builtins))]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_builtins)]
#![cfg_attr(not(test), compiler_builtins)]

mod intrinsics;
mod mem;

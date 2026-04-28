//! Build script for `revm-stub`.

use std::env;

fn main() -> Result<(), ()> {
    if let Ok(val) = env::var("CARGO_CFG_PANIC")
        && val == "unwind"
    {
        // `revm-stub` is never built with unwinding panics except for when testing.
        return Ok(());
    }

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo didn't pass CARGO_MANIFEST_DIR");

    println!("cargo::rustc-link-arg=-T{manifest_dir}/linker-script.ld");

    Ok(())
}

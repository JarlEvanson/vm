//! Build script for `revm-stub`.

fn main() -> Result<(), ()> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo didn't pass CARGO_MANIFEST_DIR");

    println!("cargo::rustc-link-arg=-T{manifest_dir}/linker-script.ld");

    Ok(())
}

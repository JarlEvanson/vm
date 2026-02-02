//! Build script for `revm-stub`.

use std::{
    env,
    fs::{self, File},
    path::PathBuf,
};

/// The location of the default font.
const DEFAULT_FONT: &str = "../assets/Tamsyn8x16r.psf";

fn main() -> Result<(), ()> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("cargo didn't pass CARGO_MANIFEST_DIR");

    println!("cargo::rustc-link-arg=-T{manifest_dir}/linker-script.ld");

    let font_path = if let Some(font_path) = env::var_os("STUB_FONT") {
        PathBuf::from(font_path)
    } else {
        PathBuf::from(DEFAULT_FONT)
    };

    let font = fs::read(font_path).expect("failed to load specified font");
    let glyph_array =
        File::create("../target/glyph_array.bin").expect("failed to create glyph_array.bin");
    let font_map = File::create("../target/font_map.bin").expect("failed to create font_map.bin");

    font_converter::convert_psf(font.as_slice(), glyph_array, font_map)
        .expect("failed to convert PSF file");

    Ok(())
}

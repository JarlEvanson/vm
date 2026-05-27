//! Tool for converting various bitmap font formats into the custom [`GlyphArray`] and
//! [`FontMapBuilder`] dump formats.

use std::{env::args, fs::File, path::Path};

use anyhow::Result;

fn main() -> Result<()> {
    let font_path = args().nth(1).expect("expected font path");
    let dir_path = args().nth(2).expect("expected output directory path");
    let dir_path = Path::new(dir_path.as_str());

    let font = std::fs::read(font_path).expect("error reading provided font");

    let glyph_array = File::create(dir_path.join("glyph_array.bin")).unwrap();
    let font_map = File::create(dir_path.join("font_map.bin")).unwrap();

    font_converter::convert_psf(font.as_slice(), glyph_array, font_map)
}

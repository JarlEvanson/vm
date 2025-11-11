//! Tool for converting various bitmap font formats into the custom [`GlyphArray`] and [`FontMap`]
//! dump formats.

use core::{error, fmt};
use std::io::Write;

use anyhow::Result;
use font::{font_map::FontMapBuilder, glyph::GlyphArray};

/// Parses `font` as a PSF file and writes its corresponding [`GlyphArray`] to `glyph_writer`
/// and the [`FontMap`][fm] to `font_map_writer`.
///
/// [fm]: font::font_map::FontMap
pub fn convert_psf<W0: Write, W1: Write>(
    font: &[u8],
    glyph_writer: W0,
    font_map_writer: W1,
) -> Result<()> {
    if font.len() < 4 {
        return Err(PsfError::TruncatedHeader {
            actual_size: font.len(),
            expected_size: 4,
        }
        .into());
    }

    let psf_2;
    let glyph_start;
    let glyph_count;
    let glyph_size;
    let glyph_height;
    let glyph_width;
    let has_unicode_table;

    if font[..2] == [0x36, 0x04] {
        // PSF 1
        let mode = font[2];
        let _glyph_size = font[3];

        psf_2 = false;
        glyph_start = 4;
        glyph_count = 256 * (1 + (mode & 0x01 == 0x01) as u32);
        glyph_size = u32::from(_glyph_size);
        glyph_height = u32::from(glyph_size);
        glyph_width = 8;
        has_unicode_table = (mode & 0x02 == 0x02) || (mode & 04 == 0x4);
    } else if font[..4] == [0x72, 0xb5, 0x4a, 0x86] {
        // PSF 2
        if font.len() < 4 * 8 {
            return Err(PsfError::TruncatedHeader {
                actual_size: font.len(),
                expected_size: 32,
            }
            .into());
        }

        let version = parse_u32(font, 4).unwrap();
        if version != 0 {
            anyhow::bail!(PsfError::InvalidVersion(version));
        }

        let header_size = parse_u32(font, 8).unwrap();
        let flags = parse_u32(font, 12).unwrap();
        let length = parse_u32(font, 16).unwrap();
        let _glyph_size = parse_u32(font, 20).unwrap();
        let height = parse_u32(font, 24).unwrap();
        let width = parse_u32(font, 28).unwrap();

        psf_2 = true;
        glyph_start = header_size;
        glyph_count = length;
        glyph_size = _glyph_size;
        glyph_height = height;
        glyph_width = width;
        has_unicode_table = flags & 0x00000001 == 0x00000001;
    } else {
        anyhow::bail!(PsfError::InvalidMagic(parse_u32(font, 0).unwrap()));
    }

    let total_size = glyph_count.strict_mul(glyph_size);
    let max_offset = glyph_height.strict_add(total_size);
    let max_offset_usize = usize::try_from(max_offset)?;
    if font.len() < max_offset_usize {
        anyhow::bail!(PsfError::TruncatedData {
            actual_size: font.len(),
            expected_size: max_offset
        });
    }

    let glyph_start_usize = usize::try_from(glyph_start)?;
    let glyph_slice = &font[glyph_start_usize..max_offset_usize];
    let width = u8::try_from(glyph_width)?;
    let height = u8::try_from(glyph_height)?;
    let glyph_array = GlyphArray::new(glyph_slice, width, height);

    let mut font_map;
    let unicode_table_array_start = max_offset_usize;
    if has_unicode_table {
        let base_iter;
        if psf_2 {
            base_iter = Psf1Iter(&[]).chain(Psf2Iter(&font[unicode_table_array_start..]));
        } else {
            base_iter = Psf1Iter(&font[unicode_table_array_start..]).chain(Psf2Iter(&[]));
        }

        let mut iter = base_iter.clone();
        let mut mapping_count = 0;
        'overall: for _ in 0..glyph_count {
            loop {
                let behavior = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("unexpected end to unicode table"))??;
                match behavior {
                    Behavior::Char(_) => mapping_count += 1,
                    Behavior::BeginSeries => break,
                    Behavior::EndEntry => continue 'overall,
                }
            }

            loop {
                let behavior = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("unexpected end to unicode table"))??;
                match behavior {
                    Behavior::Char(_) => continue,
                    Behavior::BeginSeries => continue,
                    Behavior::EndEntry => continue 'overall,
                }
            }
        }

        if iter.next().is_some() {
            return Err(anyhow::anyhow!("unicode table has too many entries"));
        }

        iter = base_iter;
        font_map = FontMapBuilder::new((mapping_count * 3) / 2);
        'overall: for glyph_index in 0..glyph_count {
            loop {
                let behavior = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("unexpected end to unicode table"))??;
                match behavior {
                    Behavior::Char(c) => font_map.insert(c, glyph_index).unwrap(),
                    Behavior::BeginSeries => break,
                    Behavior::EndEntry => continue 'overall,
                }
            }

            loop {
                let behavior = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("unexpected end to unicode table"))??;
                match behavior {
                    Behavior::Char(_) => continue,
                    Behavior::BeginSeries => continue,
                    Behavior::EndEntry => continue 'overall,
                }
            }
        }
    } else {
        let glyph_count_usize = usize::try_from(glyph_count)?;
        font_map = FontMapBuilder::new((glyph_count_usize * 3) / 2);
        for i in 0..glyph_count {
            let c = char::from_u32(i).ok_or(anyhow::anyhow!("unsupported char"))?;
            font_map.insert(c, i).unwrap();
        }
    }

    glyph_array.dump(glyph_writer);
    font_map.dump(font_map_writer, true);
    Ok(())
}

fn parse_u16(slice: &[u8], offset: usize) -> Option<u16> {
    let Some(_) = offset.checked_add(4) else {
        return None;
    };

    let Some(bytes) = slice.split_at(offset).1.first_chunk() else {
        return None;
    };

    Some(u16::from_le_bytes(*bytes))
}

fn parse_u32(slice: &[u8], offset: usize) -> Option<u32> {
    let Some(_) = offset.checked_add(4) else {
        return None;
    };

    let Some(bytes) = slice.split_at(offset).1.first_chunk() else {
        return None;
    };

    Some(u32::from_le_bytes(*bytes))
}

enum Behavior {
    Char(char),
    BeginSeries,
    EndEntry,
}

#[derive(Clone)]
struct Psf1Iter<'slice>(&'slice [u8]);

impl Iterator for Psf1Iter<'_> {
    type Item = Result<Behavior>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = parse_u16(self.0, 0)?;
        match value {
            0xFFFE => {
                self.0 = self.0.get(2..)?;
                Some(Ok(Behavior::BeginSeries))
            }
            0xFFFF => {
                self.0 = self.0.get(2..)?;
                Some(Ok(Behavior::EndEntry))
            }
            _ => {
                let c = match char::from_u32(u32::from(value)) {
                    Some(c) => c,
                    None => return Some(Err(anyhow::anyhow!("unsupported char"))),
                };

                self.0 = self.0.get(2..)?;
                Some(Ok(Behavior::Char(c)))
            }
        }
    }
}

#[derive(Clone)]
struct Psf2Iter<'slice>(&'slice [u8]);

impl Iterator for Psf2Iter<'_> {
    type Item = Result<Behavior>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.0.get(0).copied()?;
        match value {
            0xFE => {
                self.0 = self.0.get(1..)?;
                Some(Ok(Behavior::BeginSeries))
            }
            0xFF => {
                self.0 = self.0.get(1..)?;
                Some(Ok(Behavior::EndEntry))
            }
            _ => {
                let utf8_len = match value {
                    value if value & 0b1000_0000 == 0 => 1,
                    value if value & 0b1110_0000 == 0b1100_0000 => 2,
                    value if value & 0b1111_0000 == 0b1110_0000 => 3,
                    value if value & 0b1111_1000 == 0b1111_0000 => 4,
                    _ => return Some(Err(anyhow::anyhow!("unsupported char"))),
                };
                if self.0.len() < utf8_len {
                    return Some(Err(anyhow::anyhow!("unexpected end to unicode table")));
                }

                let c = match str::from_utf8(&self.0[..utf8_len]) {
                    Ok(c) => c,
                    Err(err) => return Some(Err(err.into())),
                };
                let c = c.chars().next()?;

                self.0 = self.0.get(utf8_len..)?;
                Some(Ok(Behavior::Char(c)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PsfError {
    InvalidMagic(u32),
    InvalidVersion(u32),
    TruncatedHeader {
        actual_size: usize,
        expected_size: u32,
    },
    TruncatedData {
        actual_size: usize,
        expected_size: u32,
    },
}

impl fmt::Display for PsfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic(magic) => write!(f, "invalid magic: {magic:08X}"),
            Self::InvalidVersion(version) => write!(f, "invalid version: {version}"),
            Self::TruncatedHeader {
                actual_size,
                expected_size,
            } => write!(
                f,
                "header is truncated: expected {expected_size} bytes but got {actual_size} bytes"
            ),
            Self::TruncatedData {
                actual_size,
                expected_size,
            } => write!(
                f,
                "data is truncated: expected {expected_size} bytes but got {actual_size} bytes"
            ),
        }
    }
}

impl error::Error for PsfError {}

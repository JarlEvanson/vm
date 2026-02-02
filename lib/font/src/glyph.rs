//! Interface for interacting with glyphs.

#[cfg(feature = "std")]
use std::io::Write;

/// An array of [`Glyph`]s.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphArray<'buffer> {
    /// The buffer that contains the glyph data.
    buffer: &'buffer [u8],
    /// The width of each glyph.
    width: u8,
    /// The height of each glyph.
    height: u8,
}

impl<'buffer> GlyphArray<'buffer> {
    /// Creates a new [`GlyphArray`].
    pub const fn new(buffer: &'buffer [u8], width: u8, height: u8) -> Self {
        Self {
            buffer,
            width,
            height,
        }
    }

    /// Creates a new [`GlyphArray`] from a dumped blob.
    #[expect(clippy::missing_panics_doc, reason = "splitting is safe")]
    pub const fn from_dump(dump: &'buffer [u8]) -> Option<Self> {
        if dump.len() < 2 {
            return None;
        }

        let glyph_array = Self {
            buffer: dump.split_first().unwrap().1.split_first().unwrap().1,
            width: dump[0],
            height: dump[1],
        };
        Some(glyph_array)
    }

    /// Returns the [`Glyph`] at `index` or `None` if out of bounds.
    pub fn get(&self, index: usize) -> Option<Glyph<'buffer>> {
        if index >= self.glyph_count() {
            return None;
        }

        let row_byte_count = usize::from(self.width.div_ceil(8));
        let glyph_byte_count = row_byte_count * usize::from(self.height);

        let glyph = Glyph {
            buffer: &self.buffer[index * glyph_byte_count..],
            width: self.width,
            height: self.height,
        };
        Some(glyph)
    }

    /// Returns the width of a [`Glyph`] in pixels.
    pub const fn width(&self) -> u8 {
        self.width
    }

    /// Returns the height of a [`Glyph`] in pixels.
    pub const fn height(&self) -> u8 {
        self.height
    }

    /// Returns the number of [`Glyph`]s in this [`GlyphArray`].
    #[expect(clippy::as_conversions)]
    pub const fn glyph_count(&self) -> usize {
        let row_byte_count = self.width.div_ceil(8) as usize;
        let glyph_byte_count = row_byte_count * self.height as usize;

        self.buffer.len() / glyph_byte_count
    }

    /// Dumps the [`GlyphArray`] into the `writer`.
    ///
    /// # Panics
    ///
    /// Panics if writing panics.
    #[cfg(feature = "std")]
    pub fn dump<W: Write>(&self, mut writer: W) {
        writer.write_all(&[self.width]).unwrap();
        writer.write_all(&[self.height]).unwrap();
        writer.write_all(self.buffer).unwrap();
    }
}

/// Stores the on/off layout of a specific glyph in a font.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Glyph<'buffer> {
    /// The buffer utilized to store the glyph.
    buffer: &'buffer [u8],
    /// The width of each glyph.
    width: u8,
    /// The height of each glyph.
    height: u8,
}

impl<'buffer> IntoIterator for Glyph<'buffer> {
    type IntoIter = GlyphRowsIter<'buffer>;
    type Item = GlyphRow<'buffer>;

    fn into_iter(self) -> Self::IntoIter {
        GlyphRowsIter {
            buffer: self.buffer,
            width: self.width,
            height: self.height,
            index: 0,
        }
    }
}

/// An [`Iterator`] over the rows of a [`Glyph`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphRowsIter<'buffer> {
    /// The buffer utilized to store the glyph.
    buffer: &'buffer [u8],
    /// The width of each glyph.
    width: u8,
    /// The height of each glyph.
    height: u8,
    /// The index of the row that will be returned next.
    index: u8,
}

impl<'buffer> Iterator for GlyphRowsIter<'buffer> {
    type Item = GlyphRow<'buffer>;

    #[expect(clippy::as_conversions)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.height {
            return None;
        }

        let row_byte_count = self.width.div_ceil(8) as usize;
        let row_index = row_byte_count * self.index as usize;

        self.index += 1;
        let row = GlyphRow {
            buffer: &self.buffer[row_index..],
            width: self.width,
        };
        Some(row)
    }
}

/// A row in the [`Glyph`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphRow<'buffer> {
    /// The buffer utilized to store the glyph row.
    buffer: &'buffer [u8],
    /// The width of the row.
    width: u8,
}

impl<'buffer> IntoIterator for GlyphRow<'buffer> {
    type Item = bool;
    type IntoIter = GlyphRowIter<'buffer>;

    fn into_iter(self) -> Self::IntoIter {
        GlyphRowIter {
            buffer: self.buffer,
            width: self.width,
            index: 0,
        }
    }
}

/// An [`Iterator`] over the pixels in a [`GlyphRow`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlyphRowIter<'buffer> {
    /// The buffer used to store the glpyh's rows.
    buffer: &'buffer [u8],
    /// The width of each row.
    width: u8,
    /// The index of the pixel value to be returned.
    index: u8,
}

impl Iterator for GlyphRowIter<'_> {
    type Item = bool;

    #[expect(clippy::as_conversions)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.width {
            return None;
        }

        let byte_index = (self.index / 8) as usize;
        let bit_index = (self.index % 8) as usize;
        let bit = (self.buffer[byte_index] >> (7 - bit_index)) & 0b1;

        self.index += 1;
        Some(bit == 1)
    }
}

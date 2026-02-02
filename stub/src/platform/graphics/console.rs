//! Implementation of an output-only graphical console to be used for logging.

use core::fmt;

use font::{font_map::FontMap, glyph::GlyphArray};

use crate::platform::graphics::surface::{Point, Region, Surface};

/// Text-based graphical output device.
pub struct Console<'font, S: Surface> {
    /// The x position of the next character to be printed.
    x: usize,
    /// The y position of the next character to be printed.
    y: usize,

    /// The width of the character array.
    text_width: usize,
    /// The height of the character array.
    text_height: usize,

    /// The color of the foreground.
    foreground: u32,
    /// The color of the background.
    background: u32,

    /// The [`Surface`] on which this console writes.
    surface: S,

    /// The [`GlyphArray`] containing all glyphs used by the [`Console`].
    glyph_array: GlyphArray<'font>,
    /// The map of [`char`]s to glyphs.
    font_map: FontMap<'font>,
}

impl<'font, S: Surface> Console<'font, S> {
    /// Creates a new [`Console`] that prints characters using the given [`GlyphArray`] and
    /// [`FontMap`] onto the given [`Surface`].
    pub fn new(
        surface: S,
        glyph_array: GlyphArray<'font>,
        font_map: FontMap<'font>,
        foreground: u32,
        background: u32,
    ) -> Self {
        let text_width = surface.width() / glyph_array.width() as usize;
        let text_height = surface.height() / glyph_array.height() as usize;

        Self {
            x: 0,
            y: 0,

            text_width,
            text_height,

            foreground,
            background,

            surface,

            glyph_array,
            font_map,
        }
    }

    /// Writes the given [`char`] to the [`Surface`].
    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.new_line(),
            '\r' => self.carriage_return(),
            c => {
                if self.x + 1 >= self.text_width {
                    self.new_line();
                }

                let Some(glyph_index) = self.font_map.get(c) else {
                    self.x += 1;
                    return;
                };
                let Some(glyph) = self.glyph_array.get(glyph_index as usize) else {
                    return;
                };

                let x_base = self.x * self.glyph_array.width() as usize;
                let y_base = self.y * self.glyph_array.height() as usize;

                for (y_offset, row) in glyph.into_iter().enumerate() {
                    for (x_offset, pixel_on) in row.into_iter().enumerate() {
                        let color = if pixel_on {
                            self.foreground
                        } else {
                            self.background
                        };

                        self.surface
                            .write_pixel(
                                Point {
                                    x: x_base + x_offset,
                                    y: y_base + y_offset,
                                },
                                color,
                            )
                            .unwrap();
                    }
                }

                self.x += 1;
            }
        }
    }

    /// Scrolls the [`Console`] a single line.
    fn scroll(&mut self) {
        // The point to start copying from (one line down).
        let source_point = Point {
            x: 0,
            y: usize::from(self.glyph_array.height()),
        };

        // The region to copy into.
        let copy_height = (self.text_height - 1) * usize::from(self.glyph_array.height());
        let write = Region {
            point: Point { x: 0, y: 0 },
            width: self.surface.width(),
            height: copy_height,
        };

        self.surface.copy_within(write, source_point).unwrap();

        let fill_region = Region {
            point: Point {
                x: 0,
                y: copy_height,
            },
            width: self.surface.width(),
            height: self.glyph_array.height() as usize,
        };
        self.surface.fill(fill_region, self.background).unwrap();
    }

    /// Handles new lines (with an included carriage return).
    ///
    /// This also handles scrolling if necessary.
    fn new_line(&mut self) {
        self.carriage_return();
        self.y += 1;

        if self.y + 1 >= self.text_height {
            self.y -= 1;
            self.scroll();
        }
    }

    /// Handles carriage returns (sets `x` to 0).
    const fn carriage_return(&mut self) {
        self.x = 0;
    }
}

impl<S: Surface> fmt::Write for Console<'_, S> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c)
        }

        Ok(())
    }
}

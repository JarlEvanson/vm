//! [`Surface`] defines the basic interface used to interact with all graphical output devices.
//!
//! Pixels are interacted with as RGBA8 and any other pixel formats must be translated inside the
//! [`Surface`] implementation.

use core::{error, fmt};

/// Abstraction over the basic interface used to interact with all graphical output devices.
///
/// # Safety
///
/// If the provided points/regions are within bounds, all operations must be safe.
pub unsafe trait Surface {
    /// The with of the [`Surface`] in pixels.
    ///
    /// This value does not change between calls.
    fn width(&self) -> usize;

    /// The height of the [`Surface`] in pixels.
    ///
    /// This value does not change between calls.
    fn height(&self) -> usize;

    /// Writes `value` to the pixel at [`Point`].
    ///
    /// # Safety
    ///
    /// [`Point`] is in the bounds of this [`Surface`].
    unsafe fn write_pixel_unchecked(&mut self, point: Point, value: u32);

    /// Reads the value of the pixel at [`Point`].
    ///
    /// # Safety
    ///
    /// [`Point`] is in the bounds of this [`Surface`].
    unsafe fn read_pixel_unchecked(&self, point: Point) -> u32;

    /// Writes `value` to the pixel at [`Point`].
    ///
    /// # Errors
    ///
    /// [`OutOfBoundsError`] is returned when the specified [`Point`] is out of bounds.
    fn write_pixel(&mut self, point: Point, value: u32) -> Result<(), OutOfBoundsError> {
        if !point_in_bounds(point, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        // SAFETY:
        //
        // `point` is in the bounds of the [`Surface`].
        unsafe { self.write_pixel_unchecked(point, value) }

        Ok(())
    }

    /// Reads the value of the pixel at [`Point`].
    ///
    /// # Errors
    ///
    /// [`OutOfBoundsError`] is returned when the specified [`Point`] is out of bounds.
    fn read_pixel(&mut self, point: Point) -> Result<u32, OutOfBoundsError> {
        if !point_in_bounds(point, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        // SAFETY:
        //
        // `point` is in the bounds of the [`Surface`].
        let value = unsafe { self.read_pixel_unchecked(point) };
        Ok(value)
    }

    /// Fills the given [`Region`] with the given pixel `value`.
    ///
    /// # Errors
    ///
    /// [`OutOfBoundsError`] is returned when the specified [`Region`] is out of bounds.
    fn fill(&mut self, region: Region, value: u32) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(region, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        for y_offset in 0..region.height {
            for x_offset in 0..region.width {
                let point = Point {
                    x: region.point.x + x_offset,
                    y: region.point.y + y_offset,
                };

                // SAFETY:
                //
                // `point` is in the bounds of the [`Surface`].
                unsafe { self.write_pixel_unchecked(point, value) }
            }
        }

        Ok(())
    }

    /// Writes data from the buffer starting at [`Point`] into the given [`Region`] on the
    /// [`Surface`].
    ///
    /// `buffer_stride` is the number of [`u32`]s in a row in the `buffer`.
    ///
    /// # Errors
    ///
    /// - [`OutOfBoundsError`]: Returned when the specified `region` is out of bounds or if
    ///   the buffer region is out of bounds.
    fn write_to(
        &mut self,
        region: Region,
        source: Point,
        buffer_stride: usize,
        buffer: &[u32],
    ) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(region, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let buffer_height = buffer.len() / buffer_stride;
        let buffer_region = Region {
            point: source,
            width: region.width,
            height: region.height,
        };
        if !region_in_bounds(buffer_region, buffer_stride, buffer_height) {
            return Err(OutOfBoundsError);
        }

        let mut offset = source.x + source.y * buffer_stride;
        for y_offset in 0..region.height {
            for x_offset in 0..region.width {
                let point = Point {
                    x: region.point.x + x_offset,
                    y: region.point.y + y_offset,
                };

                // SAFETY:
                //
                // `point` is in the bounds of the [`Surface`].
                unsafe { self.write_pixel_unchecked(point, buffer[offset]) }
                offset += 1;
            }

            offset += buffer_stride - region.width;
        }

        Ok(())
    }

    /// Reads data from the given [`Region`] in the [`Surface`] into the buffer starting at
    /// [`Point`].
    ///
    /// `buffer_stride` is the number of [`u32`]s in a row in the `buffer`.
    ///
    /// # Errors
    ///
    /// - [`OutOfBoundsError`]: Returned when the specified `region` is out of bounds or if
    ///   the buffer region is out of bounds.
    fn read_from(
        &self,
        region: Region,
        destination: Point,
        buffer_stride: usize,
        buffer: &mut [u32],
    ) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(region, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let buffer_height = buffer.len() / buffer_stride;
        let buffer_region = Region {
            point: destination,
            width: region.width,
            height: region.height,
        };
        if !region_in_bounds(buffer_region, buffer_stride, buffer_height) {
            return Err(OutOfBoundsError);
        }

        let mut offset = destination.x + destination.y * buffer_stride;
        for y_offset in 0..region.height {
            for x_offset in 0..region.width {
                let point = Point {
                    x: region.point.x + x_offset,
                    y: region.point.y + y_offset,
                };

                // SAFETY:
                //
                // `point` is in the bounds of the [`Surface`].
                buffer[offset] = unsafe { self.read_pixel_unchecked(point) };
                offset += 1;
            }

            offset += buffer_stride - region.width;
        }

        Ok(())
    }

    /// Copies the pixels from `source` to the given [`Region`] `write`.
    ///
    /// # Errors
    ///
    /// - [`OutOfBoundsError`]: Returned when the specified `region` is out of bounds or if
    ///   the buffer region is out of bounds.
    fn copy_within(&mut self, write: Region, source: Point) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(write, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let read_region = Region {
            point: source,
            width: write.width,
            height: write.height,
        };
        if !region_in_bounds(read_region, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let (write_base_y, source_base_y, y_offset_addend) = if write.point.y <= source.y {
            (write.point.y, source.y, 1)
        } else {
            (
                write.point.y + write.height - 1,
                source.y + write.height - 1,
                -1,
            )
        };
        let (write_base_x, source_base_x, x_offset_addend) = if write.point.x <= source.x {
            (write.point.x, source.x, 1)
        } else {
            (
                write.point.x + write.width - 1,
                source.x + write.width - 1,
                -1,
            )
        };

        let mut y_offset = 0;
        for _ in 0..write.height {
            let mut x_offset = 0;
            for _ in 0..write.width {
                // SAFETY:
                //
                // `point` is in the bounds of the [`Surface`].
                let value = unsafe {
                    self.read_pixel_unchecked(Point {
                        x: source_base_x.wrapping_add_signed(x_offset),
                        y: source_base_y.wrapping_add_signed(y_offset),
                    })
                };

                // SAFETY:
                //
                // `point` is in the bounds of the [`Surface`].
                unsafe {
                    self.write_pixel_unchecked(
                        Point {
                            x: write_base_x.wrapping_add_signed(x_offset),
                            y: write_base_y.wrapping_add_signed(y_offset),
                        },
                        value,
                    )
                }

                x_offset += x_offset_addend;
            }
            y_offset += y_offset_addend;
        }

        Ok(())
    }
}

// SAFETY:
//
// All data is forwarded to the underlying [`Surface`] implementation and thus must be safe.
unsafe impl<S: Surface> Surface for &mut S {
    fn width(&self) -> usize {
        <S as Surface>::width(self)
    }

    fn height(&self) -> usize {
        <S as Surface>::height(self)
    }

    unsafe fn write_pixel_unchecked(&mut self, point: Point, value: u32) {
        // SAFETY:
        //
        // The safety bounds that apply to the containing function also apply to the function we
        // are calling.
        unsafe { <S as Surface>::write_pixel_unchecked(self, point, value) }
    }

    unsafe fn read_pixel_unchecked(&self, point: Point) -> u32 {
        // SAFETY:
        //
        // The safety bounds that apply to the containing function also apply to the function we
        // are calling.
        unsafe { <S as Surface>::read_pixel_unchecked(self, point) }
    }
}

/// A requested operation would have been out of bounds.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutOfBoundsError;

impl fmt::Display for OutOfBoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "operation out of bounds".fmt(f)
    }
}

impl error::Error for OutOfBoundsError {}

/// A point in a [`Surface`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    /// The x-coordinate of the pixel.
    pub x: usize,
    /// The y-coordinate of the pixel.
    pub y: usize,
}

/// A region of pixels.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Region {
    /// The upper left corner of the region.
    pub point: Point,
    /// The width of the region in pixels.
    pub width: usize,
    /// The height of the region in pixels.
    pub height: usize,
}

impl Region {
    /// Returns the region demarcated by `a` and `b`.
    pub const fn from_points(a: Point, b: Point) -> Region {
        let min_x = const_min(a.x, b.x);
        let min_y = const_min(a.y, b.y);

        let point = Point { x: min_x, y: min_y };
        let width = const_max(a.x, b.x) - min_x;
        let height = const_max(a.y, b.y) - min_y;
        Region {
            point,
            width,
            height,
        }
    }
}

/// Returns `true` if the given `point` is within the given bounds.
pub const fn point_in_bounds(point: Point, width: usize, height: usize) -> bool {
    point.x < width && point.y < height
}

/// Returns `true` if the given `region` is within the given bounds.
pub const fn region_in_bounds(region: Region, width: usize, height: usize) -> bool {
    let end_x = region.point.x.checked_add(region.width);
    let end_y = region.point.y.checked_add(region.height);
    if let Some(end_x) = end_x
        && let Some(end_y) = end_y
    {
        end_x <= width && end_y <= height
    } else {
        false
    }
}

/// Returns the maximum of `a` and `b`.
const fn const_max(a: usize, b: usize) -> usize {
    if a >= b { a } else { b }
}

/// Returns the minimum of `a` and `b`.
const fn const_min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

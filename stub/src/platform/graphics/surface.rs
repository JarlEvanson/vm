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
    #[expect(dead_code)]
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
    #[expect(dead_code)]
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
    #[expect(dead_code)]
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

/// Generic implementation of a pixel based framebuffer.
#[derive(Debug)]
pub struct GenericSurface {
    /// The virtual address of the [`Surface`].
    address: *mut u8,
    /// The width of the [`Surface`] in pixels.
    width: usize,
    /// The height of the [`Surface`] in pixels.
    height: usize,
    /// The number of bytes between the start of one line and the start of an adjacent line.
    pitch: usize,
    /// The number of bits in a pixel.
    bpp: u16,
    /// The number of bits in the red bitmask.
    red_mask_size: u8,
    /// The offset of the bits in the red bitmask.
    red_mask_shift: u8,
    /// The number of bits in the green bitmask.
    green_mask_size: u8,
    /// The offset of the bits in the green bitmask.
    green_mask_shift: u8,
    /// The number of bits in the blue bitmask.
    blue_mask_size: u8,
    /// The offset of the bits in the blue bitmask.
    blue_mask_shift: u8,
}

impl GenericSurface {
    /// Creates a new [`GenericSurface`].
    ///
    /// # Safety
    ///
    /// The region of memory demarcated by `address` that extends `pitch * height` bytes must be
    /// writable and under the exclusive control of [`GenericSurface`] if successful.
    #[expect(clippy::too_many_arguments)]
    pub unsafe fn new(
        address: *mut u8,
        width: usize,
        height: usize,
        pitch: usize,
        bpp: u16,
        red_mask_size: u8,
        red_mask_shift: u8,
        green_mask_size: u8,
        green_mask_shift: u8,
        blue_mask_size: u8,
        blue_mask_shift: u8,
    ) -> Option<Self> {
        let max_x = width.saturating_sub(1);
        let max_x_bit_offset = max_x.checked_mul(usize::from(bpp))?;

        let max_y = height.saturating_sub(1);
        let max_y_bit_offset = max_y.checked_mul(pitch)?.checked_mul(8)?;
        let _ = max_x_bit_offset.checked_add(max_y_bit_offset)?;

        match bpp {
            8 | 16 | 32 | 64 => {}
            _ => {
                // TODO: support an arbitrary number of bits per pixel
                return None;
            }
        }

        let surface = Self {
            address,
            width,
            height,
            pitch,
            bpp,
            red_mask_size,
            red_mask_shift,
            green_mask_size,
            green_mask_shift,
            blue_mask_size,
            blue_mask_shift,
        };

        Some(surface)
    }
}

// SAFETY:
//
// Read and write bounds checking are properly implemented.
unsafe impl Surface for GenericSurface {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    unsafe fn write_pixel_unchecked(&mut self, point: Point, value: u32) {
        let x_bit_offset = point.x * usize::from(self.bpp);
        let y_bit_offset = point.y * self.pitch * 8;
        let bit_offset = x_bit_offset + y_bit_offset;

        let red = convert_from_rgba(value, self.red_mask_size, 0) << self.red_mask_shift;
        let green = convert_from_rgba(value, self.green_mask_size, 1) << self.green_mask_shift;
        let blue = convert_from_rgba(value, self.blue_mask_size, 2) << self.blue_mask_shift;
        let color = red | green | blue;

        let address = self.address.wrapping_byte_add(bit_offset / 8);
        match self.bpp {
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            #[expect(clippy::cast_possible_truncation, reason = "truncation")]
            8 => unsafe { address.write_volatile(color as u8) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            #[expect(clippy::cast_possible_truncation, reason = "truncation")]
            16 => unsafe { address.cast::<u16>().write_volatile(color as u16) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            #[expect(clippy::cast_possible_truncation, reason = "truncation")]
            32 => unsafe { address.cast::<u32>().write_volatile(color as u32) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            64 => unsafe { address.cast::<u64>().write_volatile(color) },
            _ => todo!("support an arbitrary number of bits per pixel"),
        }
    }

    unsafe fn read_pixel_unchecked(&self, point: Point) -> u32 {
        let x_bit_offset = point.x * usize::from(self.bpp);
        let y_bit_offset = point.y * self.pitch * 8;
        let bit_offset = x_bit_offset + y_bit_offset;

        let address = self.address.wrapping_byte_add(bit_offset / 8);
        let value = match self.bpp {
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            8 => unsafe { u64::from(address.read_volatile()) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            16 => unsafe { u64::from(address.cast::<u16>().read_volatile()) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            32 => unsafe { u64::from(address.cast::<u32>().read_volatile()) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            64 => unsafe { address.cast::<u64>().read_volatile() },
            _ => todo!("support an arbitrary number of bits per pixel"),
        };

        let red = convert_to_rgba(value >> self.red_mask_shift, self.red_mask_size, 0);
        let green = convert_to_rgba(value >> self.green_mask_shift, self.green_mask_size, 0);
        let blue = convert_to_rgba(value >> self.blue_mask_shift, self.blue_mask_size, 0);

        red | green | blue
    }

    fn copy_within(&mut self, write: Region, source: Point) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(write, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let read = Region {
            point: source,
            width: write.width,
            height: write.height,
        };
        if !region_in_bounds(read, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        assert!(self.bpp >= 8);
        let write_index = write.point.x + write.point.y * self.pitch;
        let read_index = read.point.x + read.point.y * self.pitch;

        let mut write_ptr = self.address.wrapping_byte_add(write_index);
        let mut read_ptr = self.address.wrapping_byte_add(read_index);

        let bytes_per_pixel = usize::from(self.bpp.div_ceil(8));
        for _ in 0..write.height {
            // SAFETY:
            //
            // This operation is performed on framebuffer memory and has had its bounds checked.
            unsafe { core::ptr::copy(read_ptr, write_ptr, write.width.strict_mul(bytes_per_pixel)) }
            write_ptr = write_ptr.wrapping_byte_add(self.pitch);
            read_ptr = read_ptr.wrapping_byte_add(self.pitch);
        }

        Ok(())
    }
}

// SAFETY:
//
// The pointer contained by [`GenericSurface`] does not provide access to thread-local or cpu-local
// memory and thus [`GenericSurface`] is [`Send`].
unsafe impl Send for GenericSurface {}

// SAFETY:
//
// All exposed methods provided by [`GenericSurface`] cannot mutate with an immutable reference and
// thus [`GenericSurface`] is [`Sync`].
unsafe impl Sync for GenericSurface {}

/// Converts a generic pixel value to its RGBA representation.
const fn convert_to_rgba(value: u64, size: u8, index: u8) -> u32 {
    let max_value_foreign = (1u64 << size) - 1;
    let converted_value_foreign = (value * 255) / max_value_foreign;

    #[expect(clippy::cast_possible_truncation, reason = "truncation")]
    {
        (converted_value_foreign << (index * 8)) as u32
    }
}

/// Converts an RGBA pixel value to its generic representation.
const fn convert_from_rgba(value: u32, size: u8, index: u8) -> u64 {
    #[expect(clippy::cast_possible_truncation, reason = "truncation")]
    let extracted_value = (value >> (index * 8)) as u8;

    let max_value_foreign = (1u64 << size) - 1;
    (extracted_value as u64 * max_value_foreign) / 255
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
    #[expect(dead_code)]
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

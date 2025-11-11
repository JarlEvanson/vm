//! Support for booting from the Limine boot protocol.

use core::{fmt::Write, ptr, slice};

use limine::{
    BASE_REVISION, BASE_REVISION_MAGIC_0, BASE_REVISION_MAGIC_1, BaseRevisionTag,
    framebuffer::{FRAMEBUFFER_REQUEST_MAGIC, FramebufferRequest, FramebufferV0},
};
use sync::ControlledModificationCell;

use crate::graphics::{
    console::Console,
    font::{FONT_MAP, GLYPH_ARRAY},
    surface::{Point, Surface},
};

/// Indicates the start of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.start")]
static REQUESTS_START_MARKER: [u64; 4] = limine::REQUESTS_START_MARKER;

/// Tag used to communicate the information regarding the base revision of the Limine protocol.
#[used]
#[unsafe(link_section = ".limine")]
static BASE_REVISION_TAG: ControlledModificationCell<BaseRevisionTag> =
    ControlledModificationCell::new(BaseRevisionTag {
        magic: BASE_REVISION_MAGIC_0,
        loaded_revision: BASE_REVISION_MAGIC_1,
        supported_revision: BASE_REVISION,
    });

/// Request for the framebuffer to be made accessible.
#[used]
#[unsafe(link_section = ".limine")]
static FRAMEBUFFER_REQUEST: ControlledModificationCell<FramebufferRequest> =
    ControlledModificationCell::new(FramebufferRequest {
        id: FRAMEBUFFER_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: [u64; 2] = limine::REQUESTS_END_MARKER;

/// Entry point for Rust when booted using the Limine boot protocol.
pub fn limine_main() -> ! {
    *crate::PANIC_FUNC.lock() = panic_handler;

    if BASE_REVISION_TAG.get().supported_revision == BASE_REVISION {
        // If the base revision this executable was loaded using is greater than or equal to 3,
        // then [`BaseRevisionTag::loaded_revision`] contains the base revision used to load the
        // executable. Otherwise, the base revision must be either 0, 1, or 2.
        if BASE_REVISION_TAG.get().loaded_revision != BASE_REVISION_MAGIC_1 {
            panic!(
                "Loaded using unsupported base revision {}",
                BASE_REVISION_TAG.get().loaded_revision
            )
        } else {
            panic!("Loaded using unsupported base revision (possible revisions are 0, 1, and 2)")
        }
    }

    loop {
        core::hint::spin_loop()
    }
}

struct LimineSurface {
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

impl LimineSurface {
    /// Creates a new [`LimineSurface`] as specified by [`FramebufferV0`].
    ///
    /// # Safety
    ///
    /// The produced [`LimineSurface`] must have exclusive access to the underlying region it is
    /// manipulating.
    pub unsafe fn new(framebuffer: &FramebufferV0) -> Option<LimineSurface> {
        let width = usize::try_from(framebuffer.width).ok()?;
        let height = usize::try_from(framebuffer.height).ok()?;
        let pitch = usize::try_from(framebuffer.pitch).ok()?;

        let max_x = width.saturating_sub(1);
        let max_x_bit_offset = max_x.checked_mul(usize::from(framebuffer.bpp))?;

        let max_y = height.saturating_sub(1);
        let max_y_bit_offset = max_y.checked_mul(pitch)?.checked_mul(8)?;
        let _ = max_x_bit_offset.checked_add(max_y_bit_offset)?;

        match framebuffer.bpp {
            8 | 16 | 32 | 64 => {}
            _ => {
                // TODO: support an arbitrary number of bits per pixel
                return None;
            }
        }

        let surface = Self {
            address: framebuffer.address.cast::<u8>(),
            width,
            height,
            pitch,
            bpp: framebuffer.bpp,
            red_mask_size: framebuffer.red_mask_size,
            red_mask_shift: framebuffer.red_mask_shift,
            green_mask_size: framebuffer.green_mask_size,
            green_mask_shift: framebuffer.green_mask_shift,
            blue_mask_size: framebuffer.blue_mask_size,
            blue_mask_shift: framebuffer.blue_mask_shift,
        };

        Some(surface)
    }
}

unsafe impl Surface for LimineSurface {
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
            8 => unsafe { address.write_volatile(color as u8) },
            16 => unsafe { address.cast::<u16>().write_volatile(color as u16) },
            32 => unsafe { address.cast::<u32>().write_volatile(color as u32) },
            64 => unsafe { address.cast::<u64>().write_volatile(color as u64) },
            _ => todo!("support an arbitrary number of bits per pixel"),
        }
    }

    unsafe fn read_pixel_unchecked(&self, point: Point) -> u32 {
        let x_bit_offset = point.x * usize::from(self.bpp);
        let y_bit_offset = point.y * self.pitch * 8;
        let bit_offset = x_bit_offset + y_bit_offset;

        let address = self.address.wrapping_byte_add(bit_offset / 8);
        let value = match self.bpp {
            8 => unsafe { address.read_volatile() as u64 },
            16 => unsafe { address.cast::<u16>().read_volatile() as u64 },
            32 => unsafe { address.cast::<u32>().read_volatile() as u64 },
            64 => unsafe { address.cast::<u64>().read_volatile() as u64 },
            _ => todo!("support an arbitrary number of bits per pixel"),
        };

        let red = convert_to_rgba(value >> self.red_mask_shift, self.red_mask_size, 0);
        let green = convert_to_rgba(value >> self.green_mask_shift, self.green_mask_size, 0);
        let blue = convert_to_rgba(value >> self.blue_mask_shift, self.blue_mask_size, 0);

        red | green | blue
    }
}

const fn convert_to_rgba(value: u64, size: u8, index: u8) -> u32 {
    let max_value_foreign = (1u64 << size) - 1;
    let converted_value_foreign = (value * 255) / max_value_foreign;

    (converted_value_foreign << (index * 8)) as u32
}

const fn convert_from_rgba(value: u32, size: u8, index: u8) -> u64 {
    let extracted_value = (value >> (index * 8)) as u8;

    let max_value_foreign = (1u64 << size) - 1;
    (extracted_value as u64 * max_value_foreign) / 255
}

fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    let framebuffer_response = FRAMEBUFFER_REQUEST.get().response;
    if let Some(framebuffer_response) = unsafe { framebuffer_response.as_ref() } {
        let framebuffers = unsafe {
            slice::from_raw_parts(
                framebuffer_response.framebuffers.cast::<&FramebufferV0>(),
                framebuffer_response.framebuffer_count as usize,
            )
        };

        for framebuffer in framebuffers {
            let Some(framebuffer) = (unsafe { LimineSurface::new(framebuffer) }) else {
                continue;
            };

            let mut console = Console::new(framebuffer, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0x00);
            let _ = writeln!(console, "{info}");
        }
    }

    loop {}
}

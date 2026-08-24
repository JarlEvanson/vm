//! Graphical logging implementation for Limine provided [`GenericSurface`]s.

use core::{fmt::Write, mem, ptr::NonNull};

use crate::platform::{
    Console as LogConsole, Metadata,
    graphics::{
        font::{FONT_MAP, GLYPH_ARRAY},
        surface::GenericSurface,
    },
    limine::FRAMEBUFFERS,
    register_console,
};
use limine::framebuffer::FramebufferV0;
use sync::{ControlledModificationCell, Spinlock};

use crate::platform::graphics::console::TextConsole;

/// Implementation of [`Console`] utilizing the first framebuffer.
static PRIMARY_FRAMEBUFFER: ControlledModificationCell<Option<Console>> =
    ControlledModificationCell::new(None);

/// Returns `true` if the [`PRIMARY_FRAMEBUFFER`] is initialized.
pub fn primary_framebuffer_initialized() -> bool {
    PRIMARY_FRAMEBUFFER.get().is_some()
}

/// Initializes the first framebuffer output device.
///
/// # Safety
///
/// After this call, this module must take exclusive control over the system framebuffers.
pub unsafe fn initialize_primary_framebuffer() {
    let [framebuffer, ..] = FRAMEBUFFERS.get() else {
        return;
    };

    // SAFETY:
    //
    // This module is the only module that interacts with framebuffers and the invariants of this
    // function ensure that this operation is safe.
    let Some(surface) = (unsafe { create_surface(framebuffer) }) else {
        return;
    };
    let text_console = TextConsole::new(surface, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0x00);
    let console = Console {
        text_console: Spinlock::new(text_console),
        log_console: LogConsole::new(write),
        next: None,
    };

    // SAFETY:
    //
    // This invariants of this function ensure that this operation is safe.
    let framebuffer = unsafe { PRIMARY_FRAMEBUFFER.get_mut() };
    let framebuffer = framebuffer.insert(console);

    // SAFETY:
    //
    // This invariants of this function ensure that this operation is safe.
    unsafe { register_console(NonNull::from_ref(&framebuffer.log_console)) }
}

/// Implementation of console writing for [`Console`].
fn write(console: NonNull<LogConsole>, metadata: Metadata, message: &str) {
    // SAFETY:
    //
    // The [`LogConsole`] passed to this function is of the type [`Console`] and thus `container-of`
    // semantics work.
    let console = unsafe { console.sub(mem::offset_of!(Console, log_console)) };
    // SAFETY:
    //
    // The [`LogConsole`] is only accessed using references when this is when logging is occurring.
    let console = unsafe { console.cast::<Console>().as_ref() };

    let mut text_console = console.text_console.lock();
    let _ = write!(text_console, "[{:?}]: {message}", metadata.level);
}

/// Creates a new [`GenericSurface`] as specified by [`FramebufferV0`].
///
/// # Safety
///
/// The produced [`GenericSurface`] must have exclusive access to the underlying region it is
/// manipulating.
pub unsafe fn create_surface(framebuffer: &FramebufferV0) -> Option<GenericSurface> {
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

    // SAFETY:
    //
    // The invariants of [`create_surface()`] ensure that the invariants of
    // [`GenericSurface::new()`] are fulfilled.
    unsafe {
        GenericSurface::new(
            framebuffer.address.cast::<u8>(),
            width,
            height,
            pitch,
            framebuffer.bpp,
            framebuffer.red_mask_size,
            framebuffer.red_mask_shift,
            framebuffer.green_mask_size,
            framebuffer.green_mask_shift,
            framebuffer.blue_mask_size,
            framebuffer.blue_mask_shift,
        )
    }
}

/// Implementation of a [`register_console()`] compatible interface for [`GenericSurface`]s.
struct Console {
    /// The textual output device.
    text_console: Spinlock<TextConsole<'static, GenericSurface>>,
    /// The [`LogConsole`] bindings (for [`register_console()`]).
    log_console: LogConsole,
    /// Tracker to handle iteration over [`GenericSurface`] graphical consoles.
    #[expect(dead_code)]
    next: Option<NonNull<Console>>,
}

// SAFETY:
//
// TODO:
unsafe impl Send for Console {}
// SAFETY:
//
// TODO:
unsafe impl Sync for Console {}

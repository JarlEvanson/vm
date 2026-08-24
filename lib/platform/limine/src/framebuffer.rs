//! Definitions of [`FramebufferRequest`] and [`FramebufferResponse`].

use core::ffi::c_void;

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as a [`FramebufferRequest`].
pub const FRAMEBUFFER_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0x9d5827dcd881dd75,
    0xa3148604f6fab11b,
];

/// Request for the framebuffers provided by the boot firmware.
#[repr(C)]
#[derive(Debug)]
pub struct FramebufferRequest {
    /// Location storing [`FRAMEBUFFER_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`FramebufferRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`FramebufferResponse`] structure for this [`FramebufferRequest`].
    pub response: *mut FramebufferResponse,
}

// SAFETY:
//
// [`FramebufferRequest`] does not interact with threads in any manner.
unsafe impl Send for FramebufferRequest {}
// SAFETY:
//
// [`FramebufferRequest`] does not interact with threads in any manner.
unsafe impl Sync for FramebufferRequest {}

/// Response to a [`FramebufferRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct FramebufferResponse {
    /// The revision of the [`FramebufferResponse`] structure.
    pub revision: u64,
    /// How many framebuffers are present.
    pub framebuffer_count: u64,
    /// A pointer to an array of [`FramebufferResponse::framebuffer_count`] pointers to
    /// framebuffer structures.
    pub framebuffers: *mut *mut FramebufferV0,
}

// SAFETY:
//
// [`FramebufferResponse`] does not interact with threads in any manner.
unsafe impl Send for FramebufferResponse {}
// SAFETY:
//
// [`FramebufferResponse`] does not interact with threads in any manner.
unsafe impl Sync for FramebufferResponse {}

/// Description of a framebufer.
///
/// Structure returned from a [`FramebufferResponse`] revision 0.
#[repr(C)]
#[derive(Debug)]
pub struct FramebufferV0 {
    /// The virtual address of the framebuffer.
    pub address: *mut c_void,
    /// The width of the framebuffer in pixels.
    pub width: u64,
    /// The height of the framebuffer in pixels.
    pub height: u64,
    /// The number of bytes between the start of one line and the start of an adjacent line.
    pub pitch: u64,
    /// The number of bits per pixel.
    pub bpp: u16,
    /// TODO:
    pub memory_model: u8,
    /// The number of bits in the red bitmask.
    pub red_mask_size: u8,
    /// The offset of bits in the red bitmask.
    pub red_mask_shift: u8,
    /// The number of bits in the green bitmask.
    pub green_mask_size: u8,
    /// The offset of bits in the green bitmask.
    pub green_mask_shift: u8,
    /// The number of bits in the blue bitmask.
    pub blue_mask_size: u8,
    /// The offset of bits in the blue bitmask.
    pub blue_mask_shift: u8,
    /// Currently unused space.
    pub _unused: [u8; 7],
    /// Size of the EDID blob in bytes.
    pub edid_size: u64,
    /// A pointer to the EDID blob.
    pub edid: *mut c_void,
}

// SAFETY:
//
// [`FramebufferV0`] does not interact with threads in any manner.
unsafe impl Send for FramebufferV0 {}
// SAFETY:
//
// [`FramebufferV0`] does not interact with threads in any manner.
unsafe impl Sync for FramebufferV0 {}

/// Description of a framebufer.
///
/// Structure returned from a [`FramebufferResponse`] revision 1.
#[repr(C)]
#[derive(Debug)]
pub struct FramebufferV1 {
    /// Definition of base revision's structure.
    pub framebuffer_v0: FramebufferV0,

    /// How many video modes are supported.
    pub mode_count: u64,
    /// A pointer to an array of [`FramebufferV1::mode_count`] pointers to [`VideoMode`]
    /// structures.
    pub modes: *mut *mut VideoMode,
}

// SAFETY:
//
// [`FramebufferV1`] does not interact with threads in any manner.
unsafe impl Send for FramebufferV1 {}
// SAFETY:
//
// [`FramebufferV1`] does not interact with threads in any manner.
unsafe impl Sync for FramebufferV1 {}

/// Description of a video mode setting.
#[repr(C)]
#[derive(Debug)]
pub struct VideoMode {
    /// The number of pixels between lines.
    pub pitch: u64,
    /// The width of the framebuffer in pixels.
    pub width: u64,
    /// The height of the framebuffer in pixels.
    pub height: u64,
    /// The number of bits per pixel.
    pub bpp: u16,
    /// TODO:
    pub memory_model: u8,
    /// The number of bits in the red bitmask.
    pub red_mask_size: u8,
    /// The offset of bits in the red bitmask.
    pub red_mask_shift: u8,
    /// The number of bits in the green bitmask.
    pub green_mask_size: u8,
    /// The offset of bits in the green bitmask.
    pub green_mask_shift: u8,
    /// The number of bits in the blue bitmask.
    pub blue_mask_size: u8,
    /// The offset of bits in the blue bitmask.
    pub blue_mask_shift: u8,
}

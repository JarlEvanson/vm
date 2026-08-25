//! Various utility functions.

unsafe extern "C" {
    #[link_name = "_image_start"]
    static IMAGE_START: u8;

    #[link_name = "_image_end"]
    static IMAGE_END: u8;
}

/// Returns the virtual address of the start of the image.
pub fn image_start() -> usize {
    (&raw const IMAGE_START).addr()
}

/// Returns the virtual address of the end of the image.
pub fn image_end() -> usize {
    (&raw const IMAGE_END).addr()
}

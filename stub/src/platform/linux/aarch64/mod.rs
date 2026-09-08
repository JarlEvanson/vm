//! Support for booting using the Linux `aarch64` boot protocol.

use device_tree::raw::FdtHeader;

/// Rust entry point for the Linux boot protocol on `aarch64`.
pub extern "C" fn main(
    dtb_ptr: *mut FdtHeader,
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

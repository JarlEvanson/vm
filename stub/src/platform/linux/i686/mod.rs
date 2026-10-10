//! Support for booting using the Linux `i686` boot protocol.

use linux::x86::BootParams;

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn main(
    boot_params_ptr: *mut BootParams,
    image_start: u32,
    image_size: u32,
    stack_start: u32,
    stack_size: u32,
) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

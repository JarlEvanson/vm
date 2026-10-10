//! Support for booting using the Linux `x86_64` boot protocol.

use linux::x86::BootParams;

/// Rust entrypoint for the Linux boot protocol.
pub extern "C" fn main(
    boot_params: *mut BootParams,
    image_start: u64,
    image_size: u64,
    stack_start: u64,
    stack_size: u64,
) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

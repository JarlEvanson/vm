//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Platform support modules.
#[cfg(CONFIG_STUB_PLATFORM_UEFI)]
mod uefi;

#[cfg(CONFIG_STUB_PLATFORM_UEFI)]
use uefi::uefi_main;

#[cfg(not(CONFIG_STUB_PLATFORM_UEFI))]
use dummy_main as uefi_main;

// Other support modules.

#[cfg(CONFIG_STUB_SELF_FRAME_ALLOCATOR)]
mod frame_allocator;
#[cfg(CONFIG_STUB_GRAPHICS)]
mod graphics;
#[cfg(CONFIG_STUB_CUSTOM_HEAP_ALLOCATOR)]
mod heap_allocator;
mod interface;
mod relocate;

pub use interface::*;

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global main",
    "main:",

    "stp x29, x30, [sp, #-16]",
    "stp x0, x1, [sp, #-32]",
    "sub sp, sp, #32",

    "bl relocate",
    "cmp x0, #0",

    "add sp, sp, #32",
    "ldp x0, x1, [sp, #-32]",
    "ldp x29, x30, [sp, #-16]",

    "b.ne 5f", // Branch if `relocate` failed.

    "b {uefi_main}",

    "5:",
    // Return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    uefi_main = sym uefi_main,
}

#[cfg(target_arch = "x86")]
core::arch::global_asm! {
    ".global main",
    "main:",

    "pusha",
    "call relocate",

    "cmp eax, 0", // Check for successful `relocate`.
    "popa",

    "jne 5f",     // Jump if failed.
    "jmp {uefi_main}",

    "5:",
    // Return with eax = 0x80000001 (LOAD_ERROR).
    "mov eax, 0x80000001",
    "ret",

    uefi_main = sym uefi_main,
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global main",
    "main:",

    "push rcx",
    "push rdx",

    "call relocate",

    "pop rdx",
    "pop rcx",

    "cmp rax, 0",   // Check for successful `relocate`.
    "jne 5f",       // Jump if failed.

    "jmp {uefi_main}",

    "5:",
    // Return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    uefi_main = sym uefi_main,
}

#[allow(dead_code)]
fn dummy_main() -> ! {
    unreachable!()
}

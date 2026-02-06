//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
use crate::platform::limine::limine_main;
use crate::platform::uefi::uefi_main;

mod generic;
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
mod limine;
mod relocation;
mod uefi;

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

    "cbnz x0, {uefi_main}", // If first argument is non-zero, jump to UEFI.
    "b {limine_main}",

    "5:",
    "cbz x0, 6f", // If first argument is zero, spin forever (it's Limine).

    // Otherwise, return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    "6:",
    "b 6b",

    uefi_main = sym uefi_main,
    limine_main = sym limine_main,
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

    "cmp rcx, 0",
    "jne {uefi_main}",  // If first argument is non-zero, jump to UEFI.
    "jmp {limine_main}",

    "5:",
    "cmp rcx, 0", // If zero, then spin forever (it's Limine).
    "je 6f",

    // Otherwise, return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    "6:",
    "jmp 6b",

    uefi_main = sym uefi_main,
    limine_main = sym limine_main,
}

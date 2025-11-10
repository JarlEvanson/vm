//! Collection of supported boot protocols and utilities for carrying out boot operations.

mod context;
mod relocation;

mod limine;
mod uefi;

pub use context::{AllocationPolicy, Context, FailedMapping, NotFound, OutOfMemory};

use limine::limine_main;
use uefi::uefi_main;

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

    "cbz x0, {limine_main}", // If first argument is zero, jump to limine.
    "b {uefi_main}",         // Otherwise, jump to UEFI

    "5:",
    "cbz x0, 6f", // If first argument is zero, spin forever (it's limine).

    // Otherwise, return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    "6:",
    "b 6b",

    uefi_main = sym uefi_main,
    limine_main = sym limine_main,
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

    "cmp rax, 0",   // Check for succssful `relocate`.
    "jne 5f",       // Jump if failed.

    "cmp rcx, 0",
    "je {limine_main}", // If first argument is zero, jump to Limine.
    "jmp {uefi_main}",  // Otherwise, jump to UEFI

    "5:",
    "cmp rcx, 0", // If zero, then spin forever (it's linux).
    "je 6f",

    // Otherwise, return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    "6:",
    "jmp 6b",

    uefi_main = sym uefi_main,
    limine_main = sym limine_main,
}

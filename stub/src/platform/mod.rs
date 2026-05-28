//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Platform support modules.
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
mod limine;
mod linux;
mod uefi;

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
use limine::limine_main;
use linux::linux_main;
use uefi::uefi_main;

// Other support modules.

mod frame_allocator;
mod generic;
mod graphics;
mod heap_allocator;
mod relocate;
mod shared;

pub use generic::*;

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global main",
    "main:",

    "stp x29, x30, [sp, #-16]",
    "stp x0, x1, [sp, #-32]",
    "stp x2, x3, [sp, #-48]",
    "stp x4, x5, [sp, #-64]",
    "sub sp, sp, #64",

    "bl relocate",
    "cmp x0, #0",

    "add sp, sp, #64",
    "ldp x4, x5, [sp, #-64]",
    "ldp x2, x3, [sp, #-48]",
    "ldp x0, x1, [sp, #-32]",
    "ldp x29, x30, [sp, #-16]",

    "b.ne 5f", // Branch if `relocate` failed.

    // Limine always has its first argument as zero, while Linux requires a DTB pointer and UEFI
    // requires an image handle.
    "cbz x0, {limine_main}",
    // The return address is set to zero by the `aarch64` Linux boot protocol pre-bootloader.
    "cbz x30, {linux_main}",
    "b {uefi_main}",

    "5:",
    "cbz x30, 6f", // If the return address is zero, spin forever.

    // Otherwise, return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    "6:",
    "b 6b",

    linux_main = sym linux_main,
    limine_main = sym limine_main,
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

    "cmp dword ptr [esp], 0",
    "je {linux_main}",
    "jmp {uefi_main}",

    "5:",
    // Return with eax = 0x80000001 (LOAD_ERROR).
    "mov eax, 0x80000001",
    "ret",

    linux_main = sym linux_main,
    uefi_main = sym uefi_main,
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global main",
    "main:",

    "push rcx",
    "push rdx",

    "push rdi",
    "push rsi",
    "push r8",

    "call relocate",

    "pop r8",
    "pop rsi",
    "pop rdi",

    "pop rdx",
    "pop rcx",

    "cmp rax, 0",   // Check for successful `relocate`.
    "jne 5f",       // Jump if failed.

    "cmp rcx, 0", // If the first argument is zero, then this was booted using Limine.
    "je {limine_main}",
    "cmp qword ptr [rsp], 0",
    "je {linux_main}",
    "jmp {uefi_main}",

    "5:",
    "cmp rcx, 0", // If zero, then spin forever (it's Limine).
    "je 6f",

    // Otherwise, return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    "6:",
    "jmp 6b",

    linux_main = sym linux_main,
    limine_main = sym limine_main,
    uefi_main = sym uefi_main,
}

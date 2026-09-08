//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Other support modules.

mod relocation;

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global entry_point",
    "main:",

    // Save `x30` before the `bl` instruction overwrites it.
    "sub sp, sp, #16",
    "str x30, [sp]",

    "bl relocate",
    "b.cs relocate_failed",

    // Restores `x30` after the `bl` instruction overwrites it.
    "ldr x30, [sp]",
    "add sp, sp, #16",

    "relocate_failed:",

    "7:",
    "b 7b",
}

#[cfg(target_arch = "x86")]
core::arch::global_asm! {
    ".global entry_point",
    "main:",

    "call relocate",
    "jc relocate_failed",

    "relocate_failed:",

    "7:",
    "jmp 7b",
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global entry_point",
    "main:",

    "call relocate",
    "jc relocate_failed",

    "relocate_failed:",

    "7:",
    "jmp 7b",
}

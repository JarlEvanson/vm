//! A collection of supported platforms and various utilities provided by said platforms that are
//! required to carry out `revm-stub`'s goal.

// Platform support modules.

#[cfg(CONFIG_STUB_PLATFORM_UEFI)]
mod uefi;

// Other support modules.

mod relocation;

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global entry_point",
    "entry_point:",

    // Save `x30` before the `bl` instruction overwrites it.
    "sub sp, sp, #16",
    "str x30, [sp]",

    "bl relocate",
    "b.cs relocate_failed",

    // Restores `x30` after the `bl` instruction overwrites it.
    "ldr x30, [sp]",
    "add sp, sp, #16",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "b {uefi_main}",

    "relocate_failed:",

    // Return with x0 = 0x8000000000000001 (LOAD_ERROR).
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "orr x0, x0, #0x8000000000000001",
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "ret",

    "7:",
    "b 7b",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    uefi_main = sym uefi::main,
}

#[cfg(target_arch = "x86")]
core::arch::global_asm! {
    ".global entry_point",
    "entry_point:",

    "call relocate",
    "jc relocate_failed",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "jmp {uefi_main}",

    "relocate_failed:",

    // Return with eax = 0x80000001 (LOAD_ERROR).
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "mov eax, 0x80000001",
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "ret",

    "7:",
    "jmp 7b",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    uefi_main = sym uefi::main,
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global entry_point",
    "entry_point:",

    "call relocate",
    "jc relocate_failed",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "jmp {uefi_main}",

    "relocate_failed:",

    // Return with rax = 0x8000000000000001 (LOAD_ERROR).
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "mov rax, 0x8000000000000001",
    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    "ret",

    "7:",
    "jmp 7b",

    #[cfg(CONFIG_STUB_PLATFORM_UEFI)]
    uefi_main = sym uefi::main,
}

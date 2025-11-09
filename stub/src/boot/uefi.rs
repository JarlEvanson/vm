//! Support for booting from a UEFI environment.

use uefi::{
    data_type::{Handle, Status},
    table::system::SystemTable,
};

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global efi_main",
    "efi_main:",

    "stp x29, x30, [sp, #-16]",
    "stp x0, x1, [sp, #-32]",
    "sub sp, sp, #32",

    "bl relocate",
    "cmp x0, #0",

    "add sp, sp, #32",
    "ldp x0, x1, [sp, #-32]",
    "ldp x29, x30, [sp, #-16]",

    // If `relocate()` was a success, jump to `uefi_main()`.
    "b.eq {uefi_main}",
    // Otherwise, return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    uefi_main = sym uefi_main,
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global efi_main",
    "efi_main:",

    "push rcx",
    "push rdx",

    "call relocate",
    "cmp rax, 0",

    "pop rdx",
    "pop rcx",

    // If `relocate()` was a success, jump to `uefi_main()`.
    "je {uefi_main}",
    // Otherwise, return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    uefi_main = sym uefi_main,
}

extern "efiapi" fn uefi_main(image_handle: Handle, system_table_ptr: *mut SystemTable) -> Status {
    Status::LOAD_ERROR
}

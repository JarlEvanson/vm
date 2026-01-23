//! Implementations of the architecture-specific self-relocation code.

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global relocate",
    "relocate:",

    // Get slide (_image_start's linktime address is 0x0).
    "adrp x9, _image_start",
    "add x9, x9, :lo12:_image_start",

    // Get address of _DYNAMIC
    "adrp x0, _DYNAMIC",
    "add x0, x0, :lo12:_DYNAMIC",

    // Zero x5, x6, and x7
    "mov x5, #0",
    "mov x6, #0",
    "mov x7, #0",

    "3:",
    "ldr x1, [x0]",     // x1: dt_tag
    "cbz x1, 9f",       // check that d_tag != DT_NULL
    "ldr x2, [x0, #8]", // x2: dt_value
    "add x0, x0, #16",  // increment tracking pointer.

    // Check for DT_RELA.
    "cmp x1, 7",
    "b.eq .handle_rela",

    // Check for DT_RELA_SZ
    "cmp x1, 8",
    "b.eq .handle_rela_sz",

    // Check for DT_RELA_ENT
    "cmp x1, 9",
    "b.eq .handle_rela_ent",

    // Not an important entry; return to top of loop
    "b 3b",

    ".handle_rela:",
    "mov x5, x2",
    "b 3b",

    ".handle_rela_sz:",
    "mov x6, x2",
    "b 3b",

    ".handle_rela_ent:",
    "mov x7, x2",
    "b 3b",

    // Finished with _DYNAMIC array.
    //
    // x5: RELA
    // x6: RELA_SZ
    // x7: RELA_ENT
    // x9: slide
    "9:",
    "add x6, x5, x6", // x6: end of RELA section.
    "add x5, x5, x9", // x5: start of RELA section adjusted by slide
    "add x6, x6, x9", // x6: end of RELA section adjusted by slide

    "3:",
    "cmp x5, x6",           // check that there is another Rela entry.
    "b.ge 9f",              // exit if there isn't one.

    "ldr x0, [x5]",         // x0: r_offset
    "ldr x1, [x5, #8]",     // x1: r_info
    "ldr x2, [x5, #16]",    // x2: r_addend

    "mov w3, w1",           // w3: r_type
    "cmp w3, 1027",         // Compare to AARCH64_RELATIVE
    "b.ne 8f",              // if not AARCH64_RELATIVE, goto failure

    "add x10, x9, x0",      // address = slide + offset
    "add x11, x9, x2",      // value = slide + addend
    "str x11, [x10]",       // store value at address

    "add x5, x5, x7",       // Update tracking pointer
    "b 3b",                 // return to top of loop

    // Return 1 to signal unsuccessful exit.
    "8:",
    "mov x0, #1",
    "ret",

    // Return 0 to signal successful exit.
    "9:",
    "mov x0, #0",
    "ret",
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global relocate",
    "relocate:",

    // Get slide (_image_start's linktime address is 0x0).
    "lea r11, [rip + _image_start]",

    // Get address of _DYNAMIC
    "lea rax, [rip + _DYNAMIC]",

    // Zero rcx, rdx, and r8.
    "xor rcx, rcx",
    "mov rdx, rcx",
    "mov r8, rcx",

    "3:",
    "mov r9, [rax]",
    "cmp r9, 0",
    "je 9f",
    "mov r10, [rax + 8]",
    "add rax, 16",

    // Check for DT_RELA.
    "cmp r9, 7",
    "cmove rcx, r10",

    // Check for DT_RELA_SZ.
    "cmp r9, 8",
    "cmove rdx, r10",

    // Check for DT_RELA_ENT.
    "cmp r9, 9",
    "cmove r8, r10",

    // Return to top of loop.
    "jmp 3b",

    // Finished with _DYNAMIC array.
    //
    // rcx: RELA
    // rdx: RELA_SZ
    // r8: RELA_ENT
    // r11: slide
    "9:",
    "add rdx, rcx", // rdx: end of RELA section.
    "add rcx, r11", // rcx: start of RELA section adjusted by slide
    "add rdx, r11", // rdx: end of RELA section adjusted by slide

    "3:",
    "cmp rcx, rdx",
    "jge 9f",

    "mov rax, [rcx + 8]",   // Load r_info.
    "cmp eax, 8",           // Compare r_info for R_X86_64_RELATIVE.
    "jne 8f",               // If not R_X86_64_RELATIVE, then goto failure.

    "mov rax, [rcx]",       // rax: r_offset
    "add rax, r11",         // address = r_offset + slide
    "mov r10, [rcx + 16]",  // r10: r_addend
    "add r10, r11",         // value = r_addend + slide
    "mov [rax], r10",       // store value to address

    "add rcx, r8",          // Update tracking pointer
    "jmp 3b",               // return to top of loop

    "8:",
    "mov rax, 1",
    "ret",

    "9:",
    "mov rax, 0",
    "ret",
}

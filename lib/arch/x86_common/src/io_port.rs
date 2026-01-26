//! I/O port-related functions.

use core::arch::asm;

/// Writes `val` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u8(port: u16, val: u8) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Writes `val` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u16(port: u16, val: u16) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") val,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Writes `val` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u32(port: u16, val: u32) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") val,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Reads a `u8` from `port`.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u8(port: u16) -> u8 {
    let val: u8;
    // SAFETY:
    //
    // According to the invariants of this function, this is safe to run.
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            lateout("al") val,
            options(nomem, nostack, preserves_flags)
        )
    }

    val
}

/// Reads a `u16` from `port`.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u16(port: u16) -> u16 {
    let val: u16;
    // SAFETY:
    //
    // According to the invariants of this function, this is safe to run.
    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            lateout("ax") val,
            options(nomem, nostack, preserves_flags)
        )
    }

    val
}

/// Reads a `u32` from `port`.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u32(port: u16) -> u32 {
    let val: u32;
    // SAFETY:
    //
    // According to the invariants of this function, this is safe to run.
    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            lateout("eax") val,
            options(nomem, nostack, preserves_flags)
        )
    }

    val
}

/// Writes the contents of `slice` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u8_slice(port: u16, slice: &[u8]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "xchg esi, {ptr}",
            "rep outsb",
            "xchg {ptr}, esi",
            in("dx") port,
            ptr = inout(reg) slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep outsb",
            in("dx") port,
            inout("rsi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );
    }
}

/// Writes the contents of `slice` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u16_slice(port: u16, slice: &[u16]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "xchg esi, {ptr}",
            "rep outsw",
            "xchg {ptr}, esi",
            in("dx") port,
            ptr = inout(reg) slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep outsb",
            in("dx") port,
            inout("rsi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );
    }
}

/// Writes the contents of `slice` to `port`.
///
/// # Safety
///
/// Writing to `port` must not cause undefined behavior.
pub unsafe fn write_u32_slice(port: u16, slice: &[u32]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "xchg esi, {ptr}",
            "rep outsd",
            "xchg {ptr}, esi",
            in("dx") port,
            ptr = inout(reg) slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep outsb",
            in("dx") port,
            inout("rsi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(readonly, nostack, preserves_flags)
        );
    }
}

/// Reads from `port` until `slice` is filled.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u8_slice(port: u16, slice: &mut [u8]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "rep insb",
            in("dx") port,
            inout("edi") slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep insb",
            in("dx") port,
            inout("rdi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(nostack, preserves_flags)
        );
    }
}

/// Reads from `port` until `slice` is filled.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u16_slice(port: u16, slice: &mut [u16]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "rep insw",
            in("dx") port,
            inout("edi") slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep insw",
            in("dx") port,
            inout("rdi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(nostack, preserves_flags)
        );
    }
}

/// Reads from `port` until `slice` is filled.
///
/// # Safety
///
/// Reading from `port` must not cause undefined behavior.
pub unsafe fn read_u32_slice(port: u16, slice: &mut [u32]) {
    // SAFETY:
    //
    // According to the invariants of the function, this is safe to run.
    unsafe {
        #[cfg(target_arch = "x86")]
        asm!(
            "rep insd",
            in("dx") port,
            inout("edi") slice.as_ptr() => _,
            inout("ecx") slice.len() => _,
            options(nostack, preserves_flags)
        );

        #[cfg(target_arch = "x86_64")]
        asm!(
            "rep insd",
            in("dx") port,
            inout("rdi") slice.as_ptr() => _,
            inout("rcx") slice.len() => _,
            options(nostack, preserves_flags)
        );
    }
}

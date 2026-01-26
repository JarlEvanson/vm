//! TLB-management instruction support.

use core::arch::asm;

/// Invalidates the TLB entries for the page of `address`.
///
/// Executes INVLPG under the hood.
pub fn invalidate_page(address: usize) {
    // SAFETY:
    //
    // Should not cause any problems if called repeatedly.
    unsafe {
        asm!(
            "invlpg [{}]",
            in(reg) address,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Invalidates the TLB entries for the page of `address` with PCID `pcid`.
///
/// # Safety
///
/// - The processor must have the INVPCID feature.
/// - `pcid` must be a valid PCID.
/// - `address` must be a canonical address.
#[expect(clippy::as_conversions)]
pub unsafe fn invalid_pcid_address(pcid: u16, address: usize) {
    debug_assert!(pcid < (1 << 12));

    #[cfg(not(any(
        target_pointer_width = "16",
        target_pointer_width = "32",
        target_pointer_width = "64",
    )))]
    compile_error!("only 16-bit, 32-bit, and 64-bit address widths are supported by this library");
    let descriptor = u128::from(pcid) | ((address as u128) << 64);

    // SAFETY:
    //
    // - 0 is a valid `invalidation_type`.
    // - `picd | (address << 64)` is a valid descriptor value for `invalidation_type` 0.
    unsafe { invpcid(0, &descriptor) }
}

/// Invalidates the TLB entries associated with PCID `pcid`.
///
/// # Safety
///
/// - The processor must have the INVPCID feature.
/// - `pcid` must be a valid PCID.
pub unsafe fn invalidate_pcid(pcid: u16) {
    debug_assert!(pcid < (1 << 12));

    let descriptor = u128::from(pcid);
    // SAFETY:
    //
    // - 1 is a valid `invalidation_type`.
    // - `pcid` is a valid descriptor value for `invalidation_type` 1.
    unsafe { invpcid(1, &descriptor) }
}

/// Invalidates all TLB entries, including entries marked as global.
///
/// # Safety
///
/// - The processor must have the INVPCID feature.
pub unsafe fn invalidate_all() {
    // SAFETY:
    //
    // - 2 is a valid `invalidation_type`.
    // - 0 is a valid descriptor value for `invalidation_type` 2.
    unsafe { invpcid(2, &0) }
}

/// Invalidates all TLB entries, except entries marked as global.
///
/// # Safety
///
/// - The processor must have the INVPCID feature.
pub unsafe fn invalidate_all_non_global() {
    // SAFETY:
    //
    // - 3 is a valid `invalidation_type`.
    // - 0 is a valid descriptor value for `invalidation_type` 3.
    unsafe { invpcid(3, &0) }
}

/// Executes INVPCID with the given `invalidation_type` using `descriptor`.
///
/// # Safety
///
/// - `invalidation_type` must be be a valid INVPCID type.
/// - `descriptor` must be suitable for the INVPCID call.
unsafe fn invpcid(invalidation_type: usize, descriptor: *const u128) {
    debug_assert!(invalidation_type < 4);
    debug_assert!(crate::cpuid::supports_cpuid());
    // SAFETY:
    //
    // The CPUID instruction is available on this processor.
    unsafe { debug_assert!((crate::cpuid::cpuid_unchecked(0x7, 0x0).ebx >> 10) & 0b1 == 1) }

    // SAFETY:
    //
    // The processor supports the INVPCID feature.
    unsafe {
        core::arch::asm!(
            "invpcid {}, [{}]",
            in(reg) invalidation_type,
            in(reg) descriptor,
            options(readonly, nostack, preserves_flags)
        )
    }
}

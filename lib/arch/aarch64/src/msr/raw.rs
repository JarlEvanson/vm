//! Raw access to MSRs.

/// Wrapper around auto-generating raw read from system register functions.
macro_rules! sysreg_read {
    ($name:ident, $reg_name:literal) => {
        #[doc = concat!("Returns the contents of the `", $reg_name, "` register.")]
        ///
        /// # Safety
        ///
        #[doc = concat!("It must be safe to read from the `", $reg_name, "` register.")]
        #[inline(always)]
        pub unsafe fn $name() -> u64 {
            let val: u64;
            unsafe {
                core::arch::asm!(
                    concat!("mrs {val}, ", $reg_name),
                    val = out(reg) val,
                    options(nomem, nostack, preserves_flags),
                );
            }
            val
        }
    };
}

/// Wrapper around auto-generating raw write to system register functions.
macro_rules! sysreg_write {
    ($name:ident, $reg_name:literal) => {
        #[doc = concat!("Sets the contents of the `", $reg_name, "`.")]
        ///
        /// # Safety
        ///
        #[doc = concat!("It must be safe to write to the `", $reg_name, "` register and the new ")]
        #[doc = concat!("configuration of the `", $reg_name, "` register must be compatible with")]
        #[doc = "current state of the system."]
        #[inline(always)]
        pub unsafe fn $name(val: u64) {
            unsafe {
                core::arch::asm!(
                    concat!("msr ", $reg_name, ", {val}"),
                    val = in(reg) val,
                    options(nomem, nostack, preserves_flags),
                );
            }
        }
    };
}

/// Wrapper around auto-generating raw read from and write to system register functions.
macro_rules! sysreg_rw {
    ($read_name:ident, $write_name:ident, $reg_name:literal) => {
        sysreg_read! {$read_name, $reg_name}
        sysreg_write! {$write_name, $reg_name}
    };
}

sysreg_read! {read_current_el, "CurrentEL"}
sysreg_read! {read_id_aa64mmfr0_el1, "ID_AA64MMFR0_EL1"}
sysreg_rw! {read_sctlr_el1, write_sctlr_el1, "SCTLR_EL1"}
sysreg_rw! {read_tcr_el1, write_tcr_el1, "TCR_EL1"}

sysreg_rw! {read_ttbr0_el1, write_ttbr0_el1, "TTBR0_EL1"}
sysreg_rw! {read_ttbr1_el1, write_ttbr1_el1, "TTBR1_EL1"}

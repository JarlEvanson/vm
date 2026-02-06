//! Control register-related structures and functions.

#[expect(clippy::missing_docs_in_private_items)]
mod constants {
    pub const CR0_PE_SHIFT: u32 = 0;
    pub const CR0_PE_BIT: u64 = 1 << CR0_PE_SHIFT;

    pub const CR0_MP_SHIFT: u32 = 1;
    pub const CR0_MP_BIT: u64 = 1 << CR0_MP_SHIFT;

    pub const CR0_EM_SHIFT: u32 = 2;
    pub const CR0_EM_BIT: u64 = 1 << CR0_EM_SHIFT;

    pub const CR0_TS_SHIFT: u32 = 3;
    pub const CR0_TS_BIT: u64 = 1 << CR0_TS_SHIFT;

    pub const CR0_ET_SHIFT: u32 = 4;
    pub const CR0_ET_BIT: u64 = 1 << CR0_ET_SHIFT;

    pub const CR0_NE_SHIFT: u32 = 5;
    pub const CR0_NE_BIT: u64 = 1 << CR0_NE_SHIFT;

    pub const CR0_WP_SHIFT: u32 = 16;
    pub const CR0_WP_BIT: u64 = 1 << CR0_WP_SHIFT;

    pub const CR0_AM_SHIFT: u32 = 18;
    pub const CR0_AM_BIT: u64 = 1 << CR0_AM_SHIFT;

    pub const CR0_NW_SHIFT: u32 = 29;
    pub const CR0_NW_BIT: u64 = 1 << CR0_NW_SHIFT;

    pub const CR0_CD_SHIFT: u32 = 30;
    pub const CR0_CD_BIT: u64 = 1 << CR0_CD_SHIFT;

    pub const CR0_PG_SHIFT: u32 = 31;
    pub const CR0_PG_BIT: u64 = 1 << CR0_PG_SHIFT;

    pub const CR4_VME_SHIFT: u32 = 0;
    pub const CR4_VME_BIT: u64 = 1 << CR4_VME_SHIFT;

    pub const CR4_PVI_SHIFT: u32 = 1;
    pub const CR4_PVI_BIT: u64 = 1 << CR4_PVI_SHIFT;

    pub const CR4_TSD_SHIFT: u32 = 2;
    pub const CR4_TSD_BIT: u64 = 1 << CR4_TSD_SHIFT;

    pub const CR4_DE_SHIFT: u32 = 3;
    pub const CR4_DE_BIT: u64 = 1 << CR4_DE_SHIFT;

    pub const CR4_PSE_SHIFT: u32 = 4;
    pub const CR4_PSE_BIT: u64 = 1 << CR4_PSE_SHIFT;

    pub const CR4_PAE_SHIFT: u32 = 5;
    pub const CR4_PAE_BIT: u64 = 1 << CR4_PAE_SHIFT;

    pub const CR4_MCE_SHIFT: u32 = 6;
    pub const CR4_MCE_BIT: u64 = 1 << CR4_MCE_SHIFT;

    pub const CR4_PGE_SHIFT: u32 = 7;
    pub const CR4_PGE_BIT: u64 = 1 << CR4_PGE_SHIFT;

    pub const CR4_PCE_SHIFT: u32 = 8;
    pub const CR4_PCE_BIT: u64 = 1 << CR4_PCE_SHIFT;

    pub const CR4_OSFXSR_SHIFT: u32 = 9;
    pub const CR4_OSFXSR_BIT: u64 = 1 << CR4_OSFXSR_SHIFT;

    pub const CR4_OSXMMEXCPT_SHIFT: u32 = 10;
    pub const CR4_OSXMMEXCPT_BIT: u64 = 1 << CR4_OSXMMEXCPT_SHIFT;

    pub const CR4_UMIP_SHIFT: u32 = 11;
    pub const CR4_UMIP_BIT: u64 = 1 << CR4_UMIP_SHIFT;

    pub const CR4_LA57_SHIFT: u32 = 12;
    pub const CR4_LA57_BIT: u64 = 1 << CR4_LA57_SHIFT;

    pub const CR4_VMXE_SHIFT: u32 = 13;
    pub const CR4_VMXE_BIT: u64 = 1 << CR4_VMXE_SHIFT;

    pub const CR4_SMXE_SHIFT: u32 = 14;
    pub const CR4_SMXE_BIT: u64 = 1 << CR4_SMXE_SHIFT;

    pub const CR4_FSGSBASE_SHIFT: u32 = 16;
    pub const CR4_FSGSBASE_BIT: u64 = 1 << CR4_FSGSBASE_SHIFT;

    pub const CR4_PCIDE_SHIFT: u32 = 17;
    pub const CR4_PCIDE_BIT: u64 = 1 << CR4_PCIDE_SHIFT;

    pub const CR4_OSXSAVE_SHIFT: u32 = 18;
    pub const CR4_OSXSAVE_BIT: u64 = 1 << CR4_OSXSAVE_SHIFT;

    pub const CR4_SMEP_SHIFT: u32 = 20;
    pub const CR4_SMEP_BIT: u64 = 1 << CR4_SMEP_SHIFT;

    pub const CR4_SMAP_SHIFT: u32 = 21;
    pub const CR4_SMAP_BIT: u64 = 1 << CR4_SMAP_SHIFT;

    pub const CR4_PKE_SHIFT: u32 = 22;
    pub const CR4_PKE_BIT: u64 = 1 << CR4_PKE_SHIFT;
}

use core::fmt;

use constants::*;

/// The state of the `CR0` register.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cr0(u64);

impl Cr0 {
    /// Returns the value of the [`Cr0`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `CR0` register.
    pub unsafe fn get() -> Self {
        #[cfg(target_arch = "x86")]
        let mut cr0: u32 = 0;
        #[cfg(target_arch = "x86_64")]
        let mut cr0: u64 = 0;

        // SAFETY:
        //
        // `CR0` is always safe to retrieve in [`PrivilegeLevel::Ring0`].
        unsafe {
            core::arch::asm!(
                "mov {}, cr0", lateout(reg) cr0
            )
        }

        #[cfg(target_arch = "x86")]
        let cr0 = u64::from(cr0);

        Self::from_bits(cr0)
    }

    /// Sets the value of the [`Cr0`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `CR0` register and the new configuration of the `CR0`
    /// register must be compatible with the current state of the system.
    pub unsafe fn set(&self) {
        #[cfg(target_arch = "x86")]
        #[expect(clippy::as_conversions)]
        let cr0: u32 = self.to_bits() as u32;
        #[cfg(target_arch = "x86_64")]
        let cr0: u64 = self.to_bits();

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov cr4, {}", in(reg) cr0
            )
        }
    }

    /// Constructs a new [`Cr0`] from the provided bit representation.
    pub const fn from_bits(value: u64) -> Self {
        Self(value)
    }

    /// Returns the bit representation of the `CR0` register.
    pub const fn to_bits(&self) -> u64 {
        self.0
    }

    /// Returns `true` if protected mode is enabled.
    pub const fn pe(self) -> bool {
        self.0 & CR0_PE_BIT == CR0_PE_BIT
    }

    /// Sets whether protected mode should be enabled.
    pub const fn set_pe(self, enable: bool) -> Self {
        Self((self.0 & !CR0_PE_BIT) | (bool_as_u64(enable) << CR0_PE_SHIFT))
    }

    /// Returns `true` if the monitor coprocessor bit is set.
    pub const fn mp(self) -> bool {
        self.0 & CR0_MP_BIT == CR0_MP_BIT
    }

    /// Sets whether the monitor coprocessor bit should be set.
    pub const fn set_mp(self, value: bool) -> Self {
        Self((self.0 & !CR0_MP_BIT) | (bool_as_u64(value) << CR0_MP_SHIFT))
    }

    /// Returns `true` if x87 emulation is enabled.
    pub const fn em(self) -> bool {
        self.0 & CR0_EM_BIT == CR0_EM_BIT
    }

    /// Sets whether x87 emulation should be enabled.
    pub const fn set_em(self, enable: bool) -> Self {
        Self((self.0 & !CR0_EM_BIT) | (bool_as_u64(enable) << CR0_EM_SHIFT))
    }

    /// Returns `true` if this flag has not been cleared since the last task switch.
    pub const fn task_switched(self) -> bool {
        self.0 & CR0_TS_BIT == CR0_TS_BIT
    }

    /// Sets whether the task switched flag should be set.
    pub const fn set_task_switched(self, switched: bool) -> Self {
        Self((self.0 & !CR0_TS_BIT) | (bool_as_u64(switched) << CR0_TS_SHIFT))
    }

    /// Returns `true` if the extension type bit is set.
    pub const fn et(self) -> bool {
        self.0 & CR0_ET_BIT == CR0_ET_BIT
    }

    /// Sets whether the extension type bit is set.
    pub const fn set_et(self, value: bool) -> Self {
        Self((self.0 & !CR0_ET_BIT) | (bool_as_u64(value) << CR0_ET_SHIFT))
    }

    /// Returns `true` if native x87 error reporting is enabled.
    pub const fn numeric_error(self) -> bool {
        self.0 & CR0_NE_BIT == CR0_NE_BIT
    }

    /// Sets whether native x87 error reporting should be enabled.
    pub const fn set_numeric_error(self, enable: bool) -> Self {
        Self((self.0 & !CR0_NE_BIT) | (bool_as_u64(enable) << CR0_NE_SHIFT))
    }

    /// Returns `true` if write protection is enabled.
    pub const fn write_protection(self) -> bool {
        self.0 & CR0_WP_BIT == CR0_WP_BIT
    }

    /// Sets whether write protection should be enabled.
    pub const fn set_write_protection(self, enable: bool) -> Self {
        Self((self.0 & !CR0_WP_BIT) | (bool_as_u64(enable) << CR0_WP_SHIFT))
    }

    /// Returns `true` if automatic alignment checking is enabled.
    pub const fn alignment_mask(self) -> bool {
        self.0 & CR0_AM_BIT == CR0_AM_BIT
    }

    /// Sets whether automatic alignment checking should be enabled.
    pub const fn set_alignment_mask(self, enable: bool) -> Self {
        Self((self.0 & !CR0_AM_BIT) | (bool_as_u64(enable) << CR0_AM_SHIFT))
    }

    /// Returns `true` if not-write-through caching is enabled.
    pub const fn nw(self) -> bool {
        self.0 & CR0_NW_BIT == CR0_NW_BIT
    }

    /// Sets whether not-write-through caching should be enabled.
    pub const fn set_nw(self, enable: bool) -> Self {
        Self((self.0 & !CR0_NW_BIT) | (bool_as_u64(enable) << CR0_NW_SHIFT))
    }

    /// Returns `true` if caching is disabled.
    pub const fn cache_disable(self) -> bool {
        self.0 & CR0_CD_BIT == CR0_CD_BIT
    }

    /// Sets whether caching should be disabled.
    pub const fn set_cache_disable(self, disable: bool) -> Self {
        Self((self.0 & !CR0_CD_BIT) | (bool_as_u64(disable) << CR0_CD_SHIFT))
    }

    /// Returns `true` if paging is enabled.
    pub const fn paging(self) -> bool {
        self.0 & CR0_PG_BIT == CR0_PG_BIT
    }

    /// Sets whether paging should be enabled.
    pub const fn set_paging(self, enable: bool) -> Self {
        Self((self.0 & !CR0_PG_BIT) | (bool_as_u64(enable) << CR0_PG_SHIFT))
    }
}

impl fmt::Debug for Cr0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cr0")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field("pe", &self.pe())
            .field("mp", &self.mp())
            .field("em", &self.em())
            .field("ts", &self.task_switched())
            .field("et", &self.et())
            .field("ne", &self.numeric_error())
            .field("wp", &self.write_protection())
            .field("am", &self.alignment_mask())
            .field("nw", &self.nw())
            .field("cd", &self.cache_disable())
            .field("pg", &self.paging())
            .finish()
    }
}

impl fmt::Display for Cr0 {
    #[expect(unused_assignments)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CR0 {:#018x} [", self.0)?;

        let mut first = true;
        macro_rules! flag {
            ($cond:expr, $name:literal) => {
                if $cond {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;
                    write!(f, $name)?;
                }
            };
        }

        flag!(self.pe(), "PE");
        flag!(self.mp(), "MP");
        flag!(self.em(), "EM");
        flag!(self.task_switched(), "TS");
        flag!(self.et(), "ET");
        flag!(self.numeric_error(), "NE");
        flag!(self.write_protection(), "WP");
        flag!(self.alignment_mask(), "AM");
        flag!(self.nw(), "NW");
        flag!(self.cache_disable(), "CD");
        flag!(self.paging(), "PG");

        write!(f, "]")
    }
}

/// The state of the `CR2` register.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cr2(u64);

#[allow(missing_docs)]
impl Cr2 {
    /// Returns the value of the [`Cr2`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `CR2` register.
    pub unsafe fn get() -> Self {
        #[cfg(target_arch = "x86")]
        let mut cr2: u32 = 0;
        #[cfg(target_arch = "x86_64")]
        let mut cr2: u64 = 0;

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov {}, cr2", lateout(reg) cr2
            )
        }

        #[cfg(target_arch = "x86")]
        let cr2 = u64::from(cr2);

        Self::from_bits(cr2)
    }

    /// Sets the value of the [`Cr2`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `CR2` register.
    pub unsafe fn set(&self) {
        #[cfg(target_arch = "x86")]
        #[expect(clippy::as_conversions)]
        let cr2: u32 = self.to_bits() as u32;
        #[cfg(target_arch = "x86_64")]
        let cr2: u64 = self.to_bits();

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov cr2, {}", in(reg) cr2
            )
        }
    }

    /// Constructs a new [`Cr2`] from the provided bit representation.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns the bit representation of the [`Cr2`] register.
    pub const fn to_bits(&self) -> u64 {
        self.0
    }

    /// Returns the linear address that caused the page fault.
    pub const fn faulting_address(self) -> u64 {
        self.0
    }

    /// Sets the linear address that caused the page fault.
    pub const fn set_faulting_address(self, addr: u64) -> Self {
        Self(addr)
    }
}

impl fmt::Debug for Cr2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cr2")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field(
                "faulting_address",
                &format_args!("{:#018x}", self.faulting_address()),
            )
            .finish()
    }
}

impl fmt::Display for Cr2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CR2 {:#018x} [faulting_address: {:#018x}]",
            self.0,
            self.faulting_address()
        )
    }
}

/// The state of the `CR3` register.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cr3(u64);

#[allow(missing_docs)]
impl Cr3 {
    /// Returns the value of the [`Cr3`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `CR3` register.
    pub unsafe fn get() -> Self {
        #[cfg(target_arch = "x86")]
        let mut cr3: u32 = 0;
        #[cfg(target_arch = "x86_64")]
        let mut cr3: u64 = 0;

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov {}, cr3", lateout(reg) cr3
            )
        }

        #[cfg(target_arch = "x86")]
        let cr3 = u64::from(cr3);

        Self::from_bits(cr3)
    }

    /// Sets the value of the [`Cr3`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `CR3` register and the new configuration must be compatible
    /// with the current paging setup.
    pub unsafe fn set(&self) {
        #[cfg(target_arch = "x86")]
        #[expect(clippy::as_conversions)]
        let cr3: u32 = self.to_bits() as u32;
        #[cfg(target_arch = "x86_64")]
        let cr3: u64 = self.to_bits();

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov cr3, {}", in(reg) cr3
            )
        }
    }

    /// Constructs a new [`Cr3`] from the provided bit representation.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Returns the bit representation of the [`Cr3`] register.
    pub const fn to_bits(&self) -> u64 {
        self.0
    }
}

impl fmt::Debug for Cr3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cr3")
            .field("raw", &format_args!("{:#018x}", self.0))
            .finish()
    }
}

impl fmt::Display for Cr3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CR3 {:#018x}", self.0)?;

        write!(f, "]")
    }
}

/// The state of the `CR4` register.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cr4(u64);

#[allow(missing_docs)]
impl Cr4 {
    /// Returns the value of the [`Cr4`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `CR4` register.
    pub unsafe fn get() -> Self {
        #[cfg(target_arch = "x86")]
        let mut cr4: u32 = 0;
        #[cfg(target_arch = "x86_64")]
        let mut cr4: u64 = 0;

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov {}, cr4", lateout(reg) cr4
            )
        }

        #[cfg(target_arch = "x86")]
        let cr4 = u64::from(cr4);

        Self::from_bits(cr4)
    }

    /// Sets the value of the [`Cr4`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `CR4` register and the new configuration of the `CR4`
    /// register must be compatible with the current state of the system.
    pub unsafe fn set(&self) {
        #[cfg(target_arch = "x86")]
        #[expect(clippy::as_conversions)]
        let cr4: u32 = self.to_bits() as u32;
        #[cfg(target_arch = "x86_64")]
        let cr4: u64 = self.to_bits();

        // SAFETY:
        //
        // The invariants of this function suffice to make this operation safe.
        unsafe {
            core::arch::asm!(
                "mov cr4, {}", in(reg) cr4
            )
        }
    }

    /// Constructs a new [`Cr4`] from the provided bit representation.
    pub const fn from_bits(value: u64) -> Self {
        Self(value)
    }

    /// Returns the bit representation of the `CR4` register.
    pub const fn to_bits(&self) -> u64 {
        self.0
    }

    pub const fn vme(self) -> bool {
        self.0 & CR4_VME_BIT == CR4_VME_BIT
    }

    pub const fn set_vme(self, enable: bool) -> Self {
        Self((self.0 & !CR4_VME_BIT) | (bool_as_u64(enable) << CR4_VME_SHIFT))
    }

    pub const fn pvi(self) -> bool {
        self.0 & CR4_PVI_BIT == CR4_PVI_BIT
    }

    pub const fn set_pvi(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PVI_BIT) | (bool_as_u64(enable) << CR4_PVI_SHIFT))
    }

    pub const fn tsd(self) -> bool {
        self.0 & CR4_TSD_BIT == CR4_TSD_BIT
    }

    pub const fn set_tsd(self, enable: bool) -> Self {
        Self((self.0 & !CR4_TSD_BIT) | (bool_as_u64(enable) << CR4_TSD_SHIFT))
    }

    pub const fn de(self) -> bool {
        self.0 & CR4_DE_BIT == CR4_DE_BIT
    }

    pub const fn set_de(self, enable: bool) -> Self {
        Self((self.0 & !CR4_DE_BIT) | (bool_as_u64(enable) << CR4_DE_SHIFT))
    }

    pub const fn pse(self) -> bool {
        self.0 & CR4_PSE_BIT == CR4_PSE_BIT
    }

    pub const fn set_pse(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PSE_BIT) | (bool_as_u64(enable) << CR4_PSE_SHIFT))
    }

    pub const fn pae(self) -> bool {
        self.0 & CR4_PAE_BIT == CR4_PAE_BIT
    }

    pub const fn set_pae(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PAE_BIT) | (bool_as_u64(enable) << CR4_PAE_SHIFT))
    }

    pub const fn mce(self) -> bool {
        self.0 & CR4_MCE_BIT == CR4_MCE_BIT
    }

    pub const fn set_mce(self, enable: bool) -> Self {
        Self((self.0 & !CR4_MCE_BIT) | (bool_as_u64(enable) << CR4_MCE_SHIFT))
    }

    pub const fn pge(self) -> bool {
        self.0 & CR4_PGE_BIT == CR4_PGE_BIT
    }

    pub const fn set_pge(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PGE_BIT) | (bool_as_u64(enable) << CR4_PGE_SHIFT))
    }

    pub const fn pce(self) -> bool {
        self.0 & CR4_PCE_BIT == CR4_PCE_BIT
    }

    pub const fn set_pce(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PCE_BIT) | (bool_as_u64(enable) << CR4_PCE_SHIFT))
    }

    pub const fn osfxsr(self) -> bool {
        self.0 & CR4_OSFXSR_BIT == CR4_OSFXSR_BIT
    }

    pub const fn set_osfxsr(self, enable: bool) -> Self {
        Self((self.0 & !CR4_OSFXSR_BIT) | (bool_as_u64(enable) << CR4_OSFXSR_SHIFT))
    }

    pub const fn osxmmexcpt(self) -> bool {
        self.0 & CR4_OSXMMEXCPT_BIT == CR4_OSXMMEXCPT_BIT
    }

    pub const fn set_osxmmexcpt(self, enable: bool) -> Self {
        Self((self.0 & !CR4_OSXMMEXCPT_BIT) | (bool_as_u64(enable) << CR4_OSXMMEXCPT_SHIFT))
    }

    pub const fn umip(self) -> bool {
        self.0 & CR4_UMIP_BIT == CR4_UMIP_BIT
    }

    pub const fn set_umip(self, enable: bool) -> Self {
        Self((self.0 & !CR4_UMIP_BIT) | (bool_as_u64(enable) << CR4_UMIP_SHIFT))
    }

    pub const fn la57(self) -> bool {
        self.0 & CR4_LA57_BIT == CR4_LA57_BIT
    }

    pub const fn set_la57(self, enable: bool) -> Self {
        Self((self.0 & !CR4_LA57_BIT) | (bool_as_u64(enable) << CR4_LA57_SHIFT))
    }

    pub const fn vmxe(self) -> bool {
        self.0 & CR4_VMXE_BIT == CR4_VMXE_BIT
    }

    pub const fn set_vmxe(self, enable: bool) -> Self {
        Self((self.0 & !CR4_VMXE_BIT) | (bool_as_u64(enable) << CR4_VMXE_SHIFT))
    }

    pub const fn smxe(self) -> bool {
        self.0 & CR4_SMXE_BIT == CR4_SMXE_BIT
    }

    pub const fn set_smxe(self, enable: bool) -> Self {
        Self((self.0 & !CR4_SMXE_BIT) | (bool_as_u64(enable) << CR4_SMXE_SHIFT))
    }

    pub const fn fsgsbase(self) -> bool {
        self.0 & CR4_FSGSBASE_BIT == CR4_FSGSBASE_BIT
    }

    pub const fn set_fsgsbase(self, enable: bool) -> Self {
        Self((self.0 & !CR4_FSGSBASE_BIT) | (bool_as_u64(enable) << CR4_FSGSBASE_SHIFT))
    }

    pub const fn pcide(self) -> bool {
        self.0 & CR4_PCIDE_BIT == CR4_PCIDE_BIT
    }

    pub const fn set_pcide(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PCIDE_BIT) | (bool_as_u64(enable) << CR4_PCIDE_SHIFT))
    }

    pub const fn osxsave(self) -> bool {
        self.0 & CR4_OSXSAVE_BIT == CR4_OSXSAVE_BIT
    }

    pub const fn set_osxsave(self, enable: bool) -> Self {
        Self((self.0 & !CR4_OSXSAVE_BIT) | (bool_as_u64(enable) << CR4_OSXSAVE_SHIFT))
    }

    pub const fn smep(self) -> bool {
        self.0 & CR4_SMEP_BIT == CR4_SMEP_BIT
    }

    pub const fn set_smep(self, enable: bool) -> Self {
        Self((self.0 & !CR4_SMEP_BIT) | (bool_as_u64(enable) << CR4_SMEP_SHIFT))
    }

    pub const fn smap(self) -> bool {
        self.0 & CR4_SMAP_BIT == CR4_SMAP_BIT
    }

    pub const fn set_smap(self, enable: bool) -> Self {
        Self((self.0 & !CR4_SMAP_BIT) | (bool_as_u64(enable) << CR4_SMAP_SHIFT))
    }

    pub const fn pke(self) -> bool {
        self.0 & CR4_PKE_BIT == CR4_PKE_BIT
    }

    pub const fn set_pke(self, enable: bool) -> Self {
        Self((self.0 & !CR4_PKE_BIT) | (bool_as_u64(enable) << CR4_PKE_SHIFT))
    }
}

impl fmt::Debug for Cr4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cr4")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field("vme", &self.vme())
            .field("pvi", &self.pvi())
            .field("tsd", &self.tsd())
            .field("de", &self.de())
            .field("pse", &self.pse())
            .field("pae", &self.pae())
            .field("mce", &self.mce())
            .field("pge", &self.pge())
            .field("pce", &self.pce())
            .field("osfxsr", &self.osfxsr())
            .field("osxmmexcpt", &self.osxmmexcpt())
            .field("umip", &self.umip())
            .field("la57", &self.la57())
            .field("vmxe", &self.vmxe())
            .field("smxe", &self.smxe())
            .field("fsgsbase", &self.fsgsbase())
            .field("pcide", &self.pcide())
            .field("osxsave", &self.osxsave())
            .field("smep", &self.smep())
            .field("smap", &self.smap())
            .field("pke", &self.pke())
            .finish()
    }
}

impl fmt::Display for Cr4 {
    #[expect(unused_assignments)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CR4 {:#018x} [", self.0)?;

        let mut first = true;
        macro_rules! flag {
            ($cond:expr, $name:literal) => {
                if $cond {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;
                    write!(f, $name)?;
                }
            };
        }

        flag!(self.vme(), "VME");
        flag!(self.pvi(), "PVI");
        flag!(self.tsd(), "TSD");
        flag!(self.de(), "DE");
        flag!(self.pse(), "PSE");
        flag!(self.pae(), "PAE");
        flag!(self.mce(), "MCE");
        flag!(self.pge(), "PGE");
        flag!(self.pce(), "PCE");
        flag!(self.osfxsr(), "OSFXSR");
        flag!(self.osxmmexcpt(), "OSXMMEXCPT");
        flag!(self.umip(), "UMIP");
        flag!(self.la57(), "LA57");
        flag!(self.vmxe(), "VMXE");
        flag!(self.smxe(), "SMXE");
        flag!(self.fsgsbase(), "FSGSBASE");
        flag!(self.pcide(), "PCIDE");
        flag!(self.osxsave(), "OSXSAVE");
        flag!(self.smep(), "SMEP");
        flag!(self.smap(), "SMAP");
        flag!(self.pke(), "PKE");

        write!(f, "]")
    }
}

#[expect(clippy::as_conversions, clippy::missing_docs_in_private_items)]
const fn bool_as_u64(value: bool) -> u64 {
    value as u64
}

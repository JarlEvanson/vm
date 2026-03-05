//! Model-Specific Register-related functions.

use core::fmt;

use crate::common::{EL, Granule, PhysicalAddressSpaceSize};

pub mod raw;

/// The state of the `CurrentEL` register.
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CurrentEl(u64);

impl CurrentEl {
    /// Returns the value of the [`CurrentEl`] register.
    pub fn get() -> Self {
        // SAFETY:
        //
        // It is always safe to read from the `CurrentEL` register.
        let val = unsafe { raw::read_current_el() };
        Self(val)
    }

    /// Returns the current [`EL`] of the processor.
    pub const fn el(self) -> EL {
        match (self.0 >> 2) & 0b11 {
            0 => EL::EL0,
            1 => EL::EL1,
            2 => EL::EL2,
            3 => EL::EL3,
            _ => unreachable!(),
        }
    }
}

/// The state of the `ID_AA64MMFR0_EL1` register.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Aarch64MemoryModelFeatureRegister0EL1(u64);

impl Aarch64MemoryModelFeatureRegister0EL1 {
    /// Returns the value of the [`Aarch64MemoryModelFeatureRegister0EL1`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `SCTLR_EL1` register.
    pub unsafe fn get() -> Self {
        // SAFETY:
        //
        // It is always safe to read from the `ID_AA64MMFR0_EL1` register.
        let val = unsafe { raw::read_id_aa64mmfr0_el1() };
        Self(val)
    }

    /// Returns the number of bits that are supported for physical addresses.
    pub const fn physical_address_bits(self) -> PhysicalAddressSpaceSize {
        PhysicalAddressSpaceSize::from_bits(self.0 as u8 & 0xF)
    }

    /// Returns the number of bits used to represent an ASID.
    pub const fn asid_bits(self) -> u8 {
        match (self.0 >> 4) & 0xF {
            0b0000 => 8,
            0b0010 => 16,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports mixed-endian configurations.
    pub const fn mixed_endian(self) -> bool {
        match (self.0 >> 8) & 0xF {
            0b0000 => false,
            0b0001 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports a distinction between `Secure` and `Non-secure`
    /// memory.
    pub const fn secure_memory(self) -> bool {
        match (self.0 >> 12) & 0xF {
            0b0000 => false,
            0b0001 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports mixed-endian configurations at EL0 only.
    pub const fn mixed_endian_el0_only(self) -> bool {
        match (self.0 >> 16) & 0xF {
            0b0000 => false,
            0b0001 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `16-KiB` memory translation granule size.
    pub const fn granule_16_supported(self) -> bool {
        match (self.0 >> 20) & 0xF {
            0b0000 => false,
            0b0001 => true,
            0b0010 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `16-KiB` memory translation granule size with
    /// 52-bit support.
    pub const fn granule_16_supported_52_bits(self) -> bool {
        match (self.0 >> 20) & 0xF {
            0b0000 => false,
            0b0001 => false,
            0b0010 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `64-KiB` memory translation granule size.
    pub const fn granule_64_supported(self) -> bool {
        match (self.0 >> 24) & 0xF {
            0b0000 => true,
            0b1111 => false,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `4-KiB` memory translation granule size.
    pub const fn granule_4_supported(self) -> bool {
        match (self.0 >> 28) & 0xF {
            0b0000 => true,
            0b0001 => true,
            0b1111 => false,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `4-KiB` memory translation granule size with
    /// 52-bit support.
    pub const fn granule_4_supported_52_bits(self) -> bool {
        match (self.0 >> 28) & 0xF {
            0b0000 => false,
            0b0001 => true,
            0b1111 => false,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `16-KiB` memory translation granule size at stage
    /// 2.
    pub const fn granule_16_supported_stage_2(self) -> bool {
        match (self.0 >> 32) & 0xF {
            0b0000 => self.granule_16_supported(),
            0b0001 => false,
            0b0010 => true,
            0b0011 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `16-KiB` memory translation granule size at stage
    /// 2 with 52-bit support.
    pub const fn granule_16_supported_stage_2_52_bits(self) -> bool {
        match (self.0 >> 32) & 0xF {
            0b0000 => self.granule_16_supported_52_bits(),
            0b0001 => false,
            0b0010 => false,
            0b0011 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `64-KiB` memory translation granule size at stage
    /// 2.
    pub const fn granule_64_supported_stage_2(self) -> bool {
        match (self.0 >> 36) & 0xF {
            0b0000 => self.granule_64_supported(),
            0b0001 => false,
            0b0010 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `4-KiB` memory translation granule size at stage
    /// 2.
    pub const fn granule_4_supported_stage_2(self) -> bool {
        match (self.0 >> 40) & 0xF {
            0b0000 => self.granule_4_supported(),
            0b0001 => false,
            0b0010 => true,
            0b0011 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if the processor supports `4-KiB` memory translation granule size at stage
    /// 2 with 52-bit support.
    pub const fn granule_4_supported_stage_2_52_bits(self) -> bool {
        match (self.0 >> 40) & 0xF {
            0b0000 => self.granule_4_supported_52_bits(),
            0b0001 => false,
            0b0010 => false,
            0b0011 => true,
            _ => unimplemented!(),
        }
    }

    /// Returns `true` if disabling context-synchronizing exception entry and exit is supported.
    pub const fn disable_context_sync_exceptions(self) -> bool {
        match (self.0 >> 44) & 0xF {
            0b0000 => true,
            0b0001 => false,
            _ => unimplemented!(),
        }
    }
}

impl fmt::Debug for Aarch64MemoryModelFeatureRegister0EL1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aarch64MemoryModelFeatureRegister0EL1")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field("physical_address_bits", &self.physical_address_bits())
            .field("asid_bits", &self.asid_bits())
            .field("mixed_endian", &self.mixed_endian())
            .field("secure_memory", &self.secure_memory())
            .field("mixed_endian_el0_only", &self.mixed_endian_el0_only())
            .field("granule_16_supported", &self.granule_16_supported())
            .field(
                "granule_16_supported_52_bits",
                &self.granule_16_supported_52_bits(),
            )
            .field("granule_64_supported", &self.granule_64_supported())
            .field("granule_4_supported", &self.granule_4_supported())
            .field(
                "granule_4_supported_52_bits",
                &self.granule_4_supported_52_bits(),
            )
            .field(
                "granule_16_supported_stage_2",
                &self.granule_16_supported_stage_2(),
            )
            .field(
                "granule_16_supported_stage_2_52_bits",
                &self.granule_16_supported_stage_2_52_bits(),
            )
            .field(
                "granule_64_supported_stage_2",
                &self.granule_64_supported_stage_2(),
            )
            .field(
                "granule_4_supported_stage_2",
                &self.granule_4_supported_stage_2(),
            )
            .field(
                "granule_4_supported_stage_2_52_bits",
                &self.granule_4_supported_stage_2_52_bits(),
            )
            .field(
                "disable_context_sync_exceptions",
                &self.disable_context_sync_exceptions(),
            )
            .finish()
    }
}

/// The state of the `SCTLR_EL1` register.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SctlrEL1(u64);

impl SctlrEL1 {
    /// Returns the value of the [`SctlrEL1`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `SCTLR_EL1` register.
    pub unsafe fn get() -> Self {
        // SAFETY:
        //
        // It is always safe to read from the `SCTLR_EL1` register.
        let val = unsafe { raw::read_sctlr_el1() };
        Self(val)
    }

    /// Sets the value of the [`SctlrEL1`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `SCTLR_EL1` register and the new configuration of the
    /// `SCTLR_EL1` register must be compatible with the current state of the system.
    pub unsafe fn set(self) {
        unsafe { raw::write_sctlr_el1(self.0) }
    }

    /// Returns `true` if the MMU is enabled for `EL1` and `EL0` stage 1 address translation.
    pub const fn mmu_enable(self) -> bool {
        (self.0 & 0b1) == 0b1
    }

    /// Sets whether the MMU should be enabled for `EL1` and `EL0` stage 1 address translation.
    pub const fn set_mmu_enable(self, enable: bool) -> Self {
        Self((self.0 & !0b1) | (enable as u64))
    }
}

impl fmt::Debug for SctlrEL1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SctlrEL1")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field("mmu_enable", &self.mmu_enable())
            .finish()
    }
}

/// The state of the `TCR_EL1` register.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct TcrEL1(u64);

impl TcrEL1 {
    /// Returns the value of the [`TcrEL1`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to read from the `TCR_EL1` register.
    pub unsafe fn get() -> Self {
        // SAFETY:
        //
        // It is always safe to read from the `TCR_EL1` register.
        let val = unsafe { raw::read_tcr_el1() };
        Self(val)
    }

    /// Sets the value of the [`TcrEL1`] register.
    ///
    /// # Safety
    ///
    /// It must be safe to write to the `TCR_EL1` register and the new configuration of the
    /// `TCR_EL1` register must be compatible with the current state of the system.
    pub unsafe fn set(self) {
        unsafe { raw::write_tcr_el1(self.0) }
    }

    /// Returns the size offset of the memory region addressed by [`Ttbr0EL1`].
    pub const fn size_offset_0(self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    /// Sets the size offset of the memory region addressed by [`Ttbr0EL1`].
    pub const fn set_size_offset_0(self, size: u8) -> Self {
        Self((self.0 & !0x3F) | (size as u64 & 0x3F))
    }

    /// Returns `true` if a translation table walk is *not* performed on a TLB miss, for an address
    /// that is translated using [`Ttbr0EL1`].
    pub const fn walk_disable_0(self) -> bool {
        ((self.0 >> 7) & 0b1) == 0b1
    }

    /// Sets whether a translation table walk is *not* performed on a TLB miss, for an address
    /// that is translated using [`Ttbr0EL1`].
    pub const fn set_walk_disable_0(self, disable: bool) -> Self {
        Self((self.0 & !(0b1 << 7)) & ((disable as u64) << 7))
    }

    /// Returns the inner cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr0EL1`].
    pub const fn inner_cacheability_attr_0(self) -> u8 {
        ((self.0 >> 8) & 0b11) as u8
    }

    /// Returns the inner cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr0EL1`].
    pub const fn set_inner_cacheability_attr_0(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 8)) & ((attr as u64 & 0b11) << 8))
    }

    /// Returns the outer cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr0EL1`].
    pub const fn outer_cacheability_attr_0(self) -> u8 {
        ((self.0 >> 10) & 0b11) as u8
    }

    /// Returns the outer cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr0EL1`].
    pub const fn set_outer_cacheability_attr_0(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 10)) & ((attr as u64 & 0b11) << 10))
    }

    /// Returns the shareability attribute for memory associated with translation table walks using
    /// [`Ttbr0EL1`].
    pub const fn shareability_attr_0(self) -> u8 {
        ((self.0 >> 12) & 0b11) as u8
    }

    /// Sets the shareability attribute for memory associated with translation table walks using
    /// [`Ttbr0EL1`].
    pub const fn set_shareability_attr_0(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 12)) & ((attr as u64 & 0b11) << 12))
    }

    /// Returns the translation [`Granule`] size for [`Ttbr0EL1`].
    pub const fn translation_granule_0(self) -> Granule {
        match (self.0 >> 14) & 0b11 {
            0b00 => Granule::Page4KiB,
            0b01 => Granule::Page64KiB,
            0b10 => Granule::Page16KiB,
            _ => unreachable!(),
        }
    }

    /// Sets the translation [`Granule`] size for [`Ttbr0EL1`].
    pub const fn set_translation_granule_0(self, granule: Granule) -> Self {
        let val = match granule {
            Granule::Page4KiB => 0b00,
            Granule::Page16KiB => 0b10,
            Granule::Page64KiB => 0b01,
        };

        Self((self.0 & !(0b11 << 14)) & ((val & 0b11) << 14))
    }

    /// Returns the size offset of the memory region addressed by [`Ttbr1EL1`].
    pub const fn size_offset_1(self) -> u8 {
        ((self.0 >> 16) & 0x3F) as u8
    }

    /// Sets the size offset of the memory region addressed by [`Ttbr1EL1`].
    pub const fn set_size_offset_1(self, size: u8) -> Self {
        Self((self.0 & !(0x3F << 16)) | (size as u64 & 0x3F) << 16)
    }

    /// Returns `true` if a translation table walk is *not* performed on a TLB miss, for an address
    /// that is translated using [`Ttbr1EL1`].
    pub const fn walk_disable_1(self) -> bool {
        ((self.0 >> 23) & 0b1) == 0b1
    }

    /// Sets whether a translation table walk is *not* performed on a TLB miss, for an address
    /// that is translated using [`Ttbr1EL1`].
    pub const fn set_walk_disable_1(self, disable: bool) -> Self {
        Self((self.0 & !(0b1 << 23)) & ((disable as u64) << 23))
    }

    /// Returns the inner cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr1EL1`].
    pub const fn inner_cacheability_attr_1(self) -> u8 {
        ((self.0 >> 24) & 0b11) as u8
    }

    /// Returns the inner cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr1EL1`].
    pub const fn set_inner_cacheability_attr_1(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 24)) & ((attr as u64 & 0b11) << 24))
    }

    /// Returns the outer cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr1EL1`].
    pub const fn outer_cacheability_attr_1(self) -> u8 {
        ((self.0 >> 26) & 0b11) as u8
    }

    /// Returns the outer cacheability attribute for memory associated with translation table
    /// walkes using [`Ttbr1EL1`].
    pub const fn set_outer_cacheability_attr_1(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 26)) & ((attr as u64 & 0b11) << 26))
    }

    /// Returns the shareability attribute for memory associated with translation table walks using
    /// [`Ttbr1EL1`].
    pub const fn shareability_attr_1(self) -> u8 {
        ((self.0 >> 28) & 0b11) as u8
    }

    /// Sets the shareability attribute for memory associated with translation table walks using
    /// [`Ttbr1EL1`].
    pub const fn set_shareability_attr_1(self, attr: u8) -> Self {
        Self((self.0 & !(0b11 << 28)) & ((attr as u64 & 0b11) << 28))
    }

    /// Returns the translation granule size for [`Ttbr1EL1`].
    pub const fn translation_granule_1(self) -> Granule {
        match (self.0 >> 30) & 0b11 {
            0b10 => Granule::Page4KiB,
            0b01 => Granule::Page16KiB,
            0b11 => Granule::Page64KiB,
            _ => unreachable!(),
        }
    }

    /// Sets the translation [`Granule`] size for [`Ttbr1EL1`].
    pub const fn set_translation_granule_1(self, granule: Granule) -> Self {
        let val = match granule {
            Granule::Page4KiB => 0b10,
            Granule::Page16KiB => 0b01,
            Granule::Page64KiB => 0b11,
        };

        Self((self.0 & !(0b11 << 30)) & ((val & 0b11) << 30))
    }

    /// Returns the intermediate physical address size for this translation scheme.
    pub const fn ipas(self) -> PhysicalAddressSpaceSize {
        PhysicalAddressSpaceSize::from_bits(((self.0 >> 32) & 0b111) as u8)
    }

    /// Sets the intermediate physical address size for this translation scheme.
    pub const fn set_ipas(self, size: PhysicalAddressSpaceSize) -> Self {
        Self((self.0 & !(0b111 << 32)) & ((size.to_bits() as u64 & 0b111) << 32))
    }
}

impl fmt::Debug for TcrEL1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SctlrEL1")
            .field("raw", &format_args!("{:#018x}", self.0))
            .field("size_offset_0", &self.size_offset_0())
            .field("walk_disable_0", &self.walk_disable_0())
            .field(
                "inner_cacheability_attr_0",
                &self.inner_cacheability_attr_0(),
            )
            .field(
                "outer_cacheability_attr_0",
                &self.outer_cacheability_attr_0(),
            )
            .field("shareability_0", &self.shareability_attr_0())
            .field("translation_granule_0", &self.translation_granule_0())
            .field("size_offset_1", &self.size_offset_1())
            .field("walk_disable_1", &self.walk_disable_1())
            .field(
                "inner_cacheability_attr_1",
                &self.inner_cacheability_attr_1(),
            )
            .field(
                "outer_cacheability_attr_1",
                &self.outer_cacheability_attr_1(),
            )
            .field("shareability_1", &self.shareability_attr_1())
            .field("translation_granule_1", &self.translation_granule_1())
            .field("ipas", &self.ipas())
            .finish()
    }
}

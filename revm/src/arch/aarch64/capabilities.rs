//! Architectural capability support detection for `aarch64`.
#![expect(clippy::missing_docs_in_private_items)]

use aarch64::msr::raw::{read_id_aa64mmfr0_el1, read_midr_el1};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ArchCapabilities {
    vendor: Vendor,

    physical_bits: u8,

    granule_4: u8,
    granule_16: u8,
    granule_64: u8,

    stage_2_granule_4: u8,
    stage_2_granule_16: u8,
    stage_2_granule_64: u8,
}

impl ArchCapabilities {
    pub const fn initial() -> ArchCapabilities {
        Self {
            vendor: Vendor::Reserved,

            physical_bits: 0,

            granule_4: 0,
            granule_16: 0,
            granule_64: 0,

            stage_2_granule_4: 0,
            stage_2_granule_16: 0,
            stage_2_granule_64: 0,
        }
    }

    pub fn new() -> ArchCapabilities {
        let mut support = Self::initial();

        // SAFETY:
        //
        // This program runs at EL1 or EL2 and thus is safe to execute.
        let midr_el1 = unsafe { read_midr_el1() };
        support.vendor = match (midr_el1 >> 24) & 0xFF {
            0x00 => Vendor::Reserved,
            0x41 => Vendor::Arm,
            0x42 => Vendor::Broadcom,
            0x43 => Vendor::Cavium,
            0x44 => Vendor::DigitalEquipment,
            0x46 => Vendor::Fujitsu,
            0x49 => Vendor::InfineonTechnologies,
            0x4D => Vendor::MotorolaOrFreescale,
            0x4E => Vendor::Nvidia,
            0x50 => Vendor::AppliedMicroCircuits,
            0x51 => Vendor::Qualcomm,
            0x56 => Vendor::MarvellInternational,
            0x69 => Vendor::Intel,
            0xC0 => Vendor::Ampere,
            _ => Vendor::Unknown,
        };

        // SAFETY:
        //
        // This program runs at EL1 or EL2 and thus is safe to execute.
        let id_aa64mmfr0_el1 = unsafe { read_id_aa64mmfr0_el1() };
        support.physical_bits = match id_aa64mmfr0_el1 & 0b1111 {
            0b0000 => 32,
            0b0001 => 36,
            0b0010 => 40,
            0b0011 => 42,
            0b0100 => 44,
            0b0101 => 48,
            0b0110 => 52,
            0b0111 => 56,
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.PARange value: {val:#b}"),
        };

        support.granule_4 = match (id_aa64mmfr0_el1 >> 28) & 0b1111 {
            0b0000 => support.physical_bits.min(48),
            0b0001 => support.physical_bits.min(52),
            0b1111 => 0,
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran4 value: {val:#b}"),
        };
        support.granule_16 = match (id_aa64mmfr0_el1 >> 20) & 0b1111 {
            0b0000 => 0,
            0b0001 => support.physical_bits.min(48),
            0b0010 => support.physical_bits.min(52),
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran16 value: {val:#b}"),
        };
        support.granule_64 = match (id_aa64mmfr0_el1 >> 24) & 0b1111 {
            0b0000 => support.physical_bits.min(52),
            0b1111 => 0,
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran64 value: {val:#b}"),
        };

        support.stage_2_granule_4 = match (id_aa64mmfr0_el1 >> 40) & 0b1111 {
            0b0000 => support.granule_16,
            0b0001 => 0,
            0b0010 => support.physical_bits.min(48),
            0b0011 => support.physical_bits.min(52),
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran4_2 value: {val:#b}"),
        };
        support.stage_2_granule_16 = match (id_aa64mmfr0_el1 >> 32) & 0b1111 {
            0b0000 => support.granule_16,
            0b0001 => 0,
            0b0010 => support.physical_bits.min(48),
            0b0011 => support.physical_bits.min(52),
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran16_2 value: {val:#b}"),
        };
        support.stage_2_granule_64 = match (id_aa64mmfr0_el1 >> 36) & 0b1111 {
            0b0000 => support.granule_64,
            0b0001 => 0,
            0b0010 => support.physical_bits.min(52),
            val => unimplemented!("unknown ID_AA64MMFR0_EL1.TGran64_2 value: {val:#b}"),
        };

        support
    }

    /// Returns the vendor with which this system is associated.
    pub const fn vendor(&self) -> Vendor {
        self.vendor
    }

    /// Returns the implemented number of physical bits.
    pub const fn physical_bits(&self) -> u8 {
        self.physical_bits
    }

    /// Returns the maximum physical address width supported for the 4 KiB translation granule.
    ///
    /// Returns [`None`] if the 4 KiB translation granule is not supported.
    pub const fn granule_4(&self) -> Option<u8> {
        if self.granule_4 != 0 {
            Some(self.granule_4)
        } else {
            None
        }
    }

    /// Returns the maximum physical address width supported for the 16 KiB translation granule.
    ///
    /// Returns [`None`] if the 16 KiB translation granule is not supported.
    pub const fn granule_16(&self) -> Option<u8> {
        if self.granule_16 != 0 {
            Some(self.granule_16)
        } else {
            None
        }
    }

    /// Returns the maximum physical address width supported for the 64 KiB translation granule.
    ///
    /// Returns [`None`] if the 64 KiB translation granule is not supported.
    pub const fn granule_64(&self) -> Option<u8> {
        if self.granule_64 != 0 {
            Some(self.granule_64)
        } else {
            None
        }
    }

    /// Returns the maximum physical address width supported for the 4 KiB translation granule
    /// during stage 2 translation.
    ///
    /// Returns [`None`] if the 4 KiB translation granule is not supported.
    pub const fn stage_2_granule_4(&self) -> Option<u8> {
        if self.stage_2_granule_4 != 0 {
            Some(self.stage_2_granule_4)
        } else {
            None
        }
    }

    /// Returns the maximum physical address width supported for the 16 KiB translation granule
    /// during stage 2 translation.
    ///
    /// Returns [`None`] if the 16 KiB translation granule is not supported.
    pub const fn stage_2_granule_16(&self) -> Option<u8> {
        if self.stage_2_granule_16 != 0 {
            Some(self.stage_2_granule_16)
        } else {
            None
        }
    }

    /// Returns the maximum physical address width supported for the 64 KiB translation granule
    /// during stage two translation.
    ///
    /// Returns [`None`] if the 64 KiB translation granule is not supported.
    pub const fn stage_2_granule_64(&self) -> Option<u8> {
        if self.stage_2_granule_64 != 0 {
            Some(self.stage_2_granule_64)
        } else {
            None
        }
    }
}

/// The CPU vendor.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Vendor {
    /// Reserved for software use.
    Reserved,
    /// Arm Limited.
    Arm,
    /// Broadcom Corporation.
    Broadcom,
    /// Cavium Incorporated.
    Cavium,
    /// Digital Equipment Corporation.
    DigitalEquipment,
    /// Fujitsu Limited.
    Fujitsu,
    /// Infineon Technologies AG.
    InfineonTechnologies,
    /// Motorola or Freescale Semiconductor Incorporated.
    MotorolaOrFreescale,
    /// NVIDIA Corporation.
    Nvidia,
    /// Applied Micro Circuits Corporation.
    AppliedMicroCircuits,
    /// Qualcomm Incorporated.
    Qualcomm,
    /// Marvell International Limited.
    MarvellInternational,
    /// Intel Corporation.
    Intel,
    /// Ampere Computing.
    Ampere,
    /// A unknown processor vendor.
    Unknown,
}

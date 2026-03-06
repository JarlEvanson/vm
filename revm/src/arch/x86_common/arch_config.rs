//! Architectural configuration data.
#![expect(clippy::missing_docs_in_private_items)]

use core::{error, fmt};

use x86_common::cpuid::{Cpuid, cpuid_unchecked, supports_cpuid};

#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ArchConfig {
    cpuid: bool,
    vendor: Vendor,
    max_basic_cpuid: u32,

    pse: bool,
    pse36: bool,
    msr: bool,
    pae: bool,
    apic: bool,
    mtrr: bool,
    pat: bool,

    la57: bool,

    max_extended_cpuid: u32,

    nxe: bool,
    gib_pages: bool,
    long_mode: bool,

    phys_addr_size: u8,
    guest_phys_addr_size: u8,
}

impl ArchConfig {
    pub const fn initial() -> ArchConfig {
        ArchConfig {
            cpuid: false,
            vendor: Vendor::Intel,

            max_basic_cpuid: 0,

            pse: false,
            msr: false,
            pae: false,
            apic: false,
            mtrr: false,
            pat: false,
            pse36: false,

            la57: false,

            max_extended_cpuid: 0,

            nxe: false,
            gib_pages: false,
            long_mode: false,

            phys_addr_size: 0,
            guest_phys_addr_size: 0,
        }
    }

    pub fn new() -> Result<ArchConfig, ArchConfigError> {
        let mut arch_config = ArchConfig::initial();

        arch_config.cpuid = supports_cpuid();
        'cpuid: {
            if !arch_config.cpuid {
                break 'cpuid;
            }

            let Cpuid {
                eax: max_basic,
                ebx,
                ecx,
                edx,
                // SAFETY:
                //
                // The `CPUID` instruction is supported.
            } = unsafe { cpuid_unchecked(0x0, 0) };

            if ebx == 0x756E_6547 && ecx == 0x6C65_746E && edx == 0x4965_6E69 {
                arch_config.vendor = Vendor::Intel;
            } else if ebx == 0x6874_7541 && ecx == 0x444D_4163 && edx == 0x6974_6E65 {
                arch_config.vendor = Vendor::Amd;
            } else {
                arch_config.vendor = Vendor::Unknown;
                break 'cpuid;
            }

            arch_config.max_basic_cpuid = max_basic;
            'basic: {
                if arch_config.max_basic_cpuid < 0x1 {
                    break 'basic;
                }

                // SAFETY:
                //
                // The `CPUID` instruction is supported.
                let Cpuid {
                    eax: _,
                    ebx: _,
                    ecx: _,
                    edx,
                } = unsafe { cpuid_unchecked(0x1, 0) };
                arch_config.pse = ((edx >> 3) & 0b1) == 0b1;
                arch_config.msr = ((edx >> 5) & 0b1) == 0b1;
                arch_config.pae = ((edx >> 6) & 0b1) == 0b1;
                arch_config.apic = ((edx >> 9) & 0b1) == 0b1;
                arch_config.mtrr = ((edx >> 9) & 0b1) == 0b1;
                arch_config.pat = ((edx >> 9) & 0b1) == 0b1;
                arch_config.pse36 = ((edx >> 17) & 0b1) == 0b1;

                if arch_config.max_basic_cpuid < 0x7 {
                    break 'basic;
                }

                // SAFETY:
                //
                // The `CPUID` instruction is supported.
                let Cpuid {
                    eax: _subleaf_max,
                    ebx: _,
                    ecx,
                    edx: _,
                } = unsafe { cpuid_unchecked(0x7, 0) };

                arch_config.la57 = ((ecx >> 16) & 0b1) == 0b1;
            }

            // SAFETY:
            //
            // The `CPUID` instruction is supported.
            let Cpuid {
                eax: max_extended,
                ebx: _,
                ecx: _,
                edx: _,
            } = unsafe { cpuid_unchecked(0x80000000, 0) };

            arch_config.max_extended_cpuid = max_extended;
            'extended: {
                if arch_config.max_extended_cpuid < 0x80000001 {
                    break 'extended;
                }

                // SAFETY:
                //
                // The `CPUID` instruction is supported.
                let Cpuid {
                    eax: _,
                    ebx: _,
                    ecx: _,
                    edx,
                } = unsafe { cpuid_unchecked(0x80000001, 0) };

                arch_config.nxe = ((edx >> 20) & 0b1) == 0b1;
                arch_config.gib_pages = ((edx >> 26) & 0b1) == 0b1;
                arch_config.long_mode = ((edx >> 29) & 0b1) == 0b1;

                if arch_config.max_extended_cpuid < 0x80000008 {
                    break 'extended;
                }

                // SAFETY:
                //
                // The `CPUID` instruction is supported.
                let Cpuid {
                    eax,
                    ebx: _,
                    ecx: _,
                    edx: _,
                } = unsafe { cpuid_unchecked(0x80000008, 0) };

                arch_config.phys_addr_size = (eax & 0xFF) as u8;
                arch_config.guest_phys_addr_size = ((eax >> 16) & 0xFF) as u8;
            }
        }

        if arch_config.pae {
            arch_config.phys_addr_size = arch_config.phys_addr_size.max(36);
        }

        Ok(arch_config)
    }
}

/// The CPU vendor.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Vendor {
    /// `Intel`.
    Intel,
    /// `AMD`.
    Amd,
    /// A unknown processor vendor.
    Unknown,
}

/// Various errors that may occur while gather [`ArchConfig`] data.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ArchConfigError {}

impl fmt::Display for ArchConfigError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl error::Error for ArchConfigError {}

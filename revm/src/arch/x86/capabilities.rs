//! Architectural capability support detection for `i686` and `x86_64`.
#![expect(clippy::missing_docs_in_private_items)]

use x86::cpuid::{Cpuid, cpuid_unchecked, supports_cpuid};

#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ArchCapabilities {
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

impl ArchCapabilities {
    pub const fn initial() -> ArchCapabilities {
        ArchCapabilities {
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

    pub fn new() -> ArchCapabilities {
        let mut support = ArchCapabilities::initial();

        support.cpuid = supports_cpuid();
        'cpuid: {
            if !support.cpuid {
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
                support.vendor = Vendor::Intel;
            } else if ebx == 0x6874_7541 && ecx == 0x444D_4163 && edx == 0x6974_6E65 {
                support.vendor = Vendor::Amd;
            } else {
                support.vendor = Vendor::Unknown;
                break 'cpuid;
            }

            support.max_basic_cpuid = max_basic;
            'basic: {
                if support.max_basic_cpuid < 0x1 {
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
                support.pse = ((edx >> 3) & 0b1) == 0b1;
                support.msr = ((edx >> 5) & 0b1) == 0b1;
                support.pae = ((edx >> 6) & 0b1) == 0b1;
                support.apic = ((edx >> 9) & 0b1) == 0b1;
                support.mtrr = ((edx >> 9) & 0b1) == 0b1;
                support.pat = ((edx >> 9) & 0b1) == 0b1;
                support.pse36 = ((edx >> 17) & 0b1) == 0b1;

                if support.max_basic_cpuid < 0x7 {
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

                support.la57 = ((ecx >> 16) & 0b1) == 0b1;
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

            support.max_extended_cpuid = max_extended;
            'extended: {
                if support.max_extended_cpuid < 0x80000001 {
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

                support.nxe = ((edx >> 20) & 0b1) == 0b1;
                support.gib_pages = ((edx >> 26) & 0b1) == 0b1;
                support.long_mode = ((edx >> 29) & 0b1) == 0b1;

                if support.max_extended_cpuid < 0x80000008 {
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

                support.phys_addr_size = (eax & 0xFF) as u8;
                support.guest_phys_addr_size = ((eax >> 16) & 0xFF) as u8;
            }
        }

        if support.pae {
            support.phys_addr_size = support.phys_addr_size.max(36);
        }

        support
    }

    /// Returns `true` if the CPUID instruction is supported.
    pub const fn cpuid_supported(&self) -> bool {
        self.cpuid
    }

    /// Returns the [`Vendor`] with which this system is associated.
    pub const fn vendor(&self) -> Vendor {
        self.vendor
    }

    /// Returns the maximum basic CPUID leaf supported by the processor.
    pub const fn max_basic_cpuid(&self) -> u32 {
        self.max_basic_cpuid
    }

    /// Returns `true` if Page Size Extensions (PSE) are supported.
    pub const fn pse_supported(&self) -> bool {
        self.pse
    }

    /// Returns `true` if 36-bit Page Size Extensions (PSE-36) are supported.
    pub const fn pse36_supported(&self) -> bool {
        self.pse36
    }

    /// Returns `true` if Model Specific Registers (MSRs) are supported.
    pub const fn msr_supported(&self) -> bool {
        self.msr
    }

    /// Returns `true` if Physical Address Extension (PAE) is supported.
    pub const fn pae_supported(&self) -> bool {
        self.pae
    }

    /// Returns `true` if the local Advanced Programmable Interrupt Controller (APIC) is present.
    pub const fn apic_supported(&self) -> bool {
        self.apic
    }

    /// Returns `true` if Memory Type Range Registers (MTRRs) are supported.
    pub const fn mtrr_supported(&self) -> bool {
        self.mtrr
    }

    /// Returns `true` if the Page Attribute Table (PAT) is supported.
    pub const fn pat_supported(&self) -> bool {
        self.pat
    }

    /// Returns `true` if 5-level paging (LA57) is supported.
    pub const fn la57_supported(&self) -> bool {
        self.la57
    }

    /// Returns the maximum extended CPUID leaf supported by the processor.
    pub const fn max_extended_cpuid(&self) -> u32 {
        self.max_extended_cpuid
    }

    /// Returns `true` if the No-Execute (NXE) page protection bit is supported.
    pub const fn nxe_supported(&self) -> bool {
        self.nxe
    }

    // Returns `true` if 1 GiB huge pages are supported.
    pub const fn gib_pages_supported(&self) -> bool {
        self.gib_pages
    }

    /// Returns `true` if the processor supports Long Mode (64-bit).
    pub const fn long_mode_supported(&self) -> bool {
        self.long_mode
    }

    /// Returns the maximum physical address size in bits.
    pub const fn physical_address_size(&self) -> u8 {
        self.phys_addr_size
    }

    /// Returns the maximum guest physical address size in bits (for virtualization).
    pub const fn guest_physical_address_size(&self) -> u8 {
        self.guest_phys_addr_size
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

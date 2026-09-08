//! Raw definitions of device tree structures.
#![expect(missing_docs, reason = "no need to document raw definitions")]

pub const FDT_MAGIC: u32 = 0xd00dfeed;
pub const FDT_VERSION: u32 = 17;

pub const FDT_BEGIN_NODE: u32 = 0x00000001;
pub const FDT_END_NODE: u32 = 0x00000002;
pub const FDT_PROP: u32 = 0x00000003;
pub const FDT_NOP: u32 = 0x00000004;
pub const FDT_END: u32 = 0x00000009;

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FdtHeader {
    pub magic: u32,
    pub totalsize: u32,

    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,

    pub version: u32,

    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,

    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FdtReserveEntry {
    pub address: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FdtProperty {
    pub len: u32,
    pub nameoff: u32,
}

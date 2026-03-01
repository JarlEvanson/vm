//! Code interacting with the REVM protocol.

use core::mem;

use conversion::{u64_to_usize_strict, usize_to_u64};
use stub_api::{GenericTable, HeaderV0, Status, Header, GenericTableV0};

#[cfg(target_arch = "aarch64")]
use stub_api::aarch64::{Aarch64Table as ArchTable, Aarch64TableV0 as ArchTableV0};
#[cfg(target_arch = "x86")]
use stub_api::x86_32::{X86_32Table as ArchTable, X86_32TableV0 as ArchTableV0};
#[cfg(target_arch = "x86_64")]
use stub_api::x86_64::{X86_64Table as ArchTable, X86_64TableV0 as ArchTableV0};

/// Entry point to `revm` utilizing the REVM protocol.
#[unsafe(no_mangle)]
extern "C" fn revm_entry(header_ptr: *mut HeaderV0) -> Status {
    let (generic_table, _arch_table) = match validate_protocol_table(header_ptr) {
        Ok((generic, arch)) => (generic, arch),
        Err(status) => return status,
    };

    Status::SUCCESS
}

/// Validates that the REVM protocol table is properly formatted and the versions of the table and
/// subtables are supported.
fn validate_protocol_table(
    header_ptr: *mut HeaderV0,
) -> Result<(&'static GenericTable, &'static ArchTable), Status> {
    if header_ptr.is_null() {
        // Returns [`Status::INVALID_USAGE`] to indicate that it was called with an invalid
        // parameter.
        return Err(Status::INVALID_USAGE);
    }

    // SAFETY:
    //
    // The REVM boot protocol states that  we are always passed a [`Header`] and it will always be
    // backwards compatible (thus, since this is the first [`Header`], we know that this is safe).
    let header_v0 = unsafe { header_ptr.cast::<HeaderV0>().read() };
    if header_v0.version != Header::VERSION
        || header_v0.last_major_version != Header::LAST_MAJOR_VERSION
    {
        // We currently only support the first version of the REVM boot protocol.
        return Err(Status::NOT_SUPPORTED);
    }

    // Check that the tables can fit within the provided size of the protocol table.
    if (header_v0.generic_table_offset)
        .checked_add(usize_to_u64(mem::size_of::<GenericTable>()))
        .is_none_or(|max_offset| max_offset > header_v0.length)
        || (header_v0.arch_table_offset)
            .checked_add(usize_to_u64(mem::size_of::<ArchTable>()))
            .is_none_or(|max_offset| max_offset > header_v0.length)
    {
        return Err(Status::INVALID_USAGE);
    }

    let generic_table = header_ptr
        .wrapping_byte_add(u64_to_usize_strict(header_v0.generic_table_offset))
        .cast::<GenericTableV0>();
    // SAFETY:
    //
    // The REVM boot protocol states that the generic table is always in existence and that it is
    // located `header.generic_table_offset` bytes away from the start of [`Header`].
    let generic_table = unsafe { &mut *generic_table };
    if generic_table.version != GenericTableV0::VERSION {
        // We currently only support the first version of the REVM boot protocol.
        return Err(Status::NOT_SUPPORTED);
    }

    let arch_table = header_ptr
        .wrapping_byte_add(u64_to_usize_strict(header_v0.arch_table_offset))
        .cast::<ArchTableV0>();
    // SAFETY:
    //
    // The REVM boot protocol states that the generic table is always in existence and that it is
    // located `header.arch_table_offset` bytes away from the start of [`Header`].
    let arch_table = unsafe { &mut *arch_table };
    if arch_table.version != ArchTable::VERSION {
        // We currently only support the first version of the REVM boot protocol.
        return Err(Status::NOT_SUPPORTED);
    }

    Ok((generic_table, arch_table))
}

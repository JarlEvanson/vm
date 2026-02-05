//! Code interacting with the REVM protocol.

use core::{
    mem, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use stub_api::{GenericTable, GenericTableV0, Header, HeaderV0, Status};

#[cfg(target_arch = "x86_64")]
pub use stub_api::x86_64::{X86_64Table as ArchTable, X86_64TableV0 as ArchTableV0};

use crate::{
    arch::{paging, virtualization},
    util::{image_end, image_start, u64_to_usize, usize_to_u64},
};

/// Pointer to the REVM protocol table.
static PROTOCOL_TABLE: AtomicPtr<HeaderV0> = AtomicPtr::new(ptr::null_mut());

/// Entry point to `revm` utilizing the REVM protocol.
#[unsafe(no_mangle)]
extern "C" fn revm_entry(header_ptr: *mut HeaderV0) -> Status {
    let (generic_table, arch_table) = match validate_protocol_table(header_ptr) {
        Ok((generic, arch)) => (generic, arch),
        Err(status) => return status,
    };

    PROTOCOL_TABLE.store(header_ptr, Ordering::Release);
    assert!(image_end() - image_start() <= 2 * 1024 * 1024 * 1024);

    unsafe {
        paging::initialize_mapper(
            generic_table.image_physical_address,
            generic_table.image_virtual_address,
        )
    }

    crate::debug!(
        "revm image physical address: {:#x}",
        generic_table.image_physical_address
    );
    crate::debug!(
        "revm image virtual address: {:#x}",
        generic_table.image_virtual_address
    );

    crate::trace!("{generic_table:#x?}");
    crate::trace!("{arch_table:#x?}");

    let virtualization_supported = virtualization::supported();
    crate::debug!("virtualization supported: {virtualization_supported}");
    if virtualization_supported {
        use x86_common::{control_register::*, msr::read_msr};

        let cr0: u64;
        unsafe { core::arch::asm!("mov {}, cr0", lateout(reg) cr0) };
        let cr0 = Cr0::from_bits(cr0);
        crate::debug!("{cr0:}");

        let cr4: u64;
        unsafe { core::arch::asm!("mov {}, cr4", lateout(reg) cr4) };
        let cr4 = Cr4::from_bits(cr4);
        crate::debug!("{cr4:}");

        let cr0_fixed_0 = unsafe { read_msr(0x486) };
        let cr0_fixed_1 = unsafe { read_msr(0x487) };
        let cr4_fixed_0 = unsafe { read_msr(0x488) };
        let cr4_fixed_1 = unsafe { read_msr(0x489) };

        crate::debug!("Forced-0 {}", Cr0::from_bits(!cr0_fixed_1));
        crate::debug!("Forced-1 {}", Cr0::from_bits(cr0_fixed_0));
        crate::debug!("Flexible {}", Cr0::from_bits(!cr0_fixed_0 & cr0_fixed_1));

        crate::debug!("Forced-0 {}", Cr4::from_bits(!cr4_fixed_1));
        crate::debug!("Forced-1 {}", Cr4::from_bits(cr4_fixed_0));
        crate::debug!("Flexible {}", Cr4::from_bits(!cr4_fixed_0 & cr4_fixed_1));
    }

    Status::SUCCESS
}

/// Returns the REVM protocol table.
pub fn protocol_table() -> Option<&'static Header> {
    // SAFETY:
    //
    // This reference is valid until `takeover()` is called, which has the safety invariant that
    // all REVM protocol table references are not active.
    unsafe { PROTOCOL_TABLE.load(Ordering::Acquire).as_ref() }
}

/// Returns the REVM protocol generic table.
pub fn generic_table() -> Option<&'static GenericTable> {
    let header = protocol_table()?;

    // SAFETY:
    //
    // This reference is valid until `takeover()` is called, which has the safety invariant that
    // all REVM protocol table references are not active.
    unsafe {
        (&raw const *header)
            .wrapping_byte_add(u64_to_usize(header.generic_table_offset))
            .cast::<GenericTable>()
            .as_ref()
    }
}

/// Returns the REVM protocol architecture table.
pub fn arch_table() -> Option<&'static ArchTable> {
    let header = protocol_table()?;

    // SAFETY:
    //
    // This reference is valid until `takeover()` is called, which has the safety invariant that
    // all REVM protocol table references are not active.
    unsafe {
        (&raw const *header)
            .wrapping_byte_add(u64_to_usize(header.arch_table_offset))
            .cast::<ArchTable>()
            .as_ref()
    }
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
    if (u64_to_usize(header_v0.generic_table_offset))
        .checked_add(mem::size_of::<GenericTable>())
        .is_none_or(|max_offset| max_offset > u64_to_usize(header_v0.length))
        || (u64_to_usize(header_v0.arch_table_offset))
            .checked_add(mem::size_of::<ArchTable>())
            .is_none_or(|max_offset| max_offset > u64_to_usize(header_v0.length))
    {
        return Err(Status::INVALID_USAGE);
    }

    let generic_table = header_ptr
        .wrapping_byte_add(u64_to_usize(header_v0.generic_table_offset))
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
        .wrapping_byte_add(u64_to_usize(header_v0.arch_table_offset))
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

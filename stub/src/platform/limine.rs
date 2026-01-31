//! Support for booting from the Limine boot protocol.

use core::{ptr, slice};

use limine::{
    BASE_REVISION, BASE_REVISION_MAGIC_0, BASE_REVISION_MAGIC_1, BaseRevisionTag,
    device_tree::{DEVICE_TREE_REQUEST_MAGIC, DeviceTreeRequest},
    efi_sys_table::{EFI_SYSTEM_TABLE_REQUEST_MAGIC, EfiSystemTableRequest},
    executable_addr::{EXECUTABLE_ADDRESS_REQUEST_MAGIC, ExecutableAddressRequest},
    hhdm::{HHDM_REQUEST_MAGIC, HhdmRequest},
    memory_map::{MEMORY_MAP_REQUEST_MAGIC, MemoryMapEntry, MemoryMapRequest},
    rsdp::{RSDP_REQUEST_MAGIC, RsdpRequest},
    smbios::{SMBIOS_REQUEST_MAGIC, SmbiosRequest},
};
use sync::ControlledModificationCell;

use crate::util::u64_to_usize;

/// Indicates the start of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.start")]
static REQUESTS_START_MARKER: [u64; 4] = limine::REQUESTS_START_MARKER;

/// Tag used to communicate the information regarding the base revision of the Limine protocol.
#[used]
#[unsafe(link_section = ".limine.base_tag")]
static BASE_REVISION_TAG: ControlledModificationCell<BaseRevisionTag> =
    ControlledModificationCell::new(BaseRevisionTag {
        magic: BASE_REVISION_MAGIC_0,
        loaded_revision: BASE_REVISION_MAGIC_1,
        supported_revision: BASE_REVISION,
    });

/// Request for the memory map to be provided to the application.
#[used]
#[unsafe(link_section = ".limine.requests")]
static MEMORY_MAP_REQUEST: ControlledModificationCell<MemoryMapRequest> =
    ControlledModificationCell::new(MemoryMapRequest {
        id: MEMORY_MAP_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the higher half direct map offset.
#[used]
#[unsafe(link_section = ".limine.requests")]
static HHDM_REQUEST: ControlledModificationCell<HhdmRequest> =
    ControlledModificationCell::new(HhdmRequest {
        id: HHDM_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address (both virtual and physical) of the executable.
#[used]
#[unsafe(link_section = ".limine.requests")]
static EXECUTABLE_ADDRESS_REQUEST: ControlledModificationCell<ExecutableAddressRequest> =
    ControlledModificationCell::new(ExecutableAddressRequest {
        id: EXECUTABLE_ADDRESS_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the UEFI system table.
#[used]
#[unsafe(link_section = ".limine.requests")]
static UEFI_SYSTEM_TABLE_REQUEST: ControlledModificationCell<EfiSystemTableRequest> =
    ControlledModificationCell::new(EfiSystemTableRequest {
        id: EFI_SYSTEM_TABLE_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the RSDP table.
#[used]
#[unsafe(link_section = ".limine.requests")]
static RSDP_REQUEST: ControlledModificationCell<RsdpRequest> =
    ControlledModificationCell::new(RsdpRequest {
        id: RSDP_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the address of the device tree.
#[used]
#[unsafe(link_section = ".limine.requests")]
static DEVICE_TREE_REQUEST: ControlledModificationCell<DeviceTreeRequest> =
    ControlledModificationCell::new(DeviceTreeRequest {
        id: DEVICE_TREE_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Request for the addresses of the SMBIOS tables.
#[used]
#[unsafe(link_section = ".limine.requests")]
static SMBIOS_REQUEST: ControlledModificationCell<SmbiosRequest> =
    ControlledModificationCell::new(SmbiosRequest {
        id: SMBIOS_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: [u64; 2] = limine::REQUESTS_END_MARKER;

/// Entry point for Rust when booted using the Limine boot protocol.
pub extern "C" fn limine_main() -> ! {
    let (memory_map_entries, hhdm_offset, executable_physical_base, executable_virtual_base) =
        validate_required_tables();

    crate::debug!("{:#x}", crate::util::image_start());
    match crate::stub_main() {
        Ok(()) => {}
        Err(error) => crate::error!("error loading from Limine: {error}"),
    };

    loop {
        core::hint::spin_loop()
    }
}

/// Validates that the required Limine requests have been fulfilled and returns the contents of
/// those responses.
fn validate_required_tables() -> (&'static [&'static MemoryMapEntry], u64, u64, u64) {
    if BASE_REVISION_TAG.get().supported_revision == BASE_REVISION {
        // If the base revision this executable was loaded using is greater than or equal to 3,
        // then [`BaseRevisionTag::loaded_revision`] contains the base revision used to load the
        // executable. Otherwise, the base revision must be either 0, 1, or 2.
        if BASE_REVISION_TAG.get().loaded_revision != BASE_REVISION_MAGIC_1 {
            panic!(
                "Loaded using unsupported base revision {}",
                BASE_REVISION_TAG.get().loaded_revision
            )
        } else {
            panic!("Loaded using unsupported base revision (possible revisions are 0, 1, and 2)")
        }
    }

    let memory_map_response_ptr = MEMORY_MAP_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`MEMORY_MAP_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `memory_map_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer.
    let Some(memory_map_response) = (unsafe { memory_map_response_ptr.as_ref() }) else {
        panic!("Limine memory map was not provided");
    };

    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`MEMORY_MAP_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `memory_map_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer. Moreover, the
    // request must have been fulfilled according to the specification and as such, this slice is
    // valid.
    let memory_map_entries = unsafe {
        slice::from_raw_parts(
            memory_map_response.entries.cast::<&MemoryMapEntry>(),
            u64_to_usize(memory_map_response.entry_count),
        )
    };

    let hhdm_response_ptr = HHDM_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`HHDM_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `hhdm_response_ptr` was initialized to
    // `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid pointer.
    let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
        panic!("Limine higher half direct map was not provided");
    };

    let executable_address_response_ptr = EXECUTABLE_ADDRESS_REQUEST.get().response;
    // SAFETY:
    //
    // The Limine bootloader specification states that if the [`EXECUTABLE_ADDRESS_REQUEST`] pointer
    // changes, the request has been fulfilled. Since `executable_address_response_ptr` was
    // initialized to `ptr::null_mut()`, if it is not `ptr::null_mut()`, it must be a valid
    // pointer.
    let Some(executable_address_response) = (unsafe { executable_address_response_ptr.as_ref() })
    else {
        panic!("Limine executable address was not provided");
    };

    (
        memory_map_entries,
        hhdm_response.offset,
        executable_address_response.physical_base,
        executable_address_response.virtual_base,
    )
}

//! Implementation of cross-address space switching for `x86_32` and `x86_64`.

use core::{
    error, fmt,
    mem::{self, offset_of},
};

use elf::header::Machine;
use stub_api::{
    GenericTable, Header, Status,
    raw::{GenericTable32, GenericTable64},
    x86_64::X86_64Table,
};
use sync::Spinlock;
use x86_common::{
    control::{Cr0, Cr4},
    paging::{PagingMode, current_paging_mode},
};

use crate::{
    arch::{
        ArchAddressSpace,
        generic::{
            address_space::{AddressSpace, MapError, NotFound, ProtectionFlags},
            switch::{self, ENTRY_FUNC_ID, MAX_GENERIC_ID},
        },
    },
    platform::{
        AllocationPolicy, FrameAllocation, OutOfMemory, PhysicalAddress, allocate_frames_aligned,
        device_tree, frame_size, map_identity, rsdp, smbios_32, smbios_64, uefi_system_table,
        write_bytes_at, xsdp,
    },
    util::usize_to_u64,
};

mod x86_64;

/// The size, in bytes, of the stack provided to the application.
const STACK_SIZE: u64 = 64 * 1024;

/// [`AllocationPolicy`] for physical memory that may be used in protected mode.
#[expect(clippy::as_conversions)]
const PROTECTED_MODE_POLICY: AllocationPolicy = AllocationPolicy::Below(u32::MAX as u64 + 1);

/// The data required to handle the cross-address space calls.
static SWITCH_DATA: Spinlock<Option<(ArchAddressSpace, u64)>> = Spinlock::new(None);

/// Prepares and switches to the executable.
#[expect(
    clippy::missing_errors_doc,
    reason = "SwitchError provides documentation"
)]
pub fn switch(
    mut address_space: ArchAddressSpace,
    machine: Machine,
    entry_point: u64,
    image_physical_address: PhysicalAddress,
    image_virtual_address: u64,
) -> Result<(), SwitchError> {
    let stub_long_mode = match current_paging_mode() {
        PagingMode::Disabled | PagingMode::Bits32 | PagingMode::Pae => false,
        PagingMode::Level4 | PagingMode::Level5 => true,
    };

    let executable_pae_bit = match address_space {
        ArchAddressSpace::Bits32(_) => false,
        ArchAddressSpace::Pae(_) => true,
        ArchAddressSpace::LongMode(_) => true,
    };

    let executable_long_mode = match machine {
        Machine::INTEL_386 => false,
        Machine::X86_64 => true,
        _ => unreachable!(),
    };

    // Calculate mode settings.
    let (code_segment, data_segment) = if executable_long_mode {
        (24, 32)
    } else {
        (8, 16)
    };
    let cr0 = Cr0::from_bits(0)
        .set_paging(true)
        .set_write_protection(true)
        .set_pe(true)
        .to_bits();
    let cr3 = address_space.cr3();
    let cr4 = Cr4::from_bits(0)
        .set_pse(true)
        .set_pae(executable_pae_bit)
        .to_bits();

    let mut storage = Storage {
        call: CallStorage::default(),
        loader: ModeStorage::default(),
        executable: ModeStorage {
            cs: code_segment,
            ds: data_segment,
            es: data_segment,
            fs: data_segment,
            gs: data_segment,
            ss: data_segment,

            cr0,
            cr3,
            cr4,
            ..Default::default()
        },
        executable_gdt: [
            0x0000_0000_0000_0000, // Null Segment
            0x00CF_9B00_0000_FFFF, // Kernel 32-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 32-bit data segment
            0x00AF_9B00_0000_FFFF, // Kernel 64-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 64-bit data segment
        ],
        executable_entry_point: entry_point,
    };

    let stack_frame_allocation =
        allocate_stack(&mut address_space, &mut storage).map_err(SwitchError::Stack)?;

    let storage_frame_allocation =
        allocate_storage(&mut address_space, &mut storage).map_err(SwitchError::Storage)?;

    let stub_mode_storage_ptr = storage_frame_allocation
        .range()
        .start()
        .start_address()
        .value()
        .strict_add(usize_to_u64(offset_of!(Storage, loader)));

    let executable_mode_storage_ptr = storage_frame_allocation
        .range()
        .start()
        .start_address()
        .value()
        .strict_add(usize_to_u64(offset_of!(Storage, executable)));

    let stub_code_layout = if stub_long_mode {
        x86_64::allocate_code(
            &mut address_space,
            storage_frame_allocation
                .range()
                .start()
                .start_address()
                .value(),
            stub_mode_storage_ptr,
            executable_mode_storage_ptr,
        )
        .map_err(SwitchError::StubCode)?
    } else {
        todo!()
    };

    let executable_code_layout = if executable_long_mode {
        x86_64::allocate_code(
            &mut address_space,
            storage_frame_allocation
                .range()
                .start()
                .start_address()
                .value(),
            executable_mode_storage_ptr,
            stub_mode_storage_ptr,
        )
        .map_err(SwitchError::ExecutableCode)?
    } else {
        todo!()
    };

    let (protocol_table_frame_allocation, protocol_table_address) = allocate_protocol_table(
        &mut address_space,
        &executable_code_layout,
        image_physical_address,
        image_virtual_address,
    )
    .map_err(SwitchError::ProtocolTable)?;

    #[expect(clippy::as_conversions)]
    {
        storage.executable.handle_call_internal = executable_code_layout.handle_call_internal;
        storage.executable.handle_call_external = executable_code_layout.handle_call_external;

        storage.call.arg_0 = protocol_table_address;
        storage.call.arg_count = 1;
        storage.call.func_id = ENTRY_FUNC_ID;

        storage.loader.handle_call_internal = stub_code_layout.handle_call_internal;
        storage.loader.handle_call_external = handle_call as *const () as u64;
    }

    crate::trace!("{storage:#x?}");

    #[expect(clippy::as_conversions)]
    let storage_ptr = storage_frame_allocation
        .range()
        .start()
        .start_address()
        .value() as *mut Storage;
    // SAFETY:
    //
    // TODO:
    unsafe { storage_ptr.write(storage) };

    crate::debug!("Stack Top: {:#x}", storage.executable.rsp);
    crate::debug!("Storage: {:x?}", storage_frame_allocation);
    crate::debug!("Stub Code: {:x?}", stub_code_layout.frame_allocation);
    crate::debug!(
        "Executable Code: {:x?}",
        executable_code_layout.frame_allocation
    );
    crate::debug!("Protocol Table: {:#x}", protocol_table_address);

    crate::debug!("Switch Entry Point: {:#x}", executable_code_layout.entry);

    *SWITCH_DATA.lock() = Some((
        address_space,
        storage_frame_allocation
            .range()
            .start()
            .start_address()
            .value(),
    ));

    let result: u64;

    // SAFETY:
    //
    // The executable's address space and switching code has been correctly prepared.
    unsafe {
        core::arch::asm!(
            "cli",

            "call r10",
            
            "sti", 
            
            lateout("rax") result,
            
            in("r10") stub_code_layout.entry,
            clobber_abi("sysv64"))
    }

    crate::info!("Executable Result: {:?}", Status(result));

    // SAFETY:
    //
    // TODO:
    let storage = unsafe { storage_ptr.read() };

    crate::trace!("{storage:#x?}");

    *SWITCH_DATA.lock() = None;
    drop(stack_frame_allocation);
    drop(storage_frame_allocation);
    drop(stub_code_layout);
    drop(executable_code_layout);
    drop(protocol_table_frame_allocation);

    Ok(())
}

/// Allocates and maps a stack for the loaded executable.
fn allocate_stack(
    address_space: &mut ArchAddressSpace,
    storage: &mut Storage,
) -> Result<FrameAllocation, ComponentError> {
    let stack_pages = STACK_SIZE.div_ceil(address_space.page_size());
    let frame_allocation = allocate_frames_aligned(
        STACK_SIZE.div_ceil(frame_size()),
        address_space.page_size(),
        PROTECTED_MODE_POLICY,
    )?;

    // Map the stack into the application's address space.
    let stack_bottom_address = address_space.find_region(stack_pages)?;
    address_space.map(
        stack_bottom_address,
        frame_allocation.range().start().start_address().value(),
        stack_pages,
        ProtectionFlags::READ | ProtectionFlags::WRITE,
    )?;

    storage.executable.rsp = stack_bottom_address + STACK_SIZE;
    Ok(frame_allocation)
}

/// Allocates and maps the [`Storage`] for the switching mechanism.
fn allocate_storage(
    address_space: &mut ArchAddressSpace,
    storage: &mut Storage,
) -> Result<FrameAllocation, ComponentError> {
    let storage_size = mem::size_of::<Storage>();
    let storage_size_u64 = usize_to_u64(storage_size);

    let frame_allocation = allocate_frames_aligned(
        storage_size_u64.div_ceil(frame_size()),
        address_space.page_size(),
        PROTECTED_MODE_POLICY,
    )?;

    let _ = map_identity(
        frame_allocation.range().start().start_address(),
        storage_size_u64,
    );
    address_space.map(
        frame_allocation.range().start().start_address().value(),
        frame_allocation.range().start().start_address().value(),
        storage_size_u64.div_ceil(address_space.page_size()),
        ProtectionFlags::READ | ProtectionFlags::WRITE,
    )?;

    storage.executable.gdtr.size = 5 * 8;
    storage.executable.gdtr.pointer = frame_allocation
        .range()
        .start()
        .start_address()
        .value()
        .strict_add(usize_to_u64(offset_of!(Storage, executable_gdt)));
    Ok(frame_allocation)
}

/// Allocates and maps the [`Storage`] for the protocol table.
fn allocate_protocol_table(
    address_space: &mut ArchAddressSpace,
    layout: &CodeLayout,
    image_physical_address: PhysicalAddress,
    image_virtual_address: u64,
) -> Result<(FrameAllocation, u64), ComponentError> {
    let bits_32 = address_space.max_virtual_address() <= u64::from(u32::MAX) + 1;

    let total_size = if bits_32 {
        mem::size_of::<RevmProtocolTable32>()
    } else {
        mem::size_of::<RevmProtocolTable64>()
    };
    let total_size_u64 = usize_to_u64(total_size);

    let protocol_table_pages = total_size_u64.div_ceil(address_space.page_size());
    let frame_allocation = allocate_frames_aligned(
        total_size_u64.div_ceil(frame_size()),
        address_space.page_size(),
        AllocationPolicy::Below(address_space.max_physical_address()),
    )?;

    // Map the stack into the application's address space.
    let protocol_table_address = address_space.find_region(protocol_table_pages)?;
    address_space.map(
        protocol_table_address,
        frame_allocation.range().start().start_address().value(),
        protocol_table_pages,
        ProtectionFlags::READ | ProtectionFlags::WRITE,
    )?;

    if bits_32 {
        // 32-bit address space.

        let protocol_table = RevmProtocolTable32 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable32>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable32,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(RevmProtocolTable32, x86_64_table)),
            },
            generic_table: GenericTable32 {
                version: GenericTable::VERSION,
                page_frame_size: frame_size().max(address_space.page_size()),
                image_physical_address: image_physical_address.value(),
                image_virtual_address,
                write: todo!(),
                allocate_frames: todo!(),
                deallocate_frames: todo!(),
                get_memory_map: todo!(),
                map: todo!(),
                unmap: todo!(),
                takeover: todo!(),
            },
            x86_64_table: X86_64Table {
                version: X86_64Table::VERSION,

                uefi_system_table: uefi_system_table().map(|addr| addr.value()).unwrap_or(0),
                rsdp: rsdp().map(|addr| addr.value()).unwrap_or(0),
                xsdp: xsdp().map(|addr| addr.value()).unwrap_or(0),
                device_tree: device_tree().map(|addr| addr.value()).unwrap_or(0),
                smbios_32: smbios_32().map(|addr| addr.value()).unwrap_or(0),
                smbios_64: smbios_64().map(|addr| addr.value()).unwrap_or(0),
            },
        };

        // SAFETY:
        //
        // TODO:
        let protocol_table_slice = unsafe {
            core::slice::from_raw_parts(
                (&raw const protocol_table).cast::<u8>(),
                mem::size_of::<RevmProtocolTable32>(),
            )
        };
        write_bytes_at(
            frame_allocation.range().start().start_address(),
            protocol_table_slice,
        );
    } else {
        // 64-bit address space.

        let protocol_table = RevmProtocolTable64 {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: usize_to_u64(mem::size_of::<RevmProtocolTable64>()),
                generic_table_offset: usize_to_u64(mem::offset_of!(
                    RevmProtocolTable64,
                    generic_table
                )),
                arch_table_offset: usize_to_u64(mem::offset_of!(RevmProtocolTable64, x86_64_table)),
            },
            generic_table: GenericTable64 {
                version: GenericTable::VERSION,
                page_frame_size: frame_size().max(address_space.page_size()),
                image_physical_address: image_physical_address.value(),
                image_virtual_address,
                write: layout.write,
                allocate_frames: layout.allocate_frames,
                deallocate_frames: layout.deallocate_frames,
                get_memory_map: layout.get_memory_map,
                map: layout.map,
                unmap: layout.unmap,
                takeover: layout.takeover,
            },
            x86_64_table: X86_64Table {
                version: X86_64Table::VERSION,

                uefi_system_table: uefi_system_table().map(|addr| addr.value()).unwrap_or(0),
                rsdp: rsdp().map(|addr| addr.value()).unwrap_or(0),
                xsdp: xsdp().map(|addr| addr.value()).unwrap_or(0),
                device_tree: device_tree().map(|addr| addr.value()).unwrap_or(0),
                smbios_32: smbios_32().map(|addr| addr.value()).unwrap_or(0),
                smbios_64: smbios_64().map(|addr| addr.value()).unwrap_or(0),
            },
        };

        // SAFETY:
        //
        // TODO:
        let protocol_table_slice = unsafe {
            core::slice::from_raw_parts(
                (&raw const protocol_table).cast::<u8>(),
                mem::size_of::<RevmProtocolTable64>(),
            )
        };
        write_bytes_at(
            frame_allocation.range().start().start_address(),
            protocol_table_slice,
        );
    };

    Ok((frame_allocation, protocol_table_address))
}

/// Handles cross-address space calls.
pub extern "C" fn handle_call() -> Status {
    // SAFETY:
    //
    // An IDT has been installed and thus it is safe to enable interrupts.
    unsafe { core::arch::asm!("sti") }

    let mut lock = SWITCH_DATA.lock();
    let Some((address_space, storage_address)) = lock.as_mut() else {
        return Status::NOT_SUPPORTED;
    };

    // SAFETY:
    //
    // Exclusive access to the [`Storage`] has been obtained through the lock.
    #[expect(clippy::as_conversions)]
    let storage = unsafe { &mut *(((*storage_address) as usize) as *mut Storage) };

    let result = match storage.call.func_id {
        0..MAX_GENERIC_ID => switch::handle_call(
            address_space,
            storage.call.func_id,
            storage.call.arg_0,
            storage.call.arg_1,
            storage.call.arg_2,
            storage.call.arg_3,
            storage.call.arg_4,
        ),
        func_id => unreachable!("invalid func_id: {func_id}"),
    };

    // SAFETY:
    //
    // As bare-metal ring 0 application, it is always safe to disable interrupts.
    unsafe { core::arch::asm!("cli") }
    result
}

/// Various errors that can occur while preparing and switching to the executable's address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchError {
    /// An error occurred while preparing the executable's stack.
    Stack(ComponentError),
    /// An error occurred while preparing the switching mechanism's [`Storage`].
    Storage(ComponentError),
    /// An error occurred while preparing the stub's code.
    StubCode(ComponentError),
    /// An error occurred while preparing the executable's code.
    ExecutableCode(ComponentError),
    /// An error occurred while preparing the protocol table.
    ProtocolTable(ComponentError),
}

impl fmt::Display for SwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stack(error) => write!(f, "error preparing stack: {error}"),
            Self::Storage(error) => write!(f, "error preparing storage: {error}"),
            Self::StubCode(error) => write!(f, "error preparing stub code: {error}"),
            Self::ExecutableCode(error) => write!(f, "error preparing executable code: {error}"),
            Self::ProtocolTable(error) => write!(f, "error preparing protocol table: {error}"),
        }
    }
}

impl error::Error for SwitchError {}

/// Various errors that can occur while allocating and mapping a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentError {
    /// An error occurred while allocating physical memory for the component.
    Allocation(OutOfMemory),
    /// An error occurred while find a location for the allocated memory to be mapped into the
    /// executable's address space.
    FindRegion(NotFound),
    /// An error occurred while mapping the allocated memory into the executable's address space.
    Map(MapError),
}

impl From<OutOfMemory> for ComponentError {
    fn from(error: OutOfMemory) -> Self {
        Self::Allocation(error)
    }
}

impl From<NotFound> for ComponentError {
    fn from(error: NotFound) -> Self {
        Self::FindRegion(error)
    }
}

impl From<MapError> for ComponentError {
    fn from(error: MapError) -> Self {
        Self::Map(error)
    }
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allocation(error) => write!(f, "error allocating physical memory: {error}"),
            Self::FindRegion(error) => write!(f, "error locating free virtual memory: {error}"),
            Self::Map(error) => write!(
                f,
                "error mapping physical memory into virtual address space: {error}"
            ),
        }
    }
}

impl error::Error for ComponentError {}

#[derive(Debug, PartialEq, Eq)]
#[expect(clippy::missing_docs_in_private_items)]
struct CodeLayout {
    frame_allocation: FrameAllocation,

    handle_call_internal: u64,
    handle_call_external: u64,

    // Executable-provided functions.
    entry: u64,

    // Stub-provided functions.
    write: u64,
    allocate_frames: u64,
    deallocate_frames: u64,
    get_memory_map: u64,
    map: u64,
    unmap: u64,
    takeover: u64,
}

#[repr(C)]
#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
struct RevmProtocolTable32 {
    header: Header,
    generic_table: GenericTable32,
    x86_64_table: X86_64Table,
}

#[repr(C)]
#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy)]
struct RevmProtocolTable64 {
    header: Header,
    generic_table: GenericTable64,
    x86_64_table: X86_64Table,
}

#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Storage {
    call: CallStorage,

    loader: ModeStorage,
    executable: ModeStorage,

    executable_gdt: [u64; 5],
    executable_entry_point: u64,
}

#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CallStorage {
    func_id: u16,
    arg_count: u8,

    arg_0: u64,
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
    arg_5: u64,

    ret: u64,
}

#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModeStorage {
    handle_call_internal: u64,
    handle_call_external: u64,

    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rsp: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    cs: u16,
    ds: u16,
    es: u16,
    fs: u16,
    gs: u16,
    ss: u16,

    cr0: u64,
    cr3: u64,
    cr4: u64,

    gdtr: TablePointer,
    idtr: TablePointer,

    tmp_storage: [u64; 5],
}

#[repr(C, packed)]
#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TablePointer {
    size: u16,
    pointer: u64,
}

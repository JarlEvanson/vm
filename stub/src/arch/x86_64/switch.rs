//! Implementation of cross address space switching for `x86_64`.

use core::{
    arch::{asm, global_asm},
    mem::{self, ManuallyDrop},
    ptr,
};

use stub_api::{GenericTable, Header, Status, x86_64::X86_64Table};

use crate::{
    arch::{
        AddressSpaceImpl,
        generic::{
            self,
            address_space::{AddressSpace, ProtectionFlags},
            switch::{
                self, ALLOCATE_FRAMES_ID, DEALLOCATE_FRAMES_ID, GET_MEMORY_MAP_ID, MAP_ID,
                TAKEOVER_ID, UNMAP_ID, WRITE_ID,
            },
        },
    },
    debug,
    platform::{
        AllocationPolicy, allocate_frames_aligned, device_tree, frame_size, map_identity,
        page_size, rsdp, smbios_32, smbios_64, uefi_system_table, write_bytes_at, write_u32_at,
        write_u64_at, xsdp,
    },
    util::{u64_to_usize, usize_to_u64},
    warn,
};

macro_rules! calculate_offset {
    ($func:ident, $base:expr) => {{
        let offset = (ptr::addr_of!($func).addr() - ptr::addr_of!(CODE_START).addr()) as u64;
        unsafe { mem::transmute::<u64, _>($base + offset) }
    }};
}

/// Switches address spaces.
pub fn switch(
    application_space: &mut AddressSpaceImpl,
    entry_point: u64,
    image_physical_address: u64,
    image_virtual_address: u64,
) -> Result<(), ()> {
    // Create [`AllocationPolicy`] for identity mapped code and data.
    let max_physical_address = u64::from(u32::MAX) + 1;
    let under_4_gib = AllocationPolicy::Below(max_physical_address);

    // Calculate required alignment for simplest code.
    let max_alignment = frame_size()
        .max(page_size() as u64)
        .max(application_space.page_size());

    let code_frame_allocation = {
        // Validate that the code is compatible with the provided [`ContextImpl`].
        let identity_size =
            (ptr::addr_of!(IDENTIY_END).addr() - ptr::addr_of!(CODE_START).addr()) as u64;
        assert!(
            identity_size <= page_size() as u64,
            "identity-map code must fit into a single `page_size()`"
        );

        // Calculate total size of code region and allocate its physical memory.
        let total_size = (ptr::addr_of!(CODE_END).addr() - ptr::addr_of!(CODE_START).addr()) as u64;

        let frame_allocation = allocate_frames_aligned(
            total_size.div_ceil(frame_size()),
            max_alignment,
            under_4_gib,
        )
        .unwrap();

        // Map code region into the two address spaces.
        let _ = map_identity(frame_allocation.physical_address());
        application_space
            .map(
                frame_allocation.physical_address(),
                frame_allocation.physical_address(),
                total_size.div_ceil(application_space.page_size()),
                ProtectionFlags::READ | ProtectionFlags::WRITE | ProtectionFlags::EXECUTE,
            )
            .unwrap();

        // Fill the code region with the provided code.
        let code_bytes =
            unsafe { core::slice::from_raw_parts(ptr::addr_of!(CODE_START), total_size as usize) };
        write_bytes_at(frame_allocation.physical_address(), code_bytes);

        debug!(
            "Identity Mapped Code/Data: {:#x}",
            frame_allocation.physical_address()
        );
        frame_allocation
    };

    let (stack_frame_allocation, stack_pointer) = {
        // Calculate total size of stack and allocate its physical memory.
        let stack_size: u64 = 64 * 1024;
        let stack_pages = stack_size.div_ceil(application_space.page_size());
        let frame_allocation = allocate_frames_aligned(
            stack_size.div_ceil(frame_size()),
            max_alignment,
            AllocationPolicy::Any,
        )
        .unwrap();

        // Map the stack into the application's address space.
        let stack_bottom_address = application_space.find_region(stack_pages).unwrap();
        application_space
            .map(
                stack_bottom_address,
                frame_allocation.physical_address(),
                stack_pages,
                ProtectionFlags::READ | ProtectionFlags::WRITE,
            )
            .unwrap();

        debug!(
            "Application Stack Physical Address: {:#x}",
            frame_allocation.physical_address()
        );
        debug!(
            "Application Stack Virtual Address: {:#x}",
            stack_bottom_address
        );
        (frame_allocation, stack_bottom_address + stack_size)
    };

    let (protocol_table_frame_allocation, protocol_table) = {
        let total_size = mem::size_of::<RevmProtocolTable>() as u64;
        let protocol_table_pages = total_size.div_ceil(application_space.page_size());
        let frame_allocation = allocate_frames_aligned(
            total_size.div_ceil(frame_size()),
            max_alignment,
            AllocationPolicy::Any,
        )
        .unwrap();

        let protocol_table_virtual_address =
            application_space.find_region(protocol_table_pages).unwrap();
        application_space
            .map(
                protocol_table_virtual_address,
                frame_allocation.physical_address(),
                protocol_table_pages,
                ProtectionFlags::READ | ProtectionFlags::WRITE,
            )
            .unwrap();

        let protocol_table = RevmProtocolTable {
            header: Header {
                version: Header::VERSION,
                last_major_version: Header::LAST_MAJOR_VERSION,
                length: mem::size_of::<RevmProtocolTable>() as u64,
                generic_table_offset: mem::offset_of!(RevmProtocolTable, generic_table) as u64,
                arch_table_offset: mem::offset_of!(RevmProtocolTable, x86_64_table) as u64,
            },
            generic_table: GenericTable {
                version: GenericTable::VERSION,
                page_frame_size: frame_size().max(application_space.page_size()),
                image_physical_address,
                image_virtual_address,
                write: calculate_offset!(WRITE, code_frame_allocation.physical_address()),
                allocate_frames: calculate_offset!(
                    ALLOCATE_FRAMES,
                    code_frame_allocation.physical_address()
                ),
                deallocate_frames: calculate_offset!(
                    DEALLOCATE_FRAMES,
                    code_frame_allocation.physical_address()
                ),
                get_memory_map: calculate_offset!(
                    GET_MEMORY_MAP,
                    code_frame_allocation.physical_address()
                ),
                map: calculate_offset!(MAP, code_frame_allocation.physical_address()),
                unmap: calculate_offset!(UNMAP, code_frame_allocation.physical_address()),
                takeover: calculate_offset!(TAKEOVER, code_frame_allocation.physical_address()),
            },
            x86_64_table: X86_64Table {
                version: X86_64Table::VERSION,

                uefi_system_table: uefi_system_table().unwrap_or(0),
                rsdp: rsdp().unwrap_or(0),
                xsdp: xsdp().unwrap_or(0),
                device_tree: device_tree().unwrap_or(0),
                smbios_32: smbios_32().unwrap_or(0),
                smbios_64: smbios_64().unwrap_or(0),
            },
        };

        let protocol_table_slice = unsafe {
            core::slice::from_raw_parts(
                (&protocol_table) as *const RevmProtocolTable as *const u8,
                mem::size_of::<RevmProtocolTable>(),
            )
        };
        write_bytes_at(frame_allocation.physical_address(), protocol_table_slice);

        debug!(
            "Stub API Table Physical Address: {:#x}",
            frame_allocation.physical_address()
        );
        debug!(
            "Stub API Table Virtual Address: {:#x}",
            protocol_table_virtual_address
        );
        (frame_allocation, protocol_table_virtual_address)
    };

    let call_function_offset =
        ptr::addr_of!(CALL_FUNCTION).addr() - ptr::addr_of!(CODE_START).addr();

    let (
        generic_handler_frame_allocation,
        generic_handler_base_virtual_address,
        generic_handler_size,
    ) = {
        let generic_handler_size = usize_to_u64(
            (&raw const GENERIC_HANDLER_END).addr() - (&raw const GENERIC_HANDLER_START).addr(),
        );
        let mov_instruction_offset = usize_to_u64(
            (&raw const GENERIC_HANDLER_MOV_INSTRUCTION).addr()
                - (&raw const GENERIC_HANDLER_START).addr(),
        );
        let code_data_start_address_offset = usize_to_u64(
            (&raw const GENERIC_HANDLER_CODE_DATA_START_ADDRESS).addr()
                - (&raw const GENERIC_HANDLER_START).addr(),
        );
        let call_function_address_offset = usize_to_u64(
            (&raw const GENERIC_HANDLER_CALL_FUNCTION).addr()
                - (&raw const GENERIC_HANDLER_START).addr(),
        );
        let handle_call_address_offset = usize_to_u64(
            (&raw const GENERIC_HANDLER_HANDLE_CALL).addr()
                - (&raw const GENERIC_HANDLER_START).addr(),
        );

        let total_size = generic_handler_size.strict_mul(256);

        let frame_allocation = allocate_frames_aligned(
            total_size.div_ceil(frame_size()),
            max_alignment,
            AllocationPolicy::Any,
        )
        .unwrap();

        let generic_handler_base_virtual_address = application_space
            .find_region(total_size.div_ceil(application_space.page_size()))
            .unwrap();

        application_space
            .map(
                generic_handler_base_virtual_address,
                frame_allocation.physical_address(),
                total_size.div_ceil(application_space.page_size()),
                ProtectionFlags::READ | ProtectionFlags::EXECUTE,
            )
            .unwrap();

        // Fill the code region with the provided code.
        let code_bytes = unsafe {
            core::slice::from_raw_parts(
                ptr::addr_of!(GENERIC_HANDLER_START),
                u64_to_usize(generic_handler_size),
            )
        };
        for index in 0..256 {
            let offset = u64::from(index).strict_mul(generic_handler_size);
            let physical_address = frame_allocation.physical_address().strict_add(offset);

            write_bytes_at(physical_address, code_bytes);
            write_u32_at(physical_address + mov_instruction_offset + 1, index);
            write_u64_at(
                physical_address + code_data_start_address_offset,
                frame_allocation.physical_address(),
            );
            write_u64_at(
                physical_address + call_function_address_offset,
                frame_allocation.physical_address() + usize_to_u64(call_function_offset),
            );
            write_u64_at(
                physical_address + handle_call_address_offset,
                usize_to_u64(handle_call as *const () as usize),
            );
        }

        debug!(
            "Generic Exception Handler Table Physical Address: {:#x}",
            frame_allocation.physical_address()
        );
        debug!(
            "Generic Exception Handler Table Virtual Address: {:#x}",
            generic_handler_base_virtual_address
        );
        (
            frame_allocation,
            generic_handler_base_virtual_address,
            generic_handler_size,
        )
    };

    let (idt_frame_allocation, idt_virtual_address, idt_size) = {
        const IDT_ENTRIES: usize = 256;
        let idt_size = (IDT_ENTRIES * core::mem::size_of::<IdtEntry>()) as u64;

        let frame_allocation = allocate_frames_aligned(
            idt_size.div_ceil(frame_size()),
            max_alignment,
            AllocationPolicy::Any,
        )
        .unwrap();

        let idt_virtual = application_space
            .find_region(idt_size.div_ceil(application_space.page_size()))
            .unwrap();

        application_space
            .map(
                idt_virtual,
                frame_allocation.physical_address(),
                idt_size.div_ceil(application_space.page_size()),
                ProtectionFlags::READ | ProtectionFlags::WRITE,
            )
            .unwrap();

        let mut entries = [IdtEntry::missing(); IDT_ENTRIES];

        for (index, e) in entries.iter_mut().enumerate() {
            *e = IdtEntry::interrupt(
                generic_handler_base_virtual_address + usize_to_u64(index) * generic_handler_size,
                8,
            );
        }

        entries[0] = IdtEntry::interrupt(
            calculate_offset!(
                DIVIDE_ERROR_HANDLER,
                code_frame_allocation.physical_address()
            ),
            8,
        );
        entries[1] = IdtEntry::interrupt(
            calculate_offset!(DEBUG_HANDLER, code_frame_allocation.physical_address()),
            8,
        );
        entries[6] = IdtEntry::interrupt(
            calculate_offset!(
                INVALID_OPCODE_HANDLER,
                code_frame_allocation.physical_address()
            ),
            8,
        );
        entries[14] = IdtEntry::interrupt(
            calculate_offset!(PAGE_FAULT_HANDLER, code_frame_allocation.physical_address()),
            8,
        );

        write_bytes_at(frame_allocation.physical_address(), unsafe {
            core::slice::from_raw_parts(
                entries.as_ptr() as *const u8,
                entries.len() * core::mem::size_of::<IdtEntry>(),
            )
        });

        debug!(
            "IDT Physical Address: {:#x}",
            frame_allocation.physical_address()
        );
        debug!("IDT Virtual Address: {:#x}", idt_virtual);
        (frame_allocation, idt_virtual, idt_size)
    };

    let data = IdentityMappedData {
        handle_call,
        application_space: unsafe { ManuallyDrop::new(ptr::from_ref(application_space).read()) },
        loader: CpuData::default(),
        application: CpuData {
            cr3: application_space.cr3(),
            sp: stack_pointer,
            cs: 8,
            ds: 16,
            es: 16,
            gdt_size: 5 * 8,
            gdt_address: code_frame_allocation.physical_address()
                + mem::offset_of!(IdentityMappedData, gdt) as u64,
            fs: 16,
            gs: 16,
            ss: 16,
            idt_size: idt_size as u16,
            idt_address: idt_virtual_address,
        },
        tmp_storage: TmpStorage::default(),
        gdt: [
            0x0000_0000_0000_0000, // Null Segment
            0x00AF_9B00_0000_FFFF, // Kernel 64-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 64-bit data segment
            0x00CF_9B00_0000_FFFF, // Kernel 32-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 32-bit data segment
        ],
    };

    // Write the [`IdentityMappedData`] structure into its required location.
    let identity_mapped_data = code_frame_allocation.physical_address() as *mut IdentityMappedData;
    unsafe { identity_mapped_data.write(data) }

    // Calculate the address at which the program will enter.
    let call_function_offset =
        ptr::addr_of!(CALL_FUNCTION).addr() - ptr::addr_of!(CODE_START).addr();
    let call_function = (code_frame_allocation.physical_address() as *mut u8)
        .wrapping_byte_add(call_function_offset);

    let result: u64;

    unsafe {
        asm!(
            "cli",

            "call r9",

            "sti",

            in("r9") call_function, // Address of the `call_function`.

            inlateout("rax") 0u64 => result, // There are no `func_id`s provided for the loader.
            in("r10") 1, // Only a single argument is passed.
            in("r11") 1, // Non-zero value indicates entrance from the loader.
            in("r12") entry_point, // Address of the function to call in the executable's address
                                     // space.

            in("rdi") protocol_table, // Address of the REVM protocol table.

            clobber_abi("sysv64")
        )
    }

    drop(idt_frame_allocation);
    drop(generic_handler_frame_allocation);
    drop(protocol_table_frame_allocation);
    drop(stack_frame_allocation);
    drop(code_frame_allocation);

    let result = Status(result);
    if result != Status::SUCCESS {
        warn!("application return code: {result:x?}")
    }

    Ok(())
}

/// Function ID representing the Divide Error exception.
const DIVIDE_ERROR_ID: u16 = switch::MAX_GENERIC_ID + 1;
/// Function ID representing the Debug exception.
const DEBUG_ID: u16 = DIVIDE_ERROR_ID + 1;
/// Function ID representing the Invalid Opcode exception.
const INVALID_OPCODE_ID: u16 = DEBUG_ID + 1;
/// Function ID representing the Page Fault exception.
const PAGE_FAULT_ID: u16 = INVALID_OPCODE_ID + 1;
/// Function ID representing a generic exception/interrupt.
const GENERIC_ID: u16 = PAGE_FAULT_ID + 1;

extern "C" fn handle_call(
    arg_1: u64,
    arg_2: u64,
    arg_3: u64,
    arg_4: u64,
    arg_5: u64,
    arg_6: u64,
) -> Status {
    unsafe { core::arch::asm!("sti") }

    let data = unsafe {
        (arg_6 as *mut IdentityMappedData)
            .as_mut()
            .unwrap_unchecked()
    };

    let result = match data.tmp_storage.func_id {
        0..=switch::MAX_GENERIC_ID => generic::switch::handle_call(
            &mut *data.application_space,
            data.tmp_storage.func_id,
            arg_1,
            arg_2,
            arg_3,
            arg_4,
            arg_5,
            arg_6,
        ),
        DIVIDE_ERROR_ID => {
            let exception_address = arg_1;
            let cs = arg_2;

            panic!("Divide error at {cs:#x}:{exception_address:#x}");
        }
        DEBUG_ID => {
            let exception_address = arg_1;
            let cs = arg_2;

            panic!("Debug exception at {cs:#x}:{exception_address:#x}");
        }
        INVALID_OPCODE_ID => {
            let exception_address = arg_1;
            let cs = arg_2;

            panic!("Invalid opcode at {cs:#x}:{exception_address:#x}");
        }
        PAGE_FAULT_ID => {
            let exception_address = arg_1;
            let cs = arg_2;
            let error_code = arg_3;

            let cr2: u64;
            unsafe { core::arch::asm!("mov {}, cr2", lateout(reg) cr2) }

            panic!(
                "Page fault exception at {cs:#x}:{exception_address:#x} \
                accessing {cr2:#x} with error code {error_code:#b}"
            );
        }
        GENERIC_ID => {
            let interrupt_number = arg_1;

            panic!("Interrupt {interrupt_number} occurred");
        }

        func_id => panic!("invalid func_id: {func_id}"),
    };

    unsafe { core::arch::asm!("cli") }
    result
}

struct IdentityMappedData {
    /// Handler for cross-address space calls.
    handle_call: extern "C" fn(
        arg_1: u64,
        arg_2: u64,
        arg_3: u64,
        arg_4: u64,
        arg_5: u64,
        arg_6: u64,
    ) -> Status,

    /// The implementation of [`AddressSpace`].
    application_space: ManuallyDrop<AddressSpaceImpl>,

    /// The loader address space's system register values.
    loader: CpuData,
    /// The application address space's system register values.
    application: CpuData,

    /// Temporary storage for the cross address space switching.
    tmp_storage: TmpStorage,

    gdt: [u64; 5],
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct TmpStorage {
    /// The identity of the function to call.
    func_id: u16,
    /// The number of arguments that are valid.
    arg_count: u8,

    /// Storage for the 1st argument.
    arg_1: u64,
    /// Storage for the 2nd argument.
    arg_2: u64,
    /// Storage for the 3rd argument.
    arg_3: u64,
    /// Storage for the 4th argument.
    arg_4: u64,
    /// Storage for the 5th argument.
    arg_5: u64,
    /// Storage for the 6th argument.
    arg_6: u64,

    /// Storage for the return value.
    ret: u64,

    /// Temporary storage for cross-address space storage.
    tmp: u64,
}

/// Information vital to proper function of the CPU which might be changed between the loader
/// address space and the application address space.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct CpuData {
    /// The CR3 value associated with the address space.
    cr3: u64,

    /// The stack pointer associated with the address space.
    sp: u64,

    /// The CS value associated with the address space.
    cs: u16,
    /// The DS value associated with the address space.
    ds: u16,
    /// The ES value associated with the address space.
    es: u16,

    /// The size of the GDT.
    gdt_size: u16,
    /// The address of the GDT.
    gdt_address: u64,

    /// The FS value associated with the address space.
    fs: u16,
    /// The GS value associated with the address space.
    gs: u16,
    /// The SS value associated with the address space.
    ss: u16,

    /// The size of the IDT.
    idt_size: u16,
    /// The address of the IDT.
    idt_address: u64,
}

unsafe extern "C" {
    /// The start of the identity-mapped code.
    #[link_name = "_CODE_START"]
    static CODE_START: u8;

    /// The location of the `call_function` function.
    #[link_name = "call_function"]
    static CALL_FUNCTION: u8;

    /// The end of the code that must be identity-mapped.
    #[link_name = "_IDENTITY_END"]
    static IDENTIY_END: u8;

    /// The location of the Divide Error exception handler.
    #[link_name = "DIVIDE_ERROR_HANDLER"]
    static DIVIDE_ERROR_HANDLER: u8;

    /// The location of the Debug exception handler.
    #[link_name = "DEBUG_HANDLER"]
    static DEBUG_HANDLER: u8;

    /// The location of the Invalid Opcode exception handler.
    #[link_name = "INVALID_OPCODE_HANDLER"]
    static INVALID_OPCODE_HANDLER: u8;

    /// The location of the Page Fault exception handler.
    #[link_name = "PAGE_FAULT_HANDLER"]
    static PAGE_FAULT_HANDLER: u8;

    static WRITE: u8;
    static ALLOCATE_FRAMES: u8;
    static DEALLOCATE_FRAMES: u8;
    static GET_MEMORY_MAP: u8;
    static MAP: u8;
    static UNMAP: u8;
    static TAKEOVER: u8;

    /// The end of the identity-mapped code.
    #[link_name = "_CODE_END"]
    static CODE_END: u8;

    static GENERIC_HANDLER_START: u8;
    static GENERIC_HANDLER_MOV_INSTRUCTION: u8;
    static GENERIC_HANDLER_CODE_DATA_START_ADDRESS: u8;
    static GENERIC_HANDLER_CALL_FUNCTION: u8;
    static GENERIC_HANDLER_HANDLE_CALL: u8;
    static GENERIC_HANDLER_END: u8;
}

// Shared code.
#[rustfmt::skip]
global_asm!(
    // Signals the start of the shared code.
    ".global _CODE_START",
    "_CODE_START:",

    ".global _CODE_DATA",
    "_CODE_DATA:",
    ".space {IDENTITY_MAPPED_DATA_SIZE}",

    // Arguments:
    //
    // rax: func_id
    // r10: arg_count
    // r11: 0 if entering from application; otherwise entering from loader.
    // r12: address of function to call.
    //
    // rdi: arg 1
    // rsi: arg 2
    // rdx: arg 3
    // rcx: arg 4
    // r8:  arg 5
    // r9:  arg 6
    ".global call_function",
    "call_function:",

    // Store func_id.
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_FUNC_ID}], ax",

    // Store arg_count.
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_COUNT}], r10b",

    // Preserve `r15`.
    "push r15",

    // Save `r11` and store caller state.
    "mov r15, r11",
    "call store_state",

    // Refresh `r11` and load callee state.
    "mov r11, r15",
    "call load_state",

    // Store arguments.
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_1}], rdi",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_2}], rsi",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_3}], rdx",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_4}], rcx",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_5}], r8",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_ARG_6}], r9",

    "call r12",

    // Store return value.
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_RET}], rax",

    // Swap entrance side.
    "cmp r15, 0",
    "mov r10, 1",
    "cmove r15, r10",
    "mov r10, 0",
    "cmovne r15, r10",

    // Refresh `r11` and store callee state.
    "mov r11, r15",
    "call store_state",

    // Refresh `r11` and load caller state.
    "mov r11, r15",
    "call load_state",

    // Ensure the proper preservation of `r15`.
    "pop r15",
    "ret",

    // Arguments:
    //
    // r11: 0 if entering from application; otherwise entering from loader.
    //
    // rdi: arg 1 / return value
    // rsi: arg 2
    // rdx: arg 3
    // rcx: arg 4
    // r8:  arg 5
    "store_state:",

    // Calculate base address of the correct [`CpuData`] structure.
    "lea r10, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_LOADER_OFFSET}]",
    "cmp r11, 0",
    "lea r11, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_APPLICATION_OFFSET}]",
    "cmove r10, r11",

    // Store active CR3.
    "mov r11, cr3",
    "mov [r10 + {CPU_DATA_CR3_OFFSET}], r11",

    // Adjust `rsp` to not include call to this function.
    "mov r11, rsp",
    "add r11, 8",
    "mov [r10 + {CPU_DATA_SP_OFFSET}], r11",

    // Store segment registers.
    "mov r11w, cs",
    "mov [r10 + {CPU_DATA_CS_OFFSET}], r11w",
    "mov r11w, ds",
    "mov [r10 + {CPU_DATA_DS_OFFSET}], r11w",
    "mov r11w, es",
    "mov [r10 + {CPU_DATA_ES_OFFSET}], r11w",
    "mov r11w, fs",
    "mov [r10 + {CPU_DATA_FS_OFFSET}], r11w",
    "mov r11w, gs",
    "mov [r10 + {CPU_DATA_GS_OFFSET}], r11w",
    "mov r11w, ss",
    "mov [r10 + {CPU_DATA_SS_OFFSET}], r11w",

    // Store GDTR and IDTR.
    "sgdt [r10 + {CPU_DATA_GDT_OFFSET}]",
    "sidt [r10 + {CPU_DATA_IDT_OFFSET}]",

    "ret",

    // Arguments:
    //
    // r11: 0 if entering from loader; otherwise entering from application
    "load_state:",
    
    "mov r10, [rsp]",
    "mov [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_TMP}], r10",

    // Calculate base address of the correct [`CpuData`] structure.
    "lea r10, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_LOADER_OFFSET}]",
    "cmp r11, 0",
    "lea r11, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_APPLICATION_OFFSET}]",
    "cmovne r10, r11",

    // Load stored CR3 value.
    "mov r11, [r10 + {CPU_DATA_CR3_OFFSET}]",
    "mov cr3, r11",

    // Load stored stack pointer.
    "mov rsp, [r10 + {CPU_DATA_SP_OFFSET}]",

    // Load GDTR and IDTR.
    "lgdt [r10 + {CPU_DATA_GDT_OFFSET}]",
    "lidt [r10 + {CPU_DATA_IDT_OFFSET}]",

    // Load data segment registers.
    "mov r11w, [r10 + {CPU_DATA_DS_OFFSET}]",
    "mov ds, r11w",
    "mov r11w, [r10 + {CPU_DATA_ES_OFFSET}]",
    "mov es, r11w",
    "mov r11w, [r10 + {CPU_DATA_FS_OFFSET}]",
    "mov fs, r11w",
    "mov r11w, [r10 + {CPU_DATA_GS_OFFSET}]",
    "mov gs, r11w",
    "mov r11w, [r10 + {CPU_DATA_SS_OFFSET}]",
    "mov ss, r11w",

    // Load code segment register.
    "xor r11, r11",
    "mov r11w, [r10 + {CPU_DATA_CS_OFFSET}]",
    "push r11",
    "lea r11, [rip + 5f]",
    "push r11",
    "retfq",
    "5:",

    "mov r10, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_TMP_TMP}]",
    "jmp r10",

    ".global _IDENTITY_END",
    "_IDENTITY_END:",

    ".global DIVIDE_ERROR_HANDLER",
    "DIVIDE_ERROR_HANDLER:",
    
    "mov rax, {DIVIDE_ERROR_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "mov rdi, [rsp + 0]",
    "mov rsi, [rsp + 8]",
    "lea r9, [rip + _CODE_DATA]",
    "call call_function",
    "iret",

    ".global DEBUG_HANDLER",
    "DEBUG_HANDLER:",

    "mov rax, {DEBUG_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "mov rdi, [rsp + 0]",
    "mov rsi, [rsp + 8]",
    "lea r9, [rip + _CODE_DATA]",
    "call call_function",
    "iret",

    ".global INVALID_OPCODE_HANDLER",
    "INVALID_OPCODE_HANDLER:",

    "mov rax, {INVALID_OPCODE_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "mov rdi, [rsp + 0]",
    "mov rsi, [rsp + 8]",
    "lea r9, [rip + _CODE_DATA]",
    "call call_function",
    "iret",

    ".global PAGE_FAULT_HANDLER",
    "PAGE_FAULT_HANDLER:",

    "mov rax, {PAGE_FAULT_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "mov rdi, [rsp + 8]",
    "mov rsi, [rsp + 16]",
    "mov rdx, [rsp + 0]",
    "lea r9, [rip + _CODE_DATA]",
    "call call_function",
    "iret",

    ".global WRITE",
    "WRITE:",

    "mov rax, {WRITE_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global ALLOCATE_FRAMES",
    "ALLOCATE_FRAMES:",

    "mov rax, {ALLOCATE_FRAMES_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global DEALLOCATE_FRAMES",
    "DEALLOCATE_FRAMES:",

    "mov rax, {DEALLOCATE_FRAMES_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global GET_MEMORY_MAP",
    "GET_MEMORY_MAP:",

    "mov rax, {GET_MEMORY_MAP_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global MAP",
    "MAP:",

    "mov rax, {MAP_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global UNMAP",
    "UNMAP:",

    "mov rax, {UNMAP_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    "lea r9, [rip + _CODE_DATA]",
    "call call_function",

    "ret",

    ".global TAKEOVER",
    "TAKEOVER:",

    "mov rax, {TAKEOVER_ID}",
    "mov r10, 2",
    "mov r11, 0",
    "mov r12, [rip + _CODE_DATA + {IDENTITY_MAPPED_DATA_HANDLE_CALL}]",

    // Signals the end of the shared code.
    ".global _CODE_END",
    "_CODE_END:",

    IDENTITY_MAPPED_DATA_SIZE = const { mem::size_of::<IdentityMappedData>() },
    
    IDENTITY_MAPPED_DATA_LOADER_OFFSET = const { mem::offset_of!(IdentityMappedData, loader) },
    IDENTITY_MAPPED_DATA_APPLICATION_OFFSET = const { mem::offset_of!(IdentityMappedData, application) },
    
    IDENTITY_MAPPED_DATA_HANDLE_CALL = const { mem::offset_of!(IdentityMappedData, handle_call) },

    IDENTITY_MAPPED_DATA_TMP_FUNC_ID = const { mem::offset_of!(IdentityMappedData, tmp_storage.func_id) },
    IDENTITY_MAPPED_DATA_TMP_ARG_COUNT = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_count) },
    IDENTITY_MAPPED_DATA_TMP_ARG_1 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_1) },
    IDENTITY_MAPPED_DATA_TMP_ARG_2 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_2) },
    IDENTITY_MAPPED_DATA_TMP_ARG_3 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_3) },
    IDENTITY_MAPPED_DATA_TMP_ARG_4 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_4) },
    IDENTITY_MAPPED_DATA_TMP_ARG_5 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_5) },
    IDENTITY_MAPPED_DATA_TMP_ARG_6 = const { mem::offset_of!(IdentityMappedData, tmp_storage.arg_6) },
    IDENTITY_MAPPED_DATA_TMP_RET = const { mem::offset_of!(IdentityMappedData, tmp_storage.ret) },
    IDENTITY_MAPPED_DATA_TMP_TMP = const { mem::offset_of!(IdentityMappedData, tmp_storage.tmp) },

    CPU_DATA_CR3_OFFSET = const { mem::offset_of!(CpuData, cr3) },
    CPU_DATA_SP_OFFSET = const { mem::offset_of!(CpuData, sp) },
    CPU_DATA_GDT_OFFSET = const { mem::offset_of!(CpuData, gdt_size) },
    CPU_DATA_IDT_OFFSET = const { mem::offset_of!(CpuData, idt_size) },
    
    CPU_DATA_CS_OFFSET = const { mem::offset_of!(CpuData, cs) },
    CPU_DATA_DS_OFFSET = const { mem::offset_of!(CpuData, ds) },
    CPU_DATA_ES_OFFSET = const { mem::offset_of!(CpuData, es) },
    CPU_DATA_FS_OFFSET = const { mem::offset_of!(CpuData, fs) },
    CPU_DATA_GS_OFFSET = const { mem::offset_of!(CpuData, gs) },
    CPU_DATA_SS_OFFSET = const { mem::offset_of!(CpuData, ss) },

    WRITE_ID = const { WRITE_ID },
    ALLOCATE_FRAMES_ID = const { ALLOCATE_FRAMES_ID },
    DEALLOCATE_FRAMES_ID = const { DEALLOCATE_FRAMES_ID },
    GET_MEMORY_MAP_ID = const { GET_MEMORY_MAP_ID },
    MAP_ID = const { MAP_ID },
    UNMAP_ID = const { UNMAP_ID },
    TAKEOVER_ID = const { TAKEOVER_ID },

    DIVIDE_ERROR_ID = const { DIVIDE_ERROR_ID },
    DEBUG_ID = const { DEBUG_ID },
    INVALID_OPCODE_ID = const { INVALID_OPCODE_ID },
    PAGE_FAULT_ID = const { PAGE_FAULT_ID },
);

#[rustfmt::skip]
global_asm!(
    ".global GENERIC_HANDLER_START",
    "GENERIC_HANDLER_START:",
    
    "mov rax, {GENERIC_ID}",
    "mov r10, 1",
    "mov r11, 0",
    "mov r12, [rip + GENERIC_HANDLER_HANDLE_CALL]",

    ".global GENERIC_HANDLER_MOV_INSTRUCTION",
    "GENERIC_HANDLER_MOV_INSTRUCTION:",

    // mov rdi, imm32
    ".byte 0xBF, 0x00, 0x00, 0x00, 0x00",
    "mov r9, [rip + GENERIC_HANDLER_CODE_DATA_START_ADDRESS]",

    "mov rsi, [rip + GENERIC_HANDLER_CALL_FUNCTION]",
    "call rsi",
    "iret",

    ".global GENERIC_HANDLER_CODE_DATA_START_ADDRESS",
    "GENERIC_HANDLER_CODE_DATA_START_ADDRESS:",
    ".8byte 0",

    ".global GENERIC_HANDLER_CALL_FUNCTION",
    "GENERIC_HANDLER_CALL_FUNCTION:",
    ".8byte 0",

    ".global GENERIC_HANDLER_HANDLE_CALL",
    "GENERIC_HANDLER_HANDLE_CALL:",
    ".8byte 0",
    
    ".global GENERIC_HANDLER_END",
    "GENERIC_HANDLER_END:",

    GENERIC_ID = const { GENERIC_ID },
);

#[repr(C)]
struct RevmProtocolTable {
    header: Header,
    generic_table: GenericTable,
    x86_64_table: X86_64Table,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    fn interrupt(handler: u64, selector: u16) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist: 0,
            type_attr: 0x8F, // present | DPL=0 | interrupt gate
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            zero: 0,
        }
    }
}

//! Implementation of cross address space switching related functionality for `x86_32` and
//! `x86_64`.

use core::{mem, ptr};

use conversion::{u64_to_usize_strict, usize_to_u64};
use memory::address::PhysicalAddress;
use stub_api::{x86_32::X86_32Table, x86_64::X86_64Table};
use sync::Spinlock;
use x86_common::{
    control::{Cr0, Cr4},
    cpuid::{cpuid_unchecked, supports_cpuid},
    paging::{PagingMode, current_paging_mode},
};

use crate::{
    arch::{
        generic::switch::{
            func::{
                ENTER_FUNC_ID, EXEC_ON_PROCESSOR_FUNC_ID, MAX_GENERIC_EXECUTABLE_ID,
                RUN_ON_ALL_PROCESSORS_FUNC_ID, TAKEOVER_FUNC_ID, handle_call,
            },
            setup::{
                CodeLayout, ComponentError, CpuData, RevmProtocolTable32, RevmProtocolTable64,
                switch_data,
            },
        },
        paging::ArchScheme,
    },
    platform::{
        AllocationPolicy, device_tree, main_processor_id, rsdp, run_on_all_processors, smbios_32,
        smbios_64, uefi_system_table, write_bytes_at, xsdp,
    },
};

mod x86_64;

/// Allocates and maps a region of memory, then places cross address space switching code into the
/// region of memory.
#[expect(clippy::missing_errors_doc)]
pub fn allocate_code(
    scheme: &mut ArchScheme,
    storage_pointer: u64,
    storage: &mut CpuStorage,
    for_stub: bool,
) -> Result<CodeLayout, ComponentError> {
    let stub = storage_pointer.strict_add(usize_to_u64(mem::offset_of!(CpuStorage, stub)));
    let executable =
        storage_pointer.strict_add(usize_to_u64(mem::offset_of!(CpuStorage, executable)));
    let (own, other, long_mode) = if for_stub {
        (stub, executable, ((storage.stub.efer >> 8) & 0b1) == 0b1)
    } else {
        (
            executable,
            stub,
            ((storage.executable.efer >> 8) & 0b1) == 0b1,
        )
    };

    if long_mode {
        x86_64::allocate_code(scheme, storage_pointer, own, other)
    } else {
        todo!("implement x86_32")
    }
}

/// Returns the [`AllocationPolicy`] used when allocating switch data.
pub fn arch_policy() -> AllocationPolicy {
    AllocationPolicy::Below(u32::MAX as u64 + 1)
}

/// Returns the size, in bytes, of the architecture-dependent portion of the protocol table.
pub fn arch_table_size(scheme: &mut ArchScheme) -> usize {
    if scheme.long_mode() {
        mem::size_of::<RevmProtocolTable64<X86_64Table>>()
    } else {
        mem::size_of::<RevmProtocolTable32<X86_32Table>>()
    }
}

/// Returns `true` if the protocol table should be 64 bits.
pub fn arch_table_64_bit(scheme: &mut ArchScheme) -> bool {
    scheme.long_mode()
}

/// Constructs a [`CpuStorage`] to be used as the base for every CPU's future [`CpuStorage`]s.
pub fn base_cpu_storage(scheme: &mut ArchScheme) -> CpuStorage {
    let stub_long_mode = match current_paging_mode() {
        PagingMode::Disabled | PagingMode::Bits32 | PagingMode::Pae => false,
        PagingMode::Level4 | PagingMode::Level5 => true,
    };

    let executable_pse_bit = scheme.pse();
    let executable_pae_bit = scheme.pae();
    let executable_long_mode = scheme.long_mode();
    let executable_la57_bit = scheme.la57();

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
    let cr3 = scheme.cr3();
    let cr4 = Cr4::from_bits(0)
        .set_pse(executable_pse_bit)
        .set_pae(executable_pae_bit)
        .set_la57(executable_la57_bit)
        .to_bits();
    let efer = (u64::from(scheme.nxe()) << 11) | (u64::from(executable_long_mode) << 8);

    CpuStorage {
        call: CallStorage::default(),
        stub: ModeStorage {
            efer: u64::from(stub_long_mode) << 8,
            ..Default::default()
        },
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

            efer,
            ..Default::default()
        },
        change_efer: if supports_cpuid() {
            // SAFETY:
            //
            // `CPUID` is supported.
            let result = unsafe { cpuid_unchecked(0x80000001, 0) };

            (((result.edx >> 20) | (result.edx >> 29)) & 0b1) as u8
        } else {
            0
        },
        gdt: [
            0x0000_0000_0000_0000, // Null Segment
            0x00CF_9B00_0000_FFFF, // Kernel 32-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 32-bit data segment
            0x00AF_9B00_0000_FFFF, // Kernel 64-bit code segment
            0x00CF_9300_0000_FFFF, // Kernel 64-bit data segment
        ],
    }
}

/// Handler for cross address space function calls.
extern "C" fn call_handler() -> stub_api::Status {
    // SAFETY:
    //
    // An IDT has been installed and thus it is safe to enable interrupts.
    unsafe { core::arch::asm!("sti") }

    let mut lock = switch_data();
    let lock = lock.as_mut().expect("SwitchData must be initialized");

    let (scheme, cpu_data_slice) = lock.both_mut();
    let mut cpu_data = cpu_data_slice[u64_to_usize_strict(main_processor_id())].lock();
    let storage = cpu_data.storage_mut();

    let result = match storage.call.func_id {
        RUN_ON_ALL_PROCESSORS_FUNC_ID => {
            let func = storage.call.arg_0;
            let arg = storage.call.arg_1;
            drop(cpu_data);

            let mut data = (cpu_data_slice, func, arg);
            run_on_all_processors(exec_all, ptr::from_mut(&mut data).cast::<()>());
            core::hint::black_box(data);
            stub_api::Status::SUCCESS
        }
        0..=MAX_GENERIC_EXECUTABLE_ID => {
            let result = handle_call(
                scheme,
                storage.call.func_id,
                storage.call.arg_0,
                storage.call.arg_1,
                storage.call.arg_2,
                storage.call.arg_3,
                storage.call.arg_4,
            );

            if storage.call.func_id == TAKEOVER_FUNC_ID && result == stub_api::Status::SUCCESS {
                let func = storage.call.arg_2;
                let arg = 0;
                drop(cpu_data);

                let mut data = (cpu_data_slice, func, arg);
                run_on_all_processors(exec_all, ptr::from_mut(&mut data).cast::<()>());
                unreachable!()
            }

            result
        }
        func_id => unreachable!("invalid func_id: {func_id}"),
    };

    // SAFETY:
    //
    // As bare-metal ring 0 application, it is always safe to disable interrupts.
    unsafe { core::arch::asm!("cli") }
    result
}

/// Executes the provided function on the provided processor.
extern "C" fn exec_all(cpu_id: u64, arg: *mut ()) {
    // SAFETY:
    //
    // TODO:
    let (cpu_data_slice, func, revm_arg) =
        unsafe { *arg.cast::<(&'static &'static [Spinlock<CpuData>], u64, u64)>() };
    let mut cpu_data = cpu_data_slice[u64_to_usize_strict(cpu_id)].lock();
    let storage = cpu_data.storage_mut();

    storage.call.func_id = EXEC_ON_PROCESSOR_FUNC_ID;
    storage.call.arg_count = 3;

    storage.call.arg_0 = func;
    storage.call.arg_1 = cpu_id;
    storage.call.arg_2 = revm_arg;

    let call = cpu_data.stub.arch_code_layout.call;
    drop(cpu_data);

    // SAFETY:
    //
    // The executable's address space and switching code has been correctly prepared.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "cli",

            "call r10",

            "sti",

            in("r10") call,
            clobber_abi("sysv64")
        )
    }
}

/// Enters the executable at `entry_point` with the provided `protocol_table` as the first and only
/// argument.
///
/// # Panics
///
/// Panics if the switching data is not available.
pub fn enter(entry_point: u64, protocol_table: u64) -> stub_api::Status {
    let mut switch_data_lock = switch_data();
    let switch_data = switch_data_lock
        .as_mut()
        .expect("SwitchData must be initialized");
    let main_cpu_data = &switch_data.cpu_data_mut()[u64_to_usize_strict(main_processor_id())];
    let mut main_cpu_data_lock = main_cpu_data.lock();

    main_cpu_data_lock.storage_mut().call.func_id = ENTER_FUNC_ID;
    main_cpu_data_lock.storage_mut().call.arg_count = 2;
    main_cpu_data_lock.storage_mut().call.arg_0 = entry_point;
    main_cpu_data_lock.storage_mut().call.arg_1 = protocol_table;

    let call = main_cpu_data_lock.stub.arch_code_layout.call;

    drop(main_cpu_data_lock);
    drop(switch_data_lock);
    // SAFETY:
    //
    // The executable's address space and switching code has been correctly prepared.
    #[cfg(target_arch = "x86_64")]
    let result = unsafe {
        let result: u64;
        core::arch::asm!(
            "cli",

            "call r10",

            "sti",

            lateout("rax") result,

            in("r10") call,
            clobber_abi("sysv64")
        );
        result
    };

    stub_api::Status(result)
}

/// Finalizes the provided [`CpuData`] and [`CpuStorage`].
pub fn finalize_cpu_data(cpu_data: &mut CpuData, storage: &mut CpuStorage) {
    storage.executable.enter_mode = cpu_data.executable.arch_code_layout.enter_mode;
    storage.executable.call_handler = cpu_data.executable.arch_code_layout.call_handler;
    storage.executable.call = cpu_data.executable.arch_code_layout.call;

    storage.stub.enter_mode = cpu_data.stub.arch_code_layout.enter_mode;
    storage.stub.call_handler = call_handler as *const () as u64;
    storage.stub.call = cpu_data.stub.arch_code_layout.call;
}

/// Adjusts the provided [`CpuStorage`] to utilize the newly allocated stack with the provided
/// `stack_top`.
pub fn handle_stack_allocation(storage: &mut CpuStorage, stack_top: u64) {
    storage.executable.rsp = stack_top;
}

/// Adjusts the provided [`CpuStorage`] to utilize the newly allocated space for the [`CpuStorage`]
/// at `storage_base`.
pub fn handle_storage_allocation(storage: &mut CpuStorage, storage_base: u64) {
    storage.executable.gdtr.size = 5 * 8 - 1;
    storage.executable.gdtr.pointer =
        storage_base.strict_add(usize_to_u64(mem::offset_of!(CpuStorage, gdt)));
}

/// Writes the finished 32-bit protocol table at the provided `address`.
pub fn write_protocol_table_32(table: RevmProtocolTable32<()>, address: PhysicalAddress) {
    let arch_table = X86_32Table {
        version: X86_64Table::VERSION,

        uefi_system_table: uefi_system_table().map(|addr| addr.value()).unwrap_or(0),
        rsdp: rsdp().map(|addr| addr.value()).unwrap_or(0),
        xsdp: xsdp().map(|addr| addr.value()).unwrap_or(0),
        device_tree: device_tree().map(|addr| addr.value()).unwrap_or(0),
        smbios_32: smbios_32().map(|addr| addr.value()).unwrap_or(0),
        smbios_64: smbios_64().map(|addr| addr.value()).unwrap_or(0),
    };
    let table = table.transpose(arch_table);

    // SAFETY:
    //
    // TODO:
    let protocol_table_slice = unsafe {
        core::slice::from_raw_parts((&raw const table).cast::<u8>(), mem::size_of_val(&table))
    };
    write_bytes_at(address, protocol_table_slice);
}

/// Writes the 64-bit protcol table to the provided `address`.
pub fn write_protocol_table_64(table: RevmProtocolTable64<()>, address: PhysicalAddress) {
    let arch_table = X86_64Table {
        version: X86_64Table::VERSION,

        uefi_system_table: uefi_system_table().map(|addr| addr.value()).unwrap_or(0),
        rsdp: rsdp().map(|addr| addr.value()).unwrap_or(0),
        xsdp: xsdp().map(|addr| addr.value()).unwrap_or(0),
        device_tree: device_tree().map(|addr| addr.value()).unwrap_or(0),
        smbios_32: smbios_32().map(|addr| addr.value()).unwrap_or(0),
        smbios_64: smbios_64().map(|addr| addr.value()).unwrap_or(0),
    };
    let table = table.transpose(arch_table);

    // SAFETY:
    //
    // TODO:
    let protocol_table_slice = unsafe {
        core::slice::from_raw_parts((&raw const table).cast::<u8>(), mem::size_of_val(&table))
    };
    write_bytes_at(address, protocol_table_slice);
}

/// Additional architecture-specific code layout information.
#[derive(Debug)]
pub struct ArchCodeLayout {
    /// The address of the mode entrance function.
    enter_mode: u64,

    /// The address of the cross address space call handler function.
    call_handler: u64,
    /// The address of the cross address space call creator function.
    call: u64,
}

/// The data required per CPU for switching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuStorage {
    /// Information regarding cross address space function calls.
    call: CallStorage,

    /// Storage used for the stub's part of address space switching.
    stub: ModeStorage,
    /// Storage used for the executable's part of address space switching.
    executable: ModeStorage,

    /// If non-zero, then the `EFER` MSR should be changed.
    change_efer: u8,

    /// A hard-coded GDT used for the executable and switching code.
    gdt: [u64; 5],
}

/// Storage used for cross address space function calls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CallStorage {
    /// The ID representing the function to be called.
    func_id: u16,
    /// The number of arguments that are currently active.
    arg_count: u8,

    /// The value of the 0th argument.
    arg_0: u64,
    /// The value of the 1st argument.
    arg_1: u64,
    /// The value of the 2nd argument.
    arg_2: u64,
    /// The value of the 3rd argument.
    arg_3: u64,
    /// The value of the 4th argument.
    arg_4: u64,
    /// The value of the 5th argument.
    arg_5: u64,

    /// The return value.
    ret: u64,
}

/// Storage used to properly switch between address spaces and retain important register values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModeStorage {
    /// The address of the `enter_mode` assembly procedure.
    enter_mode: u64,

    /// The address of the cross address space function call handler.
    call_handler: u64,
    /// The address of the cross address space function call creator.
    call: u64,

    /// The stored value of the `rax` register.
    rax: u64,
    /// The stored value of the `rbx` register.
    rbx: u64,
    /// The stored value of the `rcx` register.
    rcx: u64,
    /// The stored value of the `rdx` register.
    rdx: u64,
    /// The stored value of the `rsi` register.
    rsi: u64,
    /// The stored value of the `rdi` register.
    rdi: u64,
    /// The stored value of the `rsp` register.
    rsp: u64,
    /// The stored value of the `rbp` register.
    rbp: u64,
    /// The stored value of the `r8` register.
    r8: u64,
    /// The stored value of the `r9` register.
    r9: u64,
    /// The stored value of the `r10` register.
    r10: u64,
    /// The stored value of the `r11` register.
    r11: u64,
    /// The stored value of the `r12` register.
    r12: u64,
    /// The stored value of the `r13` register.
    r13: u64,
    /// The stored value of the `r14` register.
    r14: u64,
    /// The stored value of the `r15` register.
    r15: u64,

    /// The stored value of the `CS` register.
    cs: u16,
    /// The stored value of the `DS` register.
    ds: u16,
    /// The stored value of the `ES` register.
    es: u16,
    /// The stored value of the `FS` register.
    fs: u16,
    /// The stored value of the `GS` register.
    gs: u16,
    /// The stored value of the `SS` register.
    ss: u16,

    /// The stored value of the `CR0` register.
    cr0: u64,
    /// The stored value of the `CR3` register.
    cr3: u64,
    /// The stored value of the `CR4` register.
    cr4: u64,

    /// The stored value of the `IDTR`.
    gdtr: TablePointer,
    /// The stored value of the `IDTR`.
    idtr: TablePointer,

    /// The stored value of the `IA32_EFER` MSR.
    efer: u64,

    /// Temporary storage used for switching.
    tmp_storage: [u64; 5],
}

/// A packed representation of a `GDTR` or `IDTR`.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TablePointer {
    /// The size, in bytes, of the table.
    size: u16,
    /// The linear address of the table.
    pointer: u64,
}

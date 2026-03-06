//! Implementation of cross address space switching related functionality for `aarch64`.

use core::mem;

use aarch64::common::{Granule, PhysicalAddressSpaceSize};
use conversion::{u64_to_usize_strict, usize_to_u64};
use memory::address::PhysicalAddress;
use stub_api::aarch64::Aarch64Table;
use sync::Spinlock;

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

mod code;

/// Allocates and maps a region of memory, then places cross address space switching code into the
/// region of memory.
#[expect(clippy::missing_errors_doc)]
pub fn allocate_code(
    scheme: &mut ArchScheme,
    storage_pointer: u64,
    _storage: &mut CpuStorage,
    for_stub: bool,
) -> Result<CodeLayout, ComponentError> {
    let stub = storage_pointer.strict_add(usize_to_u64(mem::offset_of!(CpuStorage, stub)));
    let executable =
        storage_pointer.strict_add(usize_to_u64(mem::offset_of!(CpuStorage, executable)));
    let (own, other) = if for_stub {
        (stub, executable)
    } else {
        (executable, stub)
    };

    code::allocate_code(scheme, storage_pointer, own, other)
}

/// Returns the [`AllocationPolicy`] used when allocating switch data.
pub fn arch_policy() -> AllocationPolicy {
    AllocationPolicy::Any
}

/// Returns the size, in bytes, of the protocol table for this architecture.
pub fn arch_table_size(_scheme: &mut ArchScheme) -> usize {
    mem::size_of::<RevmProtocolTable64<Aarch64Table>>()
}

/// Returns `true` if the protocol table should be 64 bits.
pub fn arch_table_64_bit(_scheme: &mut ArchScheme) -> bool {
    true
}

/// Constructs a [`CpuStorage`] to be used as the base for every CPU's future [`CpuStorage`].
pub fn base_cpu_storage(scheme: &mut ArchScheme) -> CpuStorage {
    let mut tcr_elx = 0;
    if scheme.ttbr0_enabled() {
        tcr_elx |= u64::from(scheme.t0sz());
        tcr_elx |= 0b01 << 8;
        tcr_elx |= 0b01 << 10;
        tcr_elx |= 0b11 << 12;
        let granule = match scheme.granule() {
            Granule::Page4KiB => 0b00,
            Granule::Page16KiB => 0b10,
            Granule::Page64KiB => 0b01,
        };
        tcr_elx |= granule << 14;
    } else {
        // Set the EPD0 bit, which disables translation table walks using TTBR0.
        tcr_elx |= 1u64 << 7;
    }

    if scheme.ttbr1_enabled() {
        tcr_elx |= u64::from(scheme.t1sz()) << 16;
        tcr_elx |= 0b01 << 24;
        tcr_elx |= 0b01 << 26;
        tcr_elx |= 0b11 << 28;
        let granule = match scheme.granule() {
            Granule::Page4KiB => 0b10,
            Granule::Page16KiB => 0b01,
            Granule::Page64KiB => 0b11,
        };
        tcr_elx |= granule << 30;
    } else {
        // Set the EPD1 bit, which disables translation table walks using TTBR0.
        tcr_elx |= 1u64 << 23;
    }

    let ipa = match scheme.ipa() {
        PhysicalAddressSpaceSize::Bits32 => 0b000,
        PhysicalAddressSpaceSize::Bits36 => 0b001,
        PhysicalAddressSpaceSize::Bits40 => 0b010,
        PhysicalAddressSpaceSize::Bits42 => 0b011,
        PhysicalAddressSpaceSize::Bits44 => 0b100,
        PhysicalAddressSpaceSize::Bits48 => 0b101,
        PhysicalAddressSpaceSize::Bits52 => 0b110,
        PhysicalAddressSpaceSize::Bits56 => 0b111,
    };
    tcr_elx |= ipa << 32;

    let ttbr0_elx = if scheme.ttbr0_enabled() {
        scheme.ttbr0()
    } else {
        0
    };

    let ttbr1_elx = if scheme.ttbr1_enabled() {
        scheme.ttbr1()
    } else {
        0
    };

    CpuStorage {
        call: CallStorage::default(),
        stub: ModeStorage::default(),
        executable: ModeStorage {
            tcr_elx,
            ttbr0_elx,
            ttbr1_elx,

            sctlr_elx: 0b1,

            ..Default::default()
        },
    }
}

/// Handler for cross address space function calls.
extern "C" fn call_handler() -> stub_api::Status {
    // SAFETY:
    //
    // The stub has interrupt handling working.
    unsafe { core::arch::asm!("msr daifclr, #3") }

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

            let data = (&*cpu_data_slice, func, arg);
            run_on_all_processors::<ExecAllData>(exec_all, &data);
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

                let data = (&*cpu_data_slice, func, arg);
                run_on_all_processors::<ExecAllData>(exec_all, &data);
                unreachable!()
            }

            result
        }
        func_id => unreachable!("invalid func_id: {func_id}"),
    };

    // SAFETY:
    //
    // It is always safe to disable interrupts, since the program runs in EL1 or EL2.
    unsafe { core::arch::asm!("msr daifset, #3") }
    result
}

/// Type passed to the [`ExecAllData`] function.
type ExecAllData<'a> = (&'a [Spinlock<CpuData>], u64, u64);

/// Executes the provided function on the provided processor.
extern "C" fn exec_all(cpu_id: u64, arg: *mut ()) {
    let arg = arg.cast::<ExecAllData>();
    // SAFETY:
    //
    // TODO:
    let (cpu_data_slice, func, revm_arg) = unsafe { *arg };
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
    unsafe {
        core::arch::asm!(
            "msr daifset, #3",

            "blr x16",

            "msr daifclr, #3",
            in("x16") call,
            clobber_abi("C")
        );
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
    let result = unsafe {
        let result: u64;
        core::arch::asm!(
            "msr daifset, #3",

            "blr x16",

            "msr daifclr, #3",
            in("x16") call,
            lateout("x0") result,
            clobber_abi("C")
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
    storage.executable.sp = stack_top;
}

/// Adjusts the provided [`CpuStorage`] to utilize the newly allocated space for the [`CpuStorage`]
/// at `storage_base`.
pub fn handle_storage_allocation(_storage: &mut CpuStorage, _storage_base: u64) {}

/// Writes the finished 32-bit protocol table at the provided `address`.
pub fn write_protocol_table_32(_table: RevmProtocolTable32<()>, _address: PhysicalAddress) {
    unimplemented!()
}

/// Writes the 64-bit protcol table to the provided `address`.
pub fn write_protocol_table_64(table: RevmProtocolTable64<()>, address: PhysicalAddress) {
    let arch_table = Aarch64Table {
        version: Aarch64Table::VERSION,

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
#[expect(clippy::missing_docs_in_private_items)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModeStorage {
    /// The address of the `enter_mode` assembly procedure.
    enter_mode: u64,

    /// The address of the cross address space function call handler.
    call_handler: u64,
    /// The address of the cross address space function call creator.
    call: u64,

    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    x29: u64,
    x30: u64,

    // Stack pointer.
    sp: u64,

    /// Translation control register.
    tcr_elx: u64,
    /// Translation table base registers.
    ttbr0_elx: u64,
    ttbr1_elx: u64,

    /// Memory attribute indirection register.
    mair_elx: u64,
    /// System control register.
    sctlr_elx: u64,
    /// Exception vector base address.
    vbar_elx: u64,

    /// Exception link register (return PC from exception).
    elr_elx: u64,
    /// Saved program status register for exceptions.
    spsr_elx: u64,
}

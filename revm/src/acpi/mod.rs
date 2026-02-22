//! ACPI.
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(unused)]

use core::{
    ffi, mem,
    ptr::{self, NonNull},
};

use stub_api::{MapFlags, Status};
use sync::RawSpinlock;
use uacpi_sys::*;

use crate::{
    memory::{
        general::{allocate, deallocate},
        phys::structs::{Frame, FrameRange, PhysicalAddress},
    },
    stub_protocol,
    util::{u64_to_usize_panicking, usize_to_u64},
};

pub fn initialize() {
    let initialize_flags = u64::from(UACPI_FLAG_NO_ACPI_MODE);
    let result = unsafe { uacpi_initialize(initialize_flags) };
    if result != UACPI_STATUS_OK {
        let ptr = unsafe { uacpi_status_to_string(result) };
        let c_str = unsafe { core::ffi::CStr::from_ptr(ptr) };
        let str = c_str.to_str();

        crate::error!("uacpi_initialize: {str:?}");
        return;
    }

    let result = unsafe { uacpi_namespace_load() };
    if result != UACPI_STATUS_OK {
        let ptr = unsafe { uacpi_status_to_string(result) };
        let c_str = unsafe { core::ffi::CStr::from_ptr(ptr) };
        let str = c_str.to_str();

        crate::error!("uacpi_namespace_load: {str:?}");
        return;
    }
}

#[inline(never)]
fn panic_stub(fn_name: &str) -> ! {
    panic!("uACPI stub called: {}", fn_name);
}

// Memory
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_alloc(size: uacpi_size) -> *mut core::ffi::c_void {
    let ptr = allocate(size, size)
        .map(|ptr| ptr.as_ptr().cast())
        .unwrap_or(ptr::null_mut());

    crate::trace!("uacpi_kernel_alloc({size}) -> {ptr:p}");
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free(mem: *mut core::ffi::c_void, size: uacpi_size) {
    let Some(ptr) = NonNull::new(mem.cast::<u8>()) else {
        return;
    };

    crate::trace!("uacpi_kernel_free({mem:p}, {size})");
    unsafe { deallocate(ptr, size, size) }
}

// Timing
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_stall(usec: uacpi_u8) {
    panic_stub("uacpi_kernel_stall")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_sleep(msec: uacpi_u64) {
    panic_stub("uacpi_kernel_sleep")
}

// Thread / Mutex
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_get_thread_id() -> uacpi_thread_id {
    ptr::with_exposed_provenance_mut(1)
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_mutex() -> uacpi_handle {
    let Some(ptr) = allocate(
        mem::size_of::<RawSpinlock>(),
        mem::align_of::<RawSpinlock>(),
    ) else {
        return ptr::null_mut();
    };

    // SAFETY:
    //
    // `ptr` has not escaped this function yet.
    unsafe { ptr.cast::<RawSpinlock>().write(RawSpinlock::new()) }
    ptr.as_ptr().cast::<ffi::c_void>()
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_mutex(mutex: uacpi_handle) {
    panic_stub("uacpi_kernel_free_mutex")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_acquire_mutex(
    mutex: uacpi_handle,
    timeout: uacpi_u16,
) -> uacpi_status {
    crate::trace!("uacpi_kernel_acquire_mutex({mutex:p}, {timeout})");

    let ptr = mutex.cast::<RawSpinlock>();
    let spinlock = unsafe { &*ptr };
    let Ok(()) = spinlock.try_lock() else {
        return UACPI_STATUS_TIMEOUT;
    };

    UACPI_STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_release_mutex(mutex: uacpi_handle) {
    crate::trace!("uacpi_kernel_release_mutex({mutex:p}");

    let ptr = mutex.cast::<RawSpinlock>();
    let spinlock = unsafe { &*ptr };
    spinlock.unlock()
}

// Event
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_event() -> uacpi_handle {
    panic_stub("uacpi_kernel_create_event")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_event(event: uacpi_handle) {
    panic_stub("uacpi_kernel_free_event")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_wait_for_event(
    event: uacpi_handle,
    timeout: uacpi_u16,
) -> uacpi_bool {
    panic_stub("uacpi_kernel_wait_for_event")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_signal_event(event: uacpi_handle) {
    panic_stub("uacpi_kernel_signal_event")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_reset_event(event: uacpi_handle) {
    panic_stub("uacpi_kernel_reset_event")
}

// Spinlocks
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_create_spinlock() -> uacpi_handle {
    let Some(ptr) = allocate(
        mem::size_of::<RawSpinlock>(),
        mem::align_of::<RawSpinlock>(),
    ) else {
        return ptr::null_mut();
    };

    // SAFETY:
    //
    // `ptr` has not escaped this function yet.
    unsafe { ptr.cast::<RawSpinlock>().write(RawSpinlock::new()) }
    ptr.as_ptr().cast::<ffi::c_void>()
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_free_spinlock(spinlock: uacpi_handle) {
    panic_stub("uacpi_kernel_free_spinlock")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_lock_spinlock(spinlock: uacpi_handle) -> uacpi_cpu_flags {
    crate::trace!("uacpi_kernel_lock_spinlock({spinlock:p})");

    let ptr = spinlock.cast::<RawSpinlock>();
    let spinlock = unsafe { &*ptr };
    spinlock.lock();

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_unlock_spinlock(spinlock: uacpi_handle, flags: uacpi_cpu_flags) {
    crate::trace!("uacpi_kernel_unlock_spinlock({spinlock:p})");

    let ptr = spinlock.cast::<RawSpinlock>();
    let spinlock = unsafe { &*ptr };
    spinlock.unlock();
}

// Work / Scheduler
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_schedule_work(
    work_type: uacpi_work_type,
    handler: uacpi_work_handler,
    ctx: uacpi_handle,
) -> uacpi_status {
    panic_stub("uacpi_kernel_schedule_work")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_wait_for_work_completion() -> uacpi_status {
    panic_stub("uacpi_kernel_wait_for_work_completion")
}

// PCI / IO (minimal stubs)
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_device_open(
    address: uacpi_pci_address,
    out_handle: *mut uacpi_handle,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_device_open")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_device_close(handle: uacpi_handle) {
    panic_stub("uacpi_kernel_pci_device_close")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_read8(
    device: uacpi_handle,
    offset: uacpi_size,
    out_value: *mut uacpi_u8,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_read8")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_read16(
    device: uacpi_handle,
    offset: uacpi_size,
    out_value: *mut uacpi_u16,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_read16")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_read32(
    device: uacpi_handle,
    offset: uacpi_size,
    out_value: *mut uacpi_u32,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_read32")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write8(
    device: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u8,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_write8")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write16(
    device: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u16,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_write16")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_pci_write32(
    device: uacpi_handle,
    offset: uacpi_size,
    value: uacpi_u32,
) -> uacpi_status {
    panic_stub("uacpi_kernel_pci_write32")
}

// Logging
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_log(level: uacpi_log_level, msg: *const uacpi_char) {
    let c_str = unsafe { core::ffi::CStr::from_ptr(msg) };
    let str = c_str.to_str();
    if let Ok(str) = str {
        match level {
            UACPI_LOG_TRACE => crate::trace!("{str}"),
            UACPI_LOG_DEBUG => crate::debug!("{str}"),
            UACPI_LOG_INFO => crate::info!("{str}"),
            UACPI_LOG_WARN => crate::warn!("{str}"),
            UACPI_LOG_ERROR => crate::error!("{str}"),
            _ => panic_stub("uacpi_kernel_log called with invalid uacpi_log_level"),
        }
    } else {
        match level {
            UACPI_LOG_TRACE => crate::trace!("{c_str:?}"),
            UACPI_LOG_DEBUG => crate::debug!("{c_str:?}"),
            UACPI_LOG_INFO => crate::info!("{c_str:?}"),
            UACPI_LOG_WARN => crate::warn!("{c_str:?}"),
            UACPI_LOG_ERROR => crate::error!("{c_str:?}"),
            _ => panic_stub("uacpi_kernel_log called with invalid uacpi_log_level"),
        }
    }
}

// I/O mapping
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_map(
    phys: uacpi_phys_addr,
    length: uacpi_size,
) -> *mut core::ffi::c_void {
    crate::debug!("uacpi_kernel_iomap({phys:#x}, {length:#x})");

    /*

    let start_address = PhysicalAddress::new(phys);
    let end_address = start_address.add(usize_to_u64(length));
    let range = FrameRange::from_addresses(start_address, end_address);

    let offset = start_address.frame_offset();
    let Ok(page_range) =
        crate::memory::virt::map_device(range, crate::memory::virt::Permissions::ReadWrite)
    else {
        return ptr::null_mut();
    };

    let raw_address = page_range
        .start()
        .start_address()
        .add(u64_to_usize_panicking(offset))
        .value();
    ptr::without_provenance_mut(raw_address)

    */
    ptr::with_exposed_provenance_mut(u64_to_usize_panicking(phys))
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_unmap(addr: *mut core::ffi::c_void, length: uacpi_size) {
    panic_stub("uacpi_kernel_io_unmap")
}

// Memory mapping (general)
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_map(
    addr: uacpi_phys_addr,
    size: uacpi_size,
) -> *mut core::ffi::c_void {
    crate::debug!("uacpi_kernel_map({addr:#x}, {size:#x})");

    let start_address = PhysicalAddress::new(addr);
    let end_address = start_address.add(usize_to_u64(size));
    let range = FrameRange::from_addresses(start_address, end_address);

    let offset = start_address.frame_offset();
    let Ok(page_range) =
        crate::memory::virt::map(range, crate::memory::virt::Permissions::ReadWrite)
    else {
        return ptr::null_mut();
    };

    let raw_address = page_range
        .start()
        .start_address()
        .add(u64_to_usize_panicking(offset))
        .value();
    ptr::without_provenance_mut(raw_address)
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_unmap(addr: *mut core::ffi::c_void, size: uacpi_size) {
    crate::debug!("uacpi_kernel_unmap({addr:p}, {size:#x}");

    let Some(generic_table) = stub_protocol::generic_table() else {
        crate::warn!("failed to unmap {addr:p} of {size:#x}");
        return;
    };

    let addr_usize = addr.addr() & !(u64_to_usize_panicking(generic_table.page_frame_size - 1));
    let result = unsafe {
        (generic_table.unmap)(
            addr_usize,
            size.div_ceil(u64_to_usize_panicking(generic_table.page_frame_size)),
        )
    };
    if result != Status::SUCCESS {
        crate::warn!("failed to unmap {addr:p} of {size:#x}");
    }
}

// I/O port access
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_read8(port: uacpi_u16) -> uacpi_u8 {
    panic_stub("uacpi_kernel_io_read8")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_read16(port: uacpi_u16) -> uacpi_u16 {
    panic_stub("uacpi_kernel_io_read16")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_read32(port: uacpi_u16) -> uacpi_u32 {
    panic_stub("uacpi_kernel_io_read32")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write8(port: uacpi_u16, value: uacpi_u8) {
    panic_stub("uacpi_kernel_io_write8")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write16(port: uacpi_u16, value: uacpi_u16) {
    panic_stub("uacpi_kernel_io_write16")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_io_write32(port: uacpi_u16, value: uacpi_u32) {
    panic_stub("uacpi_kernel_io_write32")
}

// Interrupts
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_uninstall_interrupt_handler(vector: uacpi_u32) {
    panic_stub("uacpi_kernel_uninstall_interrupt_handler")
}

// ACPI tables
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_get_rsdp(out_rsdp_address: *mut uacpi_phys_addr) -> uacpi_status {
    crate::trace!("uacpi_kernel_get_rsdp({out_rsdp_address:p})");

    let Some(arch_table) = stub_protocol::arch_table() else {
        return UACPI_STATUS_NOT_FOUND;
    };

    let rsdp_value = if arch_table.xsdp != 0 {
        arch_table.xsdp
    } else {
        arch_table.rsdp
    };

    unsafe { *out_rsdp_address = rsdp_value }
    UACPI_STATUS_OK
}

// Time
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_get_nanoseconds_since_boot() -> uacpi_u64 {
    use core::sync::atomic::{AtomicU64, Ordering};

    static A: AtomicU64 = AtomicU64::new(0);
    A.fetch_add(1, Ordering::Relaxed)
}

// Firmware / platform hooks
#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_handle_firmware_request(
    request_type: uacpi_u32,
    buffer: *mut uacpi_u8,
    length: uacpi_size,
) -> uacpi_status {
    panic_stub("uacpi_kernel_handle_firmware_request")
}

#[unsafe(no_mangle)]
pub extern "C" fn uacpi_kernel_install_interrupt_handler() {
    panic_stub("uacpi_kernel_install_interrupt_handler")
}

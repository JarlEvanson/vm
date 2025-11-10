//! Support for booting from a UEFI environment.

use core::{
    fmt::{self, Write},
    ptr, slice,
    sync::atomic::{AtomicPtr, Ordering},
};

use uefi::{
    data_type::{Guid, Handle, Status},
    memory::MemoryType,
    table::{boot::AllocateType, config, system::SystemTable},
};

use crate::{
    boot::{
        Context,
        context::{AllocationPolicy, ContextImpl, FailedMapping, NotFound, OutOfMemory},
    },
    stub_main,
};

#[cfg(target_arch = "aarch64")]
core::arch::global_asm! {
    ".global efi_main",
    "efi_main:",

    "stp x29, x30, [sp, #-16]",
    "stp x0, x1, [sp, #-32]",
    "sub sp, sp, #32",

    "bl relocate",
    "cmp x0, #0",

    "add sp, sp, #32",
    "ldp x0, x1, [sp, #-32]",
    "ldp x29, x30, [sp, #-16]",

    // If `relocate()` was a success, jump to `uefi_main()`.
    "b.eq {uefi_main}",
    // Otherwise, return with x0 = 0x8000000000000001 (LOAD_ERROR).
    "mov x0, #1",
    "orr x0, x0, #0x8000000000000000",
    "ret",

    uefi_main = sym uefi_main,
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm! {
    ".global efi_main",
    "efi_main:",

    "push rcx",
    "push rdx",

    "call relocate",
    "cmp rax, 0",

    "pop rdx",
    "pop rcx",

    // If `relocate()` was a success, jump to `uefi_main()`.
    "je {uefi_main}",
    // Otherwise, return with rax = 0x8000000000000001 (LOAD_ERROR).
    "mov rax, 0x8000000000000001",
    "ret",

    uefi_main = sym uefi_main,
}

static SYSTEM_TABLE_PTR: AtomicPtr<SystemTable> = AtomicPtr::new(ptr::null_mut());

extern "efiapi" fn uefi_main(image_handle: Handle, system_table_ptr: *mut SystemTable) -> Status {
    SYSTEM_TABLE_PTR.store(system_table_ptr, Ordering::Relaxed);
    *crate::PANIC_FUNC.lock() = panic_handler;

    let mut context = UefiContext { system_table_ptr };
    let mut context = Context::new(&mut context);

    stub_main(&mut context);

    Status::LOAD_ERROR
}

/// Implementation of [`ContextImpl`] for UEFI.
pub struct UefiContext {
    /// A pointer to the UEFI system table for executable.
    system_table_ptr: *mut SystemTable,
}

impl ContextImpl for UefiContext {
    fn physical_bits(&self) -> u8 {
        64
    }

    fn frame_size(&self) -> u64 {
        4096
    }

    fn allocate_frames(&mut self, kind: AllocationPolicy, count: u64) -> Result<u64, OutOfMemory> {
        let (allocation_type, mut physical_address) = match kind {
            AllocationPolicy::Any => (AllocateType::ANY_PAGES, 0),
            AllocationPolicy::At(address) => (AllocateType::ADDRESS, address),
            AllocationPolicy::Below(value) => (AllocateType::MAX_ADDRESS, value),
        };

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { (*self.system_table_ptr).boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `allocate_pages` function pointer.
        let allocate_pages_ptr = unsafe { (*boot_services_ptr).allocate_pages };

        let count = usize::try_from(count).map_err(|_| OutOfMemory)?;

        // SAFETY:
        //
        // `free_pages_ptr` came from a valid [`BootServices`] table and its arguments are
        // correct according to the UEFI specification.
        let status = unsafe {
            allocate_pages_ptr(
                allocation_type,
                MemoryType::LOADER_CODE,
                count,
                &mut physical_address,
            )
        };
        if status == Status::SUCCESS {
            Ok(physical_address)
        } else if status == Status::OUT_OF_RESOURCES {
            Err(OutOfMemory)
        } else {
            panic!("error allocating frame region of size {count}: {status:?}")
        }
    }

    unsafe fn deallocate_frames(&mut self, physical_address: u64, count: u64) {
        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let boot_services_ptr = unsafe { (*self.system_table_ptr).boot_services };
        // SAFETY:
        //
        // `boot_services_ptr` must point to a valid [`BootServices`] table and that must contain a
        // `free_pages` function pointer.
        let free_pages_ptr = unsafe { (*boot_services_ptr).free_pages };

        let mut base_address = physical_address;
        let mut remaining = count;
        while remaining != 0 {
            let iter_count = remaining.min(usize::MAX as u64) as usize;

            // SAFETY:
            //
            // `free_pages_ptr` came from a valid [`BootServices`] table and its arguments are
            // correct according to the UEFI specification.
            let status = unsafe { free_pages_ptr(physical_address, iter_count) };
            if status.error() {
                let _ = writeln!(
                    self,
                    "error deallocating frame region {:032X}-{:032X}",
                    physical_address,
                    physical_address
                        .wrapping_add(count * self.frame_size())
                        .wrapping_sub(1)
                );
            }

            base_address = base_address.wrapping_add(iter_count as u64 * self.frame_size());
            remaining -= iter_count as u64;
        }
    }

    fn page_size(&self) -> usize {
        4096
    }

    fn map_frames(&mut self, physical_address: u64, _: usize) -> Result<*mut (), FailedMapping> {
        if physical_address == 0 || physical_address > usize::MAX as u64 {
            return Err(FailedMapping);
        }

        Ok(core::ptr::with_exposed_provenance_mut(
            physical_address as usize,
        ))
    }

    fn map_frames_identity(
        &mut self,
        physical_address: u64,
        _: usize,
    ) -> Result<*mut (), FailedMapping> {
        if physical_address == 0 || physical_address > usize::MAX as u64 {
            return Err(FailedMapping);
        }

        Ok(core::ptr::with_exposed_provenance_mut(
            physical_address as usize,
        ))
    }

    unsafe fn unmap_frames(&mut self, _: *mut (), _: usize) {}

    fn device_tree(&mut self) -> Result<*mut u8, NotFound> {
        lookup_config_table(self.system_table_ptr, config::DEVICE_TREE).map(|ptr| ptr.cast::<u8>())
    }

    fn acpi_rsdp(&mut self) -> Result<*mut u8, NotFound> {
        lookup_config_table(self.system_table_ptr, config::ACPI).map(|ptr| ptr.cast::<u8>())
    }

    fn acpi_xsdp(&mut self) -> Result<*mut u8, NotFound> {
        lookup_config_table(self.system_table_ptr, config::ACPI_2).map(|ptr| ptr.cast::<u8>())
    }
}

impl Write for UefiContext {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        const BUFFER_SIZE: usize = 128;

        // SAFETY:
        //
        // `system_table_ptr` was provided by the `efi_main` entry point.
        let con_out = unsafe { (*self.system_table_ptr).con_out };
        if con_out.is_null() {
            return Err(fmt::Error);
        }

        // SAFETY:
        //
        // `con_out` should be a valid SimpleTextOutputProcotol structure, which is guaranteed to
        // contain the `output_string` function.
        let output_string_func = unsafe { (*con_out).output_string };

        let mut buffer = [0u16; BUFFER_SIZE + 1];
        let mut index = 0;

        let mut chars = s.chars();
        let mut next_char = chars.next();

        let mut newline_processed = false;
        while let Some(mut c) = next_char.take() {
            if c == '\n' && !newline_processed {
                newline_processed = true;

                next_char = Some(c);
                c = '\r';
            } else {
                newline_processed = false;
            }

            if c.len_utf16() != 1 {
                // Character is unrepresentable in UCS-2, replace with replacement character.
                c = '\u{FFFD}';
            }

            buffer[index] = c as u16;
            index += 1;

            if index == BUFFER_SIZE {
                let string = &mut buffer[..=index];
                string[index] = 0;

                // Ignore any warnings/errors (we can't fix them and logging them could cause a
                // stack overflow).
                //
                // SAFETY:
                //
                // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
                // pointer, which means it is safe to be called.
                let _ = unsafe { output_string_func(con_out, string.as_mut_ptr()) };
                index = 0;
            }

            if next_char.is_none() {
                next_char = chars.next();
            }
        }

        if index != 0 {
            let string = &mut buffer[..=index];
            string[index] = 0;

            // Ignore any warnings/errors (we can't fix them and logging them could cause a
            // stack overflow).
            //
            // SAFETY:
            //
            // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
            // pointer, which means it is safe to be called.
            let _ = unsafe { output_string_func(con_out, string.as_mut_ptr()) };
        }

        Ok(())
    }
}

/// Iterates though the UEFI Configuration Tables and returns the first table entry with the given
/// [`Guid`].
fn lookup_config_table(
    system_table_ptr: *mut SystemTable,
    guid: Guid,
) -> Result<*mut (), NotFound> {
    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let config_table_count = unsafe { (*system_table_ptr).number_of_table_entries };
    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let config_tables_ptr = unsafe { (*system_table_ptr).configuration_table };

    // SAFETY:
    //
    // `system_table_ptr` is not NULL and so according to the UEFI specification, the configuration
    // tables should be present.
    let config_tables = unsafe { slice::from_raw_parts(config_tables_ptr, config_table_count) };

    for table in config_tables {
        if table.vendor_guid == guid {
            return Ok(table.vendor_table.cast::<()>());
        }
    }

    Err(NotFound)
}

fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    let system_table_ptr = SYSTEM_TABLE_PTR.load(Ordering::Relaxed);
    let mut context = UefiContext { system_table_ptr };
    let _ = writeln!(context, "{info}");

    loop {}
}

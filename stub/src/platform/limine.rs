//! Support for booting from the Limine boot protocol.

use core::{
    fmt::{self, Write},
    ptr::{self, NonNull},
    slice,
};

use limine::{
    BASE_REVISION, BASE_REVISION_MAGIC_0, BASE_REVISION_MAGIC_1, BaseRevisionTag,
    device_tree::{DEVICE_TREE_REQUEST_MAGIC, DeviceTreeRequest},
    efi_sys_table::{EFI_SYSTEM_TABLE_REQUEST_MAGIC, EfiSystemTableRequest},
    executable_addr::{EXECUTABLE_ADDRESS_REQUEST_MAGIC, ExecutableAddressRequest},
    framebuffer::{FRAMEBUFFER_REQUEST_MAGIC, FramebufferRequest, FramebufferV0},
    hhdm::{HHDM_REQUEST_MAGIC, HhdmRequest},
    memory_map::{MEMORY_MAP_REQUEST_MAGIC, MemoryMapEntry, MemoryMapRequest, MemoryType},
    rsdp::{RSDP_REQUEST_MAGIC, RsdpRequest},
    smbios::{SMBIOS_REQUEST_MAGIC, SmbiosRequest},
};
use stub_api::{MemoryDescriptor, Status, TakeoverFlags};
use sync::{ControlledModificationCell, Spinlock};

use crate::{
    arch::{
        AddressSpaceImpl,
        generic::address_space::{AddressSpace, ProtectionFlags},
    },
    platform::{
        AllocationPolicy, BufferTooSmall, MemoryMap, OutOfMemory, Platform, allocator,
        frame_allocator, frame_size,
        graphics::{
            console::Console,
            font::{FONT_MAP, GLYPH_ARRAY},
            surface::{OutOfBoundsError, Point, Region, Surface, region_in_bounds},
        },
        platform_initialize, read_u64_at, write_u64_at,
    },
    stub_main,
    util::{u64_to_usize, usize_to_u64},
};

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

/// Request for the framebuffers of the program.
#[used]
#[unsafe(link_section = ".limine.requests")]
static FRAMEBUFFER_REQUEST: ControlledModificationCell<FramebufferRequest> =
    ControlledModificationCell::new(FramebufferRequest {
        id: FRAMEBUFFER_REQUEST_MAGIC,
        revision: 0,
        response: ptr::null_mut(),
    });

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: [u64; 2] = limine::REQUESTS_END_MARKER;

static FRAMEBUFFER: Spinlock<Option<Console<LimineSurface>>> = Spinlock::new(None);
static ADDRESS_IMPL: Spinlock<Option<AddressSpaceImpl>> = Spinlock::new(None);

/// Entry point for Rust when booted using the Limine boot protocol.
pub extern "C" fn limine_main(_: u64) -> ! {
    *crate::PANIC_FUNC.lock() = panic_handler;
    let (memory_map_entries, _, _, _) = validate_required_tables();

    let framebuffer_response = FRAMEBUFFER_REQUEST.get().response;

    // SAFETY:
    //
    // The framebuffer response can be read and should not change if it is not NULL.
    if let Some(framebuffer_response) = unsafe { framebuffer_response.as_ref() } {
        // SAFETY:
        //
        // The Limine protocol specification specifies that this operation must be valid.
        let framebuffers = unsafe {
            slice::from_raw_parts(
                framebuffer_response.framebuffers.cast::<&FramebufferV0>(),
                framebuffer_response.framebuffer_count as usize,
            )
        };

        *FRAMEBUFFER.lock() = unsafe {
            framebuffers
                .first()
                .and_then(|framebuffer| LimineSurface::new(*framebuffer))
                .map(|surface| Console::new(surface, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0x00))
        };
    }

    unsafe { platform_initialize(&Limine) };
    frame_allocator::initialize(memory_map_entries.iter().map(|entry| {
        let start = entry.base;
        let end = entry.base.strict_add(entry.length);

        let (start, end) = if entry.mem_type == MemoryType::USABLE {
            (
                start.next_multiple_of(frame_size()),
                (end / frame_size()) * frame_size(),
            )
        } else {
            (
                (start / frame_size()) * frame_size(),
                end.next_multiple_of(frame_size()),
            )
        };

        let region_type = match entry.mem_type {
            MemoryType::RESERVED => stub_api::MemoryType::RESERVED,
            MemoryType::USABLE => stub_api::MemoryType::FREE,
            MemoryType::BOOTLOADER_RECLAIMABLE => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
            MemoryType::EXECUTABLE_AND_MODULES => stub_api::MemoryType::BOOTLOADER_RECLAIMABLE,
            MemoryType::BAD_MEMORY => stub_api::MemoryType::BAD,
            MemoryType::ACPI_RECLAIMABLE => stub_api::MemoryType::ACPI_RECLAIMABLE,
            MemoryType::ACPI_TABLES => stub_api::MemoryType::ACPI_RECLAIMABLE,
            MemoryType::ACPI_NVS => stub_api::MemoryType::ACPI_NON_VOLATILE,
            _ => stub_api::MemoryType::RESERVED,
        };
        MemoryDescriptor {
            start,
            count: (end - start) / frame_size(),
            region_type,
        }
    }));
    unsafe { *allocator::MAP.get_mut() = Some(map) };
    unsafe { *allocator::UNMAP.get_mut() = Some(unmap) };

    let mut address_impl = ADDRESS_IMPL.lock();

    #[cfg(target_arch = "x86_64")]
    unsafe {
        *address_impl = Some(AddressSpaceImpl::active_current(read_u64_at, write_u64_at))
    };

    drop(address_impl);

    crate::debug!("{:x}", crate::util::image_start());
    match stub_main() {
        Ok(()) => {}
        Err(error) => crate::error!("error loading from Limine: {error}"),
    }

    loop {
        core::hint::spin_loop()
    }
}

/// Implementation of [`Platform`] for Limine.
pub struct Limine;

impl Platform for Limine {
    fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
        allocator::allocate(size, alignment)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, size: usize, alignment: usize) {
        unsafe { allocator::deallocate(ptr, size, alignment) }
    }

    fn frame_size(&self) -> u64 {
        4096
    }

    fn allocate_frames(&self, count: u64, policy: AllocationPolicy) -> Result<u64, OutOfMemory> {
        frame_allocator::allocate_frames(count, policy)
    }

    unsafe fn deallocate_frames(&self, physical_address: u64, count: u64) {
        unsafe { frame_allocator::deallocate_frames(physical_address, count) }
    }

    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [MemoryDescriptor],
    ) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
        frame_allocator::memory_map(buffer)
    }

    fn page_size(&self) -> usize {
        4096
    }

    fn map_temporary(&self, physical_address: u64) -> *mut u8 {
        let hhdm_response_ptr = HHDM_REQUEST.get().response;
        let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
            panic!("Limine higher half direct map was not provided");
        };
        let hhdm_offset = hhdm_response.offset;

        for entry in memory_map_entries() {
            match entry.mem_type {
                MemoryType::USABLE
                | MemoryType::BOOTLOADER_RECLAIMABLE
                | MemoryType::EXECUTABLE_AND_MODULES
                | MemoryType::FRAMEBUFFER
                | MemoryType::ACPI_TABLES
                | MemoryType::ACPI_RECLAIMABLE
                | MemoryType::ACPI_NVS => {}
                _ => continue,
            }

            let entry_start = entry.base;
            let entry_end = entry_start.strict_add(entry.length);
            if entry_start <= physical_address && physical_address < entry_end {
                return physical_address.strict_add(hhdm_offset) as *mut u8;
            }
        }

        todo!("implement arbitary memory mapping")
    }

    fn map_identity(&self, physical_address: u64) -> *mut u8 {
        let mut lock = ADDRESS_IMPL.lock();
        let Some(address_impl) = lock.as_mut() else {
            unreachable!("ADDRESS_IMPL was not initialized");
        };

        address_impl
            .map(
                physical_address,
                physical_address,
                1,
                ProtectionFlags::READ | ProtectionFlags::WRITE | ProtectionFlags::EXECUTE,
            )
            .expect("failed to perform mapping");

        u64_to_usize(physical_address) as *mut u8
    }

    fn translate_virtual(&self, virtual_address: usize) -> Option<u64> {
        let virtual_address = usize_to_u64(virtual_address);

        let hhdm_response_ptr = HHDM_REQUEST.get().response;
        let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
            panic!("Limine higher half direct map was not provided");
        };
        let hhdm_offset = hhdm_response.offset;

        for entry in memory_map_entries() {
            match entry.mem_type {
                MemoryType::USABLE
                | MemoryType::BOOTLOADER_RECLAIMABLE
                | MemoryType::EXECUTABLE_AND_MODULES
                | MemoryType::FRAMEBUFFER
                | MemoryType::ACPI_TABLES
                | MemoryType::ACPI_RECLAIMABLE
                | MemoryType::ACPI_NVS => {}
                _ => continue,
            }

            let entry_virtual_start = entry.base.strict_add(hhdm_offset);
            let entry_virtual_end = entry_virtual_start.strict_add(entry.length);
            if entry_virtual_start <= virtual_address && virtual_address < entry_virtual_end {
                return Some(virtual_address.strict_sub(hhdm_offset));
            }
        }

        let executable_address_response_ptr = EXECUTABLE_ADDRESS_REQUEST.get().response;
        let Some(executable_address_response) =
            (unsafe { executable_address_response_ptr.as_ref() })
        else {
            panic!("Limine executable address was not provided");
        };

        if virtual_address >= executable_address_response.virtual_base {
            return Some(
                virtual_address
                    .wrapping_sub(executable_address_response.virtual_base)
                    .wrapping_add(executable_address_response.physical_base),
            );
        }

        let lock = ADDRESS_IMPL.lock();
        let Some(address_impl) = lock.as_ref() else {
            unreachable!("ADDRESS_IMPL was not initialized");
        };

        address_impl.translate_virt(virtual_address).ok()
    }

    fn takeover(&self, key: u64, flags: TakeoverFlags) -> Status {
        todo!("{key:#x} {flags:?}")
    }

    fn print(&self, args: fmt::Arguments) {
        if let Some(console) = FRAMEBUFFER.lock().as_mut() {
            let _ = write!(console, "{args}");
        }
    }

    fn uefi_system_table(&self) -> Option<u64> {
        let uefi_system_table_response_ptr = UEFI_SYSTEM_TABLE_REQUEST.get().response;
        let uefi_system_table_response = unsafe { uefi_system_table_response_ptr.as_ref()? };
        Some(uefi_system_table_response.address)
    }

    fn rsdp(&self) -> Option<u64> {
        let rsdp_response_ptr = RSDP_REQUEST.get().response;
        let rsdp_response = unsafe { rsdp_response_ptr.as_ref()? };
        self.translate_virtual(u64_to_usize(rsdp_response.address))
    }

    fn xsdp(&self) -> Option<u64> {
        let rsdp_response_ptr = RSDP_REQUEST.get().response;
        let rsdp_response = unsafe { rsdp_response_ptr.as_ref()? };
        self.translate_virtual(u64_to_usize(rsdp_response.address))
    }

    fn device_tree(&self) -> Option<u64> {
        let device_tree_response_ptr = DEVICE_TREE_REQUEST.get().response;
        let device_tree_response = unsafe { device_tree_response_ptr.as_ref()? };
        self.translate_virtual(device_tree_response.dtb_ptr.addr())
    }

    fn smbios_32(&self) -> Option<u64> {
        let smbios_response_ptr = SMBIOS_REQUEST.get().response;
        let smbios_response = unsafe { smbios_response_ptr.as_ref()? };
        Some(smbios_response.entry_32)
    }

    fn smbios_64(&self) -> Option<u64> {
        let smbios_response_ptr = SMBIOS_REQUEST.get().response;
        let smbios_response = unsafe { smbios_response_ptr.as_ref()? };
        Some(smbios_response.entry_64)
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
    let Some(memory_map_response) = (unsafe { memory_map_response_ptr.as_ref() }) else {
        panic!("Limine memory map was not provided");
    };

    let memory_map_entries = unsafe {
        slice::from_raw_parts(
            memory_map_response.entries.cast::<&MemoryMapEntry>(),
            u64_to_usize(memory_map_response.entry_count),
        )
    };

    let hhdm_response_ptr = HHDM_REQUEST.get().response;
    let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
        panic!("Limine higher half direct map was not provided");
    };

    let executable_address_response_ptr = EXECUTABLE_ADDRESS_REQUEST.get().response;
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

/// Implementation of [`Surface`] for Limine framebuffers.
struct LimineSurface {
    /// The virtual address of the [`Surface`].
    address: *mut u8,
    /// The width of the [`Surface`] in pixels.
    width: usize,
    /// The height of the [`Surface`] in pixels.
    height: usize,
    /// The number of bytes between the start of one line and the start of an adjacent line.
    pitch: usize,
    /// The number of bits in a pixel.
    bpp: u16,
    /// The number of bits in the red bitmask.
    red_mask_size: u8,
    /// The offset of the bits in the red bitmask.
    red_mask_shift: u8,
    /// The number of bits in the green bitmask.
    green_mask_size: u8,
    /// The offset of the bits in the green bitmask.
    green_mask_shift: u8,
    /// The number of bits in the blue bitmask.
    blue_mask_size: u8,
    /// The offset of the bits in the blue bitmask.
    blue_mask_shift: u8,
}

impl LimineSurface {
    /// Creates a new [`LimineSurface`] as specified by [`FramebufferV0`].
    ///
    /// # Safety
    ///
    /// The produced [`LimineSurface`] must have exclusive access to the underlying region it is
    /// manipulating.
    pub unsafe fn new(framebuffer: &FramebufferV0) -> Option<LimineSurface> {
        let width = usize::try_from(framebuffer.width).ok()?;
        let height = usize::try_from(framebuffer.height).ok()?;
        let pitch = usize::try_from(framebuffer.pitch).ok()?;

        let max_x = width.saturating_sub(1);
        let max_x_bit_offset = max_x.checked_mul(usize::from(framebuffer.bpp))?;

        let max_y = height.saturating_sub(1);
        let max_y_bit_offset = max_y.checked_mul(pitch)?.checked_mul(8)?;
        let _ = max_x_bit_offset.checked_add(max_y_bit_offset)?;

        match framebuffer.bpp {
            8 | 16 | 32 | 64 => {}
            _ => {
                // TODO: support an arbitrary number of bits per pixel
                return None;
            }
        }

        let surface = Self {
            address: framebuffer.address.cast::<u8>(),
            width,
            height,
            pitch,
            bpp: framebuffer.bpp,
            red_mask_size: framebuffer.red_mask_size,
            red_mask_shift: framebuffer.red_mask_shift,
            green_mask_size: framebuffer.green_mask_size,
            green_mask_shift: framebuffer.green_mask_shift,
            blue_mask_size: framebuffer.blue_mask_size,
            blue_mask_shift: framebuffer.blue_mask_shift,
        };

        Some(surface)
    }
}

// SAFETY:
//
// Read and write bounds checking are properly implemented.
unsafe impl Surface for LimineSurface {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    unsafe fn write_pixel_unchecked(&mut self, point: Point, value: u32) {
        let x_bit_offset = point.x * usize::from(self.bpp);
        let y_bit_offset = point.y * self.pitch * 8;
        let bit_offset = x_bit_offset + y_bit_offset;

        let red = convert_from_rgba(value, self.red_mask_size, 0) << self.red_mask_shift;
        let green = convert_from_rgba(value, self.green_mask_size, 1) << self.green_mask_shift;
        let blue = convert_from_rgba(value, self.blue_mask_size, 2) << self.blue_mask_shift;
        let color = red | green | blue;

        let address = self.address.wrapping_byte_add(bit_offset / 8);
        match self.bpp {
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            8 => unsafe { address.write_volatile(color as u8) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            16 => unsafe { address.cast::<u16>().write_volatile(color as u16) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            32 => unsafe { address.cast::<u32>().write_volatile(color as u32) },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            64 => unsafe { address.cast::<u64>().write_volatile(color) },
            _ => todo!("support an arbitrary number of bits per pixel"),
        }
    }

    unsafe fn read_pixel_unchecked(&self, point: Point) -> u32 {
        let x_bit_offset = point.x * usize::from(self.bpp);
        let y_bit_offset = point.y * self.pitch * 8;
        let bit_offset = x_bit_offset + y_bit_offset;

        let address = self.address.wrapping_byte_add(bit_offset / 8);
        let value = match self.bpp {
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            8 => unsafe { address.read_volatile() as u64 },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            16 => unsafe { address.cast::<u16>().read_volatile() as u64 },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            32 => unsafe { address.cast::<u32>().read_volatile() as u64 },
            // SAFETY:
            //
            // `address` is within bounds and is suitable to volatile writes.
            64 => unsafe { address.cast::<u64>().read_volatile() },
            _ => todo!("support an arbitrary number of bits per pixel"),
        };

        let red = convert_to_rgba(value >> self.red_mask_shift, self.red_mask_size, 0);
        let green = convert_to_rgba(value >> self.green_mask_shift, self.green_mask_size, 0);
        let blue = convert_to_rgba(value >> self.blue_mask_shift, self.blue_mask_size, 0);

        red | green | blue
    }

    fn copy_within(&mut self, write: Region, source: Point) -> Result<(), OutOfBoundsError> {
        if !region_in_bounds(write, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        let read = Region {
            point: source,
            width: write.width,
            height: write.height,
        };
        if !region_in_bounds(read, self.width(), self.height()) {
            return Err(OutOfBoundsError);
        }

        assert!(self.bpp >= 8);
        let write_index = write.point.x + write.point.y * self.pitch;
        let read_index = read.point.x + read.point.y * self.pitch;

        let mut write_ptr = self.address.wrapping_byte_add(write_index);
        let mut read_ptr = self.address.wrapping_byte_add(read_index);

        let bytes_per_pixel = usize::from(self.bpp.div_ceil(8));
        for _ in 0..write.height {
            unsafe { core::ptr::copy(read_ptr, write_ptr, write.width.strict_mul(bytes_per_pixel)) }
            write_ptr = write_ptr.wrapping_byte_add(self.pitch);
            read_ptr = read_ptr.wrapping_byte_add(self.pitch);
        }

        Ok(())
    }
}

// SAFETY:
//
// The pointer contained by [`LimineSurface`] does not provide access to thread-local or cpu-local
// memory and thus [`LimineSurface`] is [`Send`].
unsafe impl Send for LimineSurface {}

// SAFETY:
//
// All exposed methods provided by [`LimineSurface`] cannot mutate with an immutable reference and
// thus [`LimineSurface`] is [`Sync`].
unsafe impl Sync for LimineSurface {}

/// Converts a Limine pixel value to its RGBA representation.
const fn convert_to_rgba(value: u64, size: u8, index: u8) -> u32 {
    let max_value_foreign = (1u64 << size) - 1;
    let converted_value_foreign = (value * 255) / max_value_foreign;

    (converted_value_foreign << (index * 8)) as u32
}

/// Converts an RGBA pixel value to its Limine representation.
const fn convert_from_rgba(value: u32, size: u8, index: u8) -> u64 {
    let extracted_value = (value >> (index * 8)) as u8;

    let max_value_foreign = (1u64 << size) - 1;
    (extracted_value as u64 * max_value_foreign) / 255
}

fn memory_map_entries() -> &'static [&'static MemoryMapEntry] {
    let memory_map_response_ptr = MEMORY_MAP_REQUEST.get().response;
    let Some(memory_map_response) = (unsafe { memory_map_response_ptr.as_ref() }) else {
        panic!("Limine memory map was not provided");
    };

    unsafe {
        slice::from_raw_parts(
            memory_map_response.entries.cast::<&MemoryMapEntry>(),
            u64_to_usize(memory_map_response.entry_count),
        )
    }
}

fn map(physical_address: u64, _size: u64) -> Option<NonNull<u8>> {
    let hhdm_response_ptr = HHDM_REQUEST.get().response;
    let Some(hhdm_response) = (unsafe { hhdm_response_ptr.as_ref() }) else {
        panic!("Limine higher half direct map was not provided");
    };
    let hhdm_offset = hhdm_response.offset;

    for entry in memory_map_entries() {
        match entry.mem_type {
            MemoryType::USABLE
            | MemoryType::BOOTLOADER_RECLAIMABLE
            | MemoryType::EXECUTABLE_AND_MODULES
            | MemoryType::FRAMEBUFFER
            | MemoryType::ACPI_TABLES
            | MemoryType::ACPI_RECLAIMABLE
            | MemoryType::ACPI_NVS => {}
            _ => continue,
        }

        let entry_start = entry.base;
        let entry_end = entry_start.strict_add(entry.length);
        if entry_start <= physical_address && physical_address < entry_end {
            return NonNull::new(physical_address.strict_add(hhdm_offset) as *mut u8);
        }
    }

    todo!("implement additional map functions")
}

unsafe fn unmap(_: NonNull<u8>, _: u64) {}

/// The Limine boot protocol-specific panic handler.
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    crate::error!("{info}");

    let framebuffer_response = FRAMEBUFFER_REQUEST.get().response;

    // SAFETY:
    //
    // The framebuffer response can be read and should not change if it is not NULL.
    if let Some(framebuffer_response) = unsafe { framebuffer_response.as_ref() } {
        // SAFETY:
        //
        // The Limine protocol specification specifies that this operation must be valid.
        let framebuffers = unsafe {
            slice::from_raw_parts(
                framebuffer_response.framebuffers.cast::<&FramebufferV0>(),
                framebuffer_response.framebuffer_count as usize,
            )
        };

        for framebuffer in framebuffers.iter().skip(1) {
            // SAFETY:
            //
            // We are panicking: we steal control over the framebuffers and overwrite all data.
            let Some(framebuffer) = (unsafe { LimineSurface::new(framebuffer) }) else {
                continue;
            };

            let mut console = Console::new(framebuffer, GLYPH_ARRAY, FONT_MAP, 0xFF_FF_FF_FF, 0x00);
            let _ = writeln!(console, "{info}");
        }
    }

    loop {
        core::hint::spin_loop()
    }
}

//! Definitions of the interface implementors of a platform are required to implement and an
//! abstraction over those services for use by the rest of the executable.

use core::{
    error, fmt,
    ptr::{self, NonNull},
    slice,
};

use stub_api::{MemoryDescriptor, Status, TakeoverFlags};
use sync::ControlledModificationCell;

mod allocation;
mod frame_allocation;

pub use allocation::{Allocation, allocate, deallocate, deallocate_all};
pub use frame_allocation::{
    FrameAllocation, allocate_frames, allocate_frames_aligned, deallocate_all_frames,
    deallocate_frames, frame_size,
};

use crate::util::{u64_to_usize, usize_to_u64};

/// The [`Platform`] implementation that is currently in use.
static PLATFORM: ControlledModificationCell<Option<&'static dyn Platform>> =
    ControlledModificationCell::new(None);

/// Initializes the [`Platform`] implementation to be used for its services.
///
/// # Safety
///
/// This function must only be called a single time and must return before any function that
/// utilizes [`Platform`] services it called.
pub(in crate::platform) unsafe fn platform_initialize(platform: &'static dyn Platform) {
    // SAFETY:
    //
    // The invariants of this function ensure that the mutable access begins and ends before any
    // immutable or mutable access takes place.
    unsafe { *PLATFORM.get_mut() = Some(platform) }
}

/// Cleans up the [`Platform`] implementation and any associated data.
///
/// This function must only be called a single time and must be called after all calls that utilize
/// [`Platform`] services have returned.
pub(in crate::platform) unsafe fn platform_teardown() {
    // SAFETY:
    //
    // The invariants of this function ensure that the mutable access begins and ends after any
    // immutable or mutable access takes place.
    unsafe { *PLATFORM.get_mut() = None }
}

/// Returns the [`Platform`] implementation that is currently in use.
pub(in crate::platform) fn platform() -> &'static dyn Platform {
    PLATFORM
        .get()
        .expect("platform implementation not initialized")
}

/// Returns the current physical [`MemoryMap`].
///
/// # Errors
///
/// [`BufferTooSmall`] is returned when the provided `buffer` is too small, and contains the
/// required size of the buffer. Any allocations or deallocations may change the memory map.
pub fn memory_map<'buffer>(
    buffer: &'buffer mut [MemoryDescriptor],
) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
    platform().memory_map(buffer)
}

/// Returns the size, in bytes, of a page.
pub fn page_size() -> usize {
    platform().page_size()
}

/// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
/// contained into the stub's virtual address space. Any call to this function will invalidate
/// all previous temporary mappings produced by [`Platform::map_temporary()`].
///
/// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
/// chunk, only 1 byte may be accessible.
fn map_temporary(physical_address: u64) -> *mut u8 {
    platform().map_temporary(physical_address)
}

/// Reads the `u8` located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn read_u8_at(physical_address: u64) -> u8 {
    let mut byte = 0;
    read_bytes_at(physical_address, slice::from_mut(&mut byte));

    byte
}

/// Reads the `u16` located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn read_u16_at(physical_address: u64) -> u16 {
    let mut bytes = [0; 2];
    read_bytes_at(physical_address, &mut bytes);

    u16::from_ne_bytes(bytes)
}

/// Reads the `u32` located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn read_u32_at(physical_address: u64) -> u32 {
    let mut bytes = [0; 4];
    read_bytes_at(physical_address, &mut bytes);

    u32::from_ne_bytes(bytes)
}

/// Reads the `u64` located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn read_u64_at(physical_address: u64) -> u64 {
    let mut bytes = [0; 8];
    read_bytes_at(physical_address, &mut bytes);

    u64::from_ne_bytes(bytes)
}

/// Reads the bytes located at `physical_address` into the provided `bytes` slice.
///
/// This function calls [`map_temporary()`].
pub fn read_bytes_at(mut physical_address: u64, mut bytes: &mut [u8]) {
    let page_size = usize_to_u64(page_size());
    while !bytes.is_empty() {
        // Calculate the number of bytes we will read this iteration.
        let size = u64_to_usize(page_size - physical_address % page_size).min(bytes.len());
        // Map the page we will modify.
        let ptr = map_temporary(physical_address);

        // SAFETY:
        //
        // The region that starts at `ptr` was a valid mapping and `size` represents the
        // minimum of the mapping size or the size of the remaining bytes to be copied.
        let read_slice = unsafe { slice::from_raw_parts(ptr, size) };
        bytes[..size].copy_from_slice(read_slice);

        physical_address = physical_address.wrapping_add(usize_to_u64(size));
        bytes = &mut bytes[size..];
    }
}

/// Writes the provided `u8` into the physical memory located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u8_at(physical_address: u64, value: u8) {
    write_bytes_at(physical_address, slice::from_ref(&value));
}

/// Writes the provided `u16` into the physical memory located at `physical_address`
///
/// This function calls [`map_temporary()`].
pub fn write_u16_at(physical_address: u64, value: u16) {
    write_bytes_at(physical_address, &value.to_ne_bytes());
}

/// Writes the provided `u32` into the physical memory located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u32_at(physical_address: u64, value: u32) {
    write_bytes_at(physical_address, &value.to_ne_bytes());
}

/// Writes the provided `u64` into the physical memory located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn write_u64_at(physical_address: u64, value: u64) {
    write_bytes_at(physical_address, &value.to_ne_bytes());
}

/// Writes the bytes in `bytes` into the physical memory located at `physical_address`.
///
/// This function calls [`map_temporary()`].
pub fn write_bytes_at(mut physical_address: u64, mut bytes: &[u8]) {
    let page_size = usize_to_u64(page_size());
    while !bytes.is_empty() {
        // Calculate the number of bytes we will write this iteration.
        let size = u64_to_usize(page_size - physical_address % page_size).min(bytes.len());
        // Map the page we will modify.
        let ptr = map_temporary(physical_address);

        // SAFETY:
        //
        // The region that starts at `ptr` was a valid mapping and `size` represents the
        // minimum of the mapping size or the size of the bytes that have not yet been written.
        unsafe { ptr::copy(bytes.as_ptr(), ptr, size) }

        physical_address = physical_address.wrapping_add(usize_to_u64(size));
        bytes = &bytes[size..];
    }
}

/// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
/// contained into the stub's virtual address space. Any call to this function will invalidate
/// all previous identity mappings produced by [`Platform::map_identity()`].
///
/// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
/// chunk, only 1 byte may be accessible.
///
/// # Panics
///
/// This function may panic if either the underlying implementation is invalid or the provided
/// functions are invalid.
pub fn map_identity(physical_address: u64) -> *mut u8 {
    let ptr = platform().map_identity(physical_address);
    assert_eq!(
        usize_to_u64(ptr.addr()),
        physical_address,
        "identity map implementation failed"
    );
    ptr
}
/// Returns the physical address associated with the provided `virtual_address` if the
/// translation is valid. Otherwise, return [`None`].
pub fn translate_virtual(virtual_address: usize) -> Option<u64> {
    platform().translate_virtual(virtual_address)
}

/// Executes a takover on behalf of the loaded executable. `key` is utilized to ensure that the
/// memory map that the loaded executable has is accurate.
///
/// On success, the executable becomes the sole controller of the system. This means that the
/// executable is free to directly manipulate the hardware in whatever manner it desires.
pub fn takeover(key: u64, flags: TakeoverFlags) -> stub_api::Status {
    platform().takeover(key, flags)
}

/// Returns the physical address of the UEFI system table, if present.
pub fn uefi_system_table() -> Option<u64> {
    platform().uefi_system_table()
}

/// Returns the physical address of the RSDP structure, if present.
pub fn rsdp() -> Option<u64> {
    platform().rsdp()
}

/// Returns the physical address of the XSDP structure, if present.
pub fn xsdp() -> Option<u64> {
    platform().xsdp()
}

/// Returns the physical address of the device tree structure, if present.
pub fn device_tree() -> Option<u64> {
    platform().device_tree()
}

/// Returns the physical address of the SMBIO 32 Entry Point, if present.
pub fn smbios_32() -> Option<u64> {
    platform().smbios_32()
}

/// Returns the physical address of the SMBIO 64 Entry Point, if present.
pub fn smbios_64() -> Option<u64> {
    platform().smbios_64()
}

/// The underlying printer.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    platform().print(args)
}

/// Prints to the platform-specific output.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::platform::_print(format_args!($($arg)*)));
}

/// Prints to the platform-specific output, with a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    if level >= LogLevel::Trace {
        match level {
            LogLevel::Trace => println!("TRACE: {args}"),
            LogLevel::Debug => println!("DEBUG: {args}"),
            LogLevel::Info => println!("INFO : {args}"),
            LogLevel::Warn => println!("WARN : {args}"),
            LogLevel::Error => println!("ERROR: {args}"),
        }
    }
}

/// Logs a message with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::platform::_log(
        crate::platform::LogLevel::Trace,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::platform::_log(
        crate::platform::LogLevel::Debug,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::platform::_log(
        crate::platform::LogLevel::Info,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::platform::_log(
        crate::platform::LogLevel::Warn,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Error`].
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::platform::_log(
        crate::platform::LogLevel::Error,
        format_args!($($arg)*))
    );
}

/// Collection of various services provided by a platform-specific implementation.
pub(in crate::platform) trait Platform: Send + Sync {
    /// Allocates a region of memory of `size` bytes aligned to `alignment`.
    fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<u8>>;

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The `size` and `alignment` parameters must match the parameters utilized when this region
    /// of memory was allocated. `ptr` must describe a block of memory currently allocated via a
    /// call to [`Platform::allocate()`].
    unsafe fn deallocate(&self, ptr: NonNull<u8>, size: usize, alignment: usize);

    /// Returns the size, in bytes, of a frame.
    fn frame_size(&self) -> u64;

    /// Allocates a region of `count` frames in accordance with the provided [`AllocationPolicy`].
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
    /// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
    fn allocate_frames(&self, count: u64, policy: AllocationPolicy) -> Result<u64, OutOfMemory>;

    /// Allocates a region of `count` frames with a starting physical address being a multiple of
    /// `alignment` and the entire region in accordance with the provided [`AllocationPolicy`].
    ///
    /// # Errors
    ///
    /// Returns [`OutOfMemory`] if the system cannot allocate the requested frames. This may not
    /// indicate memory exhaustion if [`AllocationPolicy::Any`] is not in use.
    fn allocate_frames_aligned(
        &self,
        count: u64,
        alignment: u64,
        policy: AllocationPolicy,
    ) -> Result<u64, OutOfMemory> {
        assert!(alignment.is_power_of_two());

        if self.frame_size() >= alignment {
            let address = self.allocate_frames(count, policy)?;

            assert!(address.is_multiple_of(alignment));
            Ok(address)
        } else {
            let total_count = alignment
                .div_ceil(self.frame_size())
                .checked_add(count)
                .ok_or(OutOfMemory)?;

            let address = self.allocate_frames(total_count, policy)?;
            let end_address = address.wrapping_add(total_count.wrapping_mul(self.frame_size()));

            let requested_region_start = address.next_multiple_of(alignment);
            let requested_region_end =
                requested_region_start.wrapping_add(count.wrapping_mul(self.frame_size()));

            let lower_size = requested_region_start.wrapping_sub(address);
            let upper_size = end_address.wrapping_sub(requested_region_end);

            if lower_size != 0 {
                let lower_count = lower_size / self.frame_size();
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(address, lower_count) };
            }

            if upper_size != 0 {
                let upper_count = upper_size / self.frame_size();
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(requested_region_end, upper_count) };
            }

            Ok(requested_region_start)
        }
    }

    /// Deallocates a region of `count` frames with the starting `physical_address`.
    ///
    /// # Safety
    ///
    /// These frames must have been allocated by a call to [`Platform::allocate_frames()`].
    unsafe fn deallocate_frames(&self, physical_address: u64, count: u64);

    /// Returns the current physical [`MemoryMap`].
    ///
    /// # Errors
    ///
    /// [`BufferTooSmall`] is returned when the provided `buffer` is too small, and contains the
    /// required size of the buffer. Any allocations or deallocations may change the memory map.
    fn memory_map<'buffer>(
        &self,
        buffer: &'buffer mut [MemoryDescriptor],
    ) -> Result<MemoryMap<'buffer>, BufferTooSmall>;

    /// Returns the size, in bytes, of a page.
    fn page_size(&self) -> usize;

    /// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
    /// contained into the stub's virtual address space. Any call to this function will invalidate
    /// all previous temporary mappings produced by [`Platform::map_temporary()`].
    ///
    /// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
    /// chunk, only 1 byte may be accessible.
    fn map_temporary(&self, physical_address: u64) -> *mut u8;

    /// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
    /// contained into the stub's virtual address space. Any call to this function will invalidate
    /// all previous identity mappings produced by [`Platform::map_identity()`].
    ///
    /// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
    /// chunk, only 1 byte may be accessible.
    fn map_identity(&self, physical_address: u64) -> *mut u8;

    /// Returns the physical address associated with the provided `virtual_address` if the
    /// translation is valid. Otherwise, return [`None`].
    fn translate_virtual(&self, virtual_address: usize) -> Option<u64>;

    /// Executes a takover on behalf of the loaded executable. `key` is utilized to ensure that the
    /// memory map that the loaded executable has is accurate.
    ///
    /// On success, the executable becomes the sole controller of the system. This means that the
    /// executable is free to directly manipulate the hardware in whatever manner it desires.
    fn takeover(&self, key: u64, flags: TakeoverFlags) -> Status;

    /// Prints the provided [`Arguments`][fmt::Arguments].
    fn print(&self, args: fmt::Arguments);

    /// Returns the physical address of the UEFI system table, if present.
    fn uefi_system_table(&self) -> Option<u64>;

    /// Returns the physical address of the RSDP structure, if present.
    fn rsdp(&self) -> Option<u64>;

    /// Returns the physical address of the XSDP structure, if present.
    fn xsdp(&self) -> Option<u64>;

    /// Returns the physical address of the device tree structure, if present.
    fn device_tree(&self) -> Option<u64>;

    /// Returns the physical address of the SMBIO 32 Entry Point, if present.
    fn smbios_32(&self) -> Option<u64>;

    /// Returns the physical address of the SMBIO 64 Entry Point, if present.
    fn smbios_64(&self) -> Option<u64>;
}

/// Structure controlling the behavior of [`Platform::allocate_frames()`].
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocationPolicy {
    /// Any frame region is suitable for allocation.
    #[default]
    Any,
    /// Only the frame region with the starting physical address that matches the associated
    /// physical address is valid for allocated.
    At(u64),
    /// The entire frame region must be below the associated physical address.
    Below(u64),
}

impl fmt::Debug for AllocationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.pad("Any"),
            Self::At(value) => write!(f, "At({value:#x})"),
            Self::Below(value) => write!(f, "Below({value:#x})"),
        }
    }
}

/// A physical [`MemoryMap`], containing the current layout and designation of physical memory.
///
/// This may be invalided by any allocation or deallocation.
pub struct MemoryMap<'buffer> {
    /// The buffer containing the [`MemoryDescriptor`]s.
    buffer: &'buffer mut [MemoryDescriptor],
    /// The key associated with said descriptors.
    key: u64,
}

impl<'buffer> MemoryMap<'buffer> {
    /// Returns a newly created [`MemoryMap`] with the provided [`MemoryDescriptor`]s.
    pub const fn new(buffer: &'buffer mut [MemoryDescriptor], key: u64) -> Self {
        Self { buffer, key }
    }

    /// Returns the [`MemoryDescriptor`]s associated with this [`MemoryMap`].
    pub const fn descriptors(&self) -> &[MemoryDescriptor] {
        self.buffer
    }

    /// Returns the unique value that identifies this [`MemoryMap`].
    pub const fn key(&self) -> u64 {
        self.key
    }
}

/// Indicates that there were no frame regions that were free and complied with the provided flags.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("out of memory")
    }
}

impl error::Error for OutOfMemory {}

/// Indicates that the provided buffer was too small to contain the memory map.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferTooSmall {
    /// The required number of entries in the buffer.
    pub required_count: usize,
}

impl fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("buffer too small")
    }
}

impl error::Error for BufferTooSmall {}

/// Various levels to determine the priority of information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Designates very low priority information.
    Trace,
    /// Designates lower priority information.
    Debug,
    /// Designates informatory logs.
    Info,
    /// Designates hazardous logs.
    Warn,
    /// Designates very serious logs.
    Error,
}

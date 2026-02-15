//! Definitions providing the interface platform implementors provide and an abstraction utilizing
//! that interface to implement services for use by the rest of the executable.

use core::{
    error, fmt,
    ptr::{self, NonNull},
    slice,
};

use conversion::{u64_to_usize_strict, usize_to_u64};
use memory::{
    address::{
        AddressSpaceDescriptor, FrameRange, PhysicalAddress, PhysicalAddressRange, VirtualAddress,
    },
    phys::PhysicalMemorySpace,
};
use stub_api::{MemoryDescriptor, Status, TakeoverFlags};
use sync::ControlledModificationCell;

mod allocation;
mod frame_allocation;

pub use allocation::*;
pub use frame_allocation::*;

/// The [`Platform`] implementation that is currently in use.
static PLATFORM: ControlledModificationCell<Option<&'static dyn Platform>> =
    ControlledModificationCell::new(None);

/// Initializes the [`Platform`] implementation to be used for its services.
///
/// # Safety
///
/// This function must be called before any function that requires [`Platform`] services is called.
pub(in crate::platform) unsafe fn platform_initialize(platform: &'static dyn Platform) {
    // SAFETY:
    //
    // The invariants of this function ensure that the mutable access begins and ends before any
    // immutable or mutable access takes place.
    unsafe { *PLATFORM.get_mut() = Some(platform) }
}

/// Cleans up the [`Platform`] implementation and any associated data.
///
/// # Safety
///
/// Any function that requires [`Platform`] services must be called before this function.
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

/// Returns the size, in bytes, of a [`Page`][memory::address::Page].
pub fn page_size() -> usize {
    platform().page_size()
}

/// Maps [`Platform::page_size()`]ed physical memory region inside which `physical_address` is
/// contained into the stub's virtual address space. Any call to this function will invalidate
/// all previous temporary mappings produced by [`Platform::map_temporary()`].
///
/// This means that if `physical_address` is 1 byte from the top of a [`Platform::page_size()`]
/// chunk, only 1 byte may be accessible.
fn map_temporary(address: PhysicalAddress) -> *mut u8 {
    platform().map_temporary(address)
}

/// Implementation of [`PhysicalMemorySpace`] utilizing [`map_temporary()`].
pub struct StubPhysicalMemory;

// SAFETY:
//
// The provided primitives are combined correctly.
unsafe impl PhysicalMemorySpace for StubPhysicalMemory {
    type Error = core::convert::Infallible;

    fn descriptor(&self) -> AddressSpaceDescriptor {
        memory::address::AddressSpaceDescriptor::new(64, false)
    }

    unsafe fn read_u8(&self, address: PhysicalAddress) -> Result<u8, Self::Error> {
        let mut byte = 0;
        read_bytes_at(address, slice::from_mut(&mut byte));

        Ok(byte)
    }

    unsafe fn read_u16_le(&self, address: PhysicalAddress) -> Result<u16, Self::Error> {
        let mut bytes = [0; 2];
        read_bytes_at(address, &mut bytes);

        Ok(u16::from_le_bytes(bytes))
    }

    unsafe fn read_u16_be(&self, address: PhysicalAddress) -> Result<u16, Self::Error> {
        let mut bytes = [0; 2];
        read_bytes_at(address, &mut bytes);

        Ok(u16::from_be_bytes(bytes))
    }

    unsafe fn read_u32_le(&self, address: PhysicalAddress) -> Result<u32, Self::Error> {
        let mut bytes = [0; 4];
        read_bytes_at(address, &mut bytes);

        Ok(u32::from_le_bytes(bytes))
    }

    unsafe fn read_u32_be(&self, address: PhysicalAddress) -> Result<u32, Self::Error> {
        let mut bytes = [0; 4];
        read_bytes_at(address, &mut bytes);

        Ok(u32::from_be_bytes(bytes))
    }

    unsafe fn read_u64_le(&self, address: PhysicalAddress) -> Result<u64, Self::Error> {
        let mut bytes = [0; 8];
        read_bytes_at(address, &mut bytes);

        Ok(u64::from_le_bytes(bytes))
    }

    unsafe fn read_u64_be(&self, address: PhysicalAddress) -> Result<u64, Self::Error> {
        let mut bytes = [0; 8];
        read_bytes_at(address, &mut bytes);

        Ok(u64::from_be_bytes(bytes))
    }

    unsafe fn write_u8(&mut self, address: PhysicalAddress, value: u8) -> Result<(), Self::Error> {
        write_bytes_at(address, slice::from_ref(&value));
        Ok(())
    }

    unsafe fn write_u16_le(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_le_bytes());
        Ok(())
    }

    unsafe fn write_u16_be(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_be_bytes());
        Ok(())
    }

    unsafe fn write_u32_le(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_le_bytes());
        Ok(())
    }

    unsafe fn write_u32_be(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_be_bytes());
        Ok(())
    }

    unsafe fn write_u64_le(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_le_bytes());
        Ok(())
    }

    unsafe fn write_u64_be(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error> {
        write_bytes_at(address, &value.to_be_bytes());
        Ok(())
    }
}

/// Reads the bytes located at `address` into the provided `bytes` slice.
///
/// This function calls [`map_temporary()`].
pub fn read_bytes_at(mut address: PhysicalAddress, mut bytes: &mut [u8]) {
    let page_size = usize_to_u64(page_size());
    while !bytes.is_empty() {
        // Calculate the number of bytes we will read this iteration.
        let size = u64_to_usize_strict(
            (page_size - address.value() % page_size).min(usize_to_u64(bytes.len())),
        );

        // Map the page we will modify.
        let ptr = map_temporary(address);

        // SAFETY:
        //
        // The region that starts at `ptr` was a valid mapping and `size` represents the
        // minimum of the mapping size or the size of the remaining bytes to be copied.
        let read_slice = unsafe { slice::from_raw_parts(ptr, size) };
        bytes[..size].copy_from_slice(read_slice);

        address = PhysicalAddress::new(address.value().wrapping_add(usize_to_u64(size)));
        bytes = &mut bytes[size..];
    }
}

/// Writes the bytes in `bytes` into the physical memory located at `address`.
///
/// This function calls [`map_temporary()`].
pub fn write_bytes_at(mut address: PhysicalAddress, mut bytes: &[u8]) {
    let page_size = usize_to_u64(page_size());
    while !bytes.is_empty() {
        // Calculate the number of bytes we will write this iteration.
        let size = u64_to_usize_strict(
            (page_size - address.value() % page_size).min(usize_to_u64(bytes.len())),
        );
        // Map the page we will modify.
        let ptr = map_temporary(address);

        // SAFETY:
        //
        // The region that starts at `ptr` was a valid mapping and `size` represents the
        // minimum of the mapping size or the size of the bytes that have not yet been written.
        unsafe { ptr::copy(bytes.as_ptr(), ptr, size) }

        address = PhysicalAddress::new(address.value().wrapping_add(usize_to_u64(size)));
        bytes = &bytes[size..];
    }
}

/// Maps the physical memory region of `size` bytes into `revm-stub`'s virtual address space.
///
/// # Panics
///
/// This function may panic if the provided implementation produces an incorrect response.
pub fn map_identity(range: PhysicalAddressRange) -> *mut u8 {
    let ptr = platform().map_identity(range);
    assert_eq!(
        usize_to_u64(ptr.addr()),
        range.start().value(),
        "identity map implementation failed"
    );
    ptr
}
/// Returns the [`PhysicalAddress`] associated with the provided `virtual_address` if the
/// translation is valid. Otherwise, return [`None`].
pub fn translate_virtual(address: VirtualAddress) -> Option<PhysicalAddress> {
    platform().translate_virtual(address)
}

/// Executes a takover on behalf of the loaded executable. `key` is utilized to ensure that the
/// memory map that the loaded executable has is accurate.
///
/// On success, the executable becomes the sole controller of the system. This means that the
/// executable is free to directly manipulate the hardware in whatever manner it desires.
pub fn takeover(key: u64, flags: TakeoverFlags) -> stub_api::Status {
    platform().takeover(key, flags)
}

/// Returns the [`PhysicalAddress`] of the UEFI system table, if present.
pub fn uefi_system_table() -> Option<PhysicalAddress> {
    platform().uefi_system_table()
}

/// Returns the [`PhysicalAddress`] of the RSDP structure, if present.
pub fn rsdp() -> Option<PhysicalAddress> {
    platform().rsdp()
}

/// Returns the [`PhysicalAddress`] of the XSDP structure, if present.
pub fn xsdp() -> Option<PhysicalAddress> {
    platform().xsdp()
}

/// Returns the [`PhysicalAddress`] of the device tree structure, if present.
pub fn device_tree() -> Option<PhysicalAddress> {
    platform().device_tree()
}

/// Returns the [`PhysicalAddress`] of the SMBIO 32 Entry Point, if present.
pub fn smbios_32() -> Option<PhysicalAddress> {
    platform().smbios_32()
}

/// Returns the [`PhysicalAddress`] of the SMBIO 64 Entry Point, if present.
pub fn smbios_64() -> Option<PhysicalAddress> {
    platform().smbios_64()
}

/// Prints the provided [`fmt::Arguments`] to the platform's output device.
pub fn print(args: fmt::Arguments) {
    platform().print(args)
}

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    if level >= LogLevel::Trace {
        match level {
            LogLevel::Trace => platform().print(format_args!("TRACE: {args}\n")),
            LogLevel::Debug => platform().print(format_args!("DEBUG: {args}\n")),
            LogLevel::Info => platform().print(format_args!("INFO : {args}\n")),
            LogLevel::Warn => platform().print(format_args!("WARN : {args}\n")),
            LogLevel::Error => platform().print(format_args!("ERROR: {args}\n")),
        }
    }
}

/// Logs a message with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Trace,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Debug,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Info,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Warn,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Error`].
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Error,
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
    fn allocate_frames(
        &self,
        count: u64,
        policy: AllocationPolicy,
    ) -> Result<FrameRange, OutOfMemory>;

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
    ) -> Result<FrameRange, OutOfMemory> {
        assert!(alignment.is_power_of_two());

        if self.frame_size() >= alignment {
            self.allocate_frames(count, policy)
        } else {
            let total_count = alignment
                .div_ceil(self.frame_size())
                .checked_add(count)
                .ok_or(OutOfMemory)?;

            let chunk_range = self.allocate_frames(total_count, policy)?;

            let returned_chunk = chunk_range
                .start()
                .checked_align_up(self.frame_size(), alignment)
                .ok_or(OutOfMemory)?;
            let requested_range = FrameRange::new(returned_chunk, count);

            let (lower, middle, upper) = chunk_range.partition(requested_range);

            if !lower.is_empty() {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(lower) };
            }

            if !lower.is_empty() {
                // SAFETY:
                //
                // The region described was allocated by a call to [`Platform::allocate_frames()`].
                unsafe { self.deallocate_frames(upper) };
            }

            Ok(middle)
        }
    }

    /// Deallocates the provided physical [`FrameRange`].
    ///
    /// # Safety
    ///
    /// These frames must have been allocated by a call to [`Platform::allocate_frames()`].
    unsafe fn deallocate_frames(&self, range: FrameRange);

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
    fn map_temporary(&self, address: PhysicalAddress) -> *mut u8;

    /// Identity maps the [`PhysicalAddressRange`] into `revm-stub`'s virtual address space.
    fn map_identity(&self, range: PhysicalAddressRange) -> *mut u8;

    /// Returns the [`PhysicalAddress`] associated with the provided [`VirtualAddress`] if the
    /// translation is valid. Otherwise, return [`None`].
    fn translate_virtual(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress>;

    /// Executes a takover on behalf of the loaded executable. `key` is utilized to ensure that the
    /// memory map that the loaded executable has is accurate.
    ///
    /// On success, the executable becomes the sole controller of the system. This means that the
    /// executable is free to directly manipulate the hardware in whatever manner it desires.
    fn takeover(&self, key: u64, flags: TakeoverFlags) -> Status;

    /// Prints the provided [`Arguments`][fmt::Arguments].
    fn print(&self, args: fmt::Arguments);

    /// Returns the [`PhysicalAddress`] of the UEFI system table, if present.
    fn uefi_system_table(&self) -> Option<PhysicalAddress>;

    /// Returns the [`PhysicalAddress`] of the RSDP structure, if present.
    fn rsdp(&self) -> Option<PhysicalAddress>;

    /// Returns the [`PhysicalAddress`] of the XSDP structure, if present.
    fn xsdp(&self) -> Option<PhysicalAddress>;

    /// Returns the [`PhysicalAddress`] of the device tree structure, if present.
    fn device_tree(&self) -> Option<PhysicalAddress>;

    /// Returns the [`PhysicalAddress`] of the SMBIO 32 Entry Point, if present.
    fn smbios_32(&self) -> Option<PhysicalAddress>;

    /// Returns the [`PhysicalAddress`] of the SMBIO 64 Entry Point, if present.
    fn smbios_64(&self) -> Option<PhysicalAddress>;
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

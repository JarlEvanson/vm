//! Definitions and implementations of interfaces provided by architecture-specific code related to
//! physical memory.

use crate::memory::phys::structs::PhysicalAddress;

/// A trait representing low-level access to physical memory.
///
/// The `PhysicalMemory` trait provides an abstraction for reading from and writing to
/// physical memory in a platform-agnostic way. It supports multiple integer sizes
/// (`u8`, `u16`, `u32`, `u64`) and both little-endian and big-endian formats.
///
/// # Purpose
///
/// This trait is typically implemented by architecture-specific code that manages
/// access to physical memory regions. It allows higher-level code—such as operating
/// systems, hypervisors, or device drivers—to perform memory operations without
/// directly depending on platform-specific memory instructions.
///
/// # Endianness
///
/// Methods are provided for both little-endian (`*_le`) and big-endian (`*_be`)
/// access. Correctly using these ensures proper interpretation of multi-byte
/// values on architectures that differ in byte order.
///
/// # Error Handling
///
/// Each implementation defines its own `Error` type. Operations return a `Result`
/// indicating success or the specific failure (e.g., invalid address, alignment
/// fault, or hardware access error).
///
/// # Safety Considerations
///
/// Accessing physical memory can have side effects and may violate memory safety
/// if not carefully managed. Implementers must ensure proper address validation
/// and alignment handling.
#[expect(clippy::missing_safety_doc)]
#[expect(clippy::missing_errors_doc)]
pub unsafe trait PhysicalMemory {
    /// Various errors that can occur while reading or writing [`PhysicalMemory`].
    type Error;

    /// Reads a single byte from the [`PhysicalMemory`].
    unsafe fn read_u8(&self, address: PhysicalAddress) -> Result<u8, Self::Error>;
    /// Reads a little-endian u16 from the [`PhysicalMemory`].
    unsafe fn read_u16_le(&self, address: PhysicalAddress) -> Result<u16, Self::Error>;
    /// Reads a bit-endian u16 from the [`PhysicalMemory`].
    unsafe fn read_u16_be(&self, address: PhysicalAddress) -> Result<u16, Self::Error>;
    /// Reads a little-endian u32 from the [`PhysicalMemory`].
    unsafe fn read_u32_le(&self, address: PhysicalAddress) -> Result<u32, Self::Error>;
    /// Reads a big-endian u32 from the [`PhysicalMemory`].
    unsafe fn read_u32_be(&self, address: PhysicalAddress) -> Result<u32, Self::Error>;
    /// Reads a little-endian u64 from the [`PhysicalMemory`].
    unsafe fn read_u64_le(&self, address: PhysicalAddress) -> Result<u64, Self::Error>;
    /// Reads a big-endian u64 from the [`PhysicalMemory`].
    unsafe fn read_u64_be(&self, address: PhysicalAddress) -> Result<u64, Self::Error>;

    /// Writes a single byte into the [`PhysicalMemory`].
    unsafe fn write_u8(&mut self, address: PhysicalAddress, value: u8) -> Result<(), Self::Error>;
    /// Writes a little-endian u16 into the [`PhysicalMemory`].
    unsafe fn write_u16_le(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error>;
    /// Writes a big-endian u16 into the [`PhysicalMemory`].
    unsafe fn write_u16_be(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error>;
    /// Writes a little-endian u32 into the [`PhysicalMemory`].
    unsafe fn write_u32_le(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error>;
    /// Writes a big-endian u32 into the [`PhysicalMemory`].
    unsafe fn write_u32_be(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error>;
    /// Writes a little-endian u64 into the [`PhysicalMemory`].
    unsafe fn write_u64_le(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error>;
    /// Writes a big-endian u64 into the [`PhysicalMemory`].
    unsafe fn write_u64_be(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error>;
}

//! Definitions and implementations of physical memory management APIs for `revm`.

use core::{convert::Infallible, error, fmt, ptr};

use crate::{
    arch::generic::memory::phys::PhysicalMemory,
    memory::{
        page_frame_size,
        phys::structs::{Frame, FrameRange, PhysicalAddress},
        virt::map_temporary,
    },
    util::{u64_to_usize_panicking, usize_to_u64},
};

pub mod structs;

/// Allocates a region of `count` frames aligned to `alignment` bytes.
///
/// # Errors
///
/// [`FrameAllocationError`] is returned if an error occurs while allocating [`Frame`]s.
pub fn allocate_frames(count: u64, alignment: u64) -> Result<FrameRange, FrameAllocationError> {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let total_bytes = count.strict_mul(usize_to_u64(page_frame_size()));
        let stub_frame_count = total_bytes.div_ceil(generic_table.page_frame_size);

        let mut physical_address = 0;

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.allocate_frames)(
                stub_frame_count,
                alignment.max(alignment),
                stub_api::AllocationFlags::ANY,
                &mut physical_address,
            )
        };
        if result != stub_api::Status::SUCCESS {
            return Err(FrameAllocationError);
        }

        let start = Frame::containing_address(PhysicalAddress::new(physical_address));
        Ok(FrameRange::new(start, count))
    } else {
        todo!("implement post-takeover frame allocation")
    }
}

/// Deallocates the [`FrameRange`].
///
/// # Safety
///
/// The physical memory region described by [`FrameRange`] must not be in use.
pub unsafe fn deallocate_frames(frame_range: FrameRange) {
    if let Some(generic_table) = crate::stub_protocol::generic_table() {
        let stub_frame_count = frame_range
            .byte_count()
            .div_ceil(generic_table.page_frame_size);

        // SAFETY:
        //
        // `generic_table()` returned a valid [`GenericTable`], so this function is required to be
        // functional.
        let result = unsafe {
            (generic_table.deallocate_frames)(
                frame_range.start().start_address().value(),
                stub_frame_count,
            )
        };
        if result != stub_api::Status::SUCCESS {
            crate::warn!("error deallocating frames: {result:?}");
        }
    } else {
        todo!("implement post-takeover frame allocation")
    }
}

/// Various errors that can occur while allocating [`Frame`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameAllocationError;

impl fmt::Display for FrameAllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error allocating frames")
    }
}

impl error::Error for FrameAllocationError {}

/// [`PhysicalMemory`] implementation that writes directly to `revm`'s view of physical memory.
pub struct BareMetalMemory;

impl BareMetalMemory {
    /// Helper to perform a volatile read of type T at a physical address
    #[expect(clippy::undocumented_unsafe_blocks)]
    unsafe fn read_volatile<T>(&self, address: PhysicalAddress) -> T {
        let offset = address.frame_offset();
        let page = map_temporary(Frame::containing_address(address));
        let ptr = ptr::with_exposed_provenance::<T>(
            page.start_address().value() + u64_to_usize_panicking(offset),
        );

        unsafe { ptr.read_volatile() }
    }

    /// Helper to perform a volatile write of type T at a physical address
    #[expect(clippy::undocumented_unsafe_blocks)]
    unsafe fn write_volatile<T>(&mut self, address: PhysicalAddress, value: T) {
        let offset = address.frame_offset();
        let page = map_temporary(Frame::containing_address(address));
        let ptr = ptr::with_exposed_provenance_mut::<T>(
            page.start_address().value() + u64_to_usize_panicking(offset),
        );

        unsafe { ptr.write_volatile(value) }
    }
}

// SAFETY:
//
// Virtual memory is utilized to ensure that all physical memory can be accessed.
#[expect(clippy::undocumented_unsafe_blocks)]
unsafe impl PhysicalMemory for BareMetalMemory {
    type Error = Infallible;

    unsafe fn read_u8(&self, address: PhysicalAddress) -> Result<u8, Self::Error> {
        Ok(unsafe { self.read_volatile(address) })
    }

    unsafe fn read_u16_le(&self, address: PhysicalAddress) -> Result<u16, Self::Error> {
        Ok(u16::from_le(unsafe { self.read_volatile(address) }))
    }

    unsafe fn read_u16_be(&self, address: PhysicalAddress) -> Result<u16, Self::Error> {
        Ok(u16::from_be(unsafe { self.read_volatile(address) }))
    }

    unsafe fn read_u32_le(&self, address: PhysicalAddress) -> Result<u32, Self::Error> {
        Ok(u32::from_le(unsafe { self.read_volatile(address) }))
    }

    unsafe fn read_u32_be(&self, address: PhysicalAddress) -> Result<u32, Self::Error> {
        Ok(u32::from_be(unsafe { self.read_volatile(address) }))
    }

    unsafe fn read_u64_le(&self, address: PhysicalAddress) -> Result<u64, Self::Error> {
        Ok(u64::from_le(unsafe { self.read_volatile(address) }))
    }

    unsafe fn read_u64_be(&self, address: PhysicalAddress) -> Result<u64, Self::Error> {
        Ok(u64::from_be(unsafe { self.read_volatile(address) }))
    }

    unsafe fn write_u8(&mut self, address: PhysicalAddress, value: u8) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value) };
        Ok(())
    }

    unsafe fn write_u16_le(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_le()) };
        Ok(())
    }

    unsafe fn write_u16_be(
        &mut self,
        address: PhysicalAddress,
        value: u16,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_be()) };
        Ok(())
    }

    unsafe fn write_u32_le(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_le()) };
        Ok(())
    }

    unsafe fn write_u32_be(
        &mut self,
        address: PhysicalAddress,
        value: u32,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_be()) };
        Ok(())
    }

    unsafe fn write_u64_le(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_le()) };
        Ok(())
    }

    unsafe fn write_u64_be(
        &mut self,
        address: PhysicalAddress,
        value: u64,
    ) -> Result<(), Self::Error> {
        unsafe { self.write_volatile(address, value.to_be()) };
        Ok(())
    }
}

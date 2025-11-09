//! Context for boot protocols.
//!
//! Provides various utilities necessary to prepare and initialize REVM.

use core::fmt::Write;

/// Provider of all boot context dependent functionality.
pub struct Context<'context>(&'context mut dyn ContextImpl);

impl<'context> Context<'context> {
    /// Creates a new [`Context`] from a [`ContextImpl`].
    ///
    /// Various parts of the implementation are also tested for correctness.
    pub(super) fn new(context_impl: &'context mut dyn ContextImpl) -> Self {
        const { assert!(usize::BITS <= u64::BITS) }

        // Validate `context_impl.physical_bits()`.
        assert_ne!(
            context_impl.physical_bits(),
            0,
            "physical addresses must be represented with a non-zero number of bits"
        );
        assert!(
            context_impl.physical_bits() <= 64,
            "physical addresses must be represented with at most 64 bits"
        );

        // Validate `context_impl.frame_size()`.
        assert_ne!(context_impl.frame_size(), 0, "frame size must not be zero");
        assert!(
            context_impl.frame_size().is_power_of_two(),
            "frame size must be a power of two"
        );
        assert!(
            context_impl.frame_size().ilog2() < u32::from(context_impl.physical_bits()),
            "frame size must fit into a physical address"
        );

        // Validate `context_impl.page_size()`.
        assert_ne!(context_impl.page_size(), 0, "page size must not be zero");
        assert!(
            context_impl.page_size().is_power_of_two(),
            "page size must be a power of two"
        );

        Self(context_impl)
    }

    /// Returns the number of bits used to represent a valid physical address.
    pub fn physical_bits(&self) -> u8 {
        self.0.physical_bits()
    }

    /// Returns the physical address mask that represents a valid physical address.
    pub fn physical_address_mask(&self) -> u64 {
        (((1u64 << (self.physical_bits() - 1)) - 1) << 1) + 1
    }

    /// Returns `true` if `address` is valid.
    pub fn is_valid_physical_address(&self, address: u64) -> bool {
        address & self.physical_address_mask() == address
    }

    /// Returns `true` if `size` is valid for the address space.
    pub fn is_valid_physical_size(&self, size: u64) -> bool {
        size <= self.physical_address_mask()
    }

    /// Returns the size, in bytes, of a frame.
    pub fn frame_size(&self) -> u64 {
        self.0.frame_size()
    }

    /// Deallocates a region of `count` frames with a base of `physical_address`.
    ///
    /// These frames must have been allocated by a call to [`ContextImpl::allocate_frames()`].
    pub fn allocate_frames(
        &mut self,
        kind: AllocationPolicy,
        count: u64,
    ) -> Result<u64, OutOfMemory> {
        assert_ne!(count, 0, "frame allocations of size zero are illegal");
        let Some(total_size) = count.checked_mul(self.frame_size()) else {
            return Err(OutOfMemory);
        };
        if !self.is_valid_physical_size(total_size) {
            return Err(OutOfMemory);
        }

        let address = match kind {
            AllocationPolicy::Any => self.0.allocate_frames(kind, count)?,
            AllocationPolicy::At(address) => {
                assert!(
                    address.is_multiple_of(self.frame_size()),
                    "AllocationPolicy::At address must be frame aligned"
                );
                assert!(
                    self.is_valid_physical_address(address),
                    "AllocationPolicy::At address must be valid"
                );

                let end_address = address.wrapping_add(total_size);
                let last_valid_address = end_address.wrapping_sub(1);
                if last_valid_address < address
                    || !self.is_valid_physical_address(last_valid_address)
                {
                    return Err(OutOfMemory);
                }

                let allocated_address = self.allocate_frames(kind, count)?;
                assert_eq!(
                    allocated_address, address,
                    "AllocationPolicy::At failed: requested {:032X}; received {:032X}",
                    address, allocated_address
                );
                allocated_address
            }
            AllocationPolicy::Below(value) => {
                if total_size >= value {
                    return Err(OutOfMemory);
                }

                let allocated_address = self.allocate_frames(kind, count)?;
                let allocated_last_valid_address =
                    allocated_address.wrapping_add(total_size).wrapping_sub(1);
                assert!(
                    allocated_last_valid_address < value,
                    "AllocationPolicy::Below failed: \
                    last valid address is {:032X} which is greater than {:032X}",
                    allocated_last_valid_address,
                    value
                );
                allocated_address
            }
        };

        assert!(
            address.is_multiple_of(self.frame_size()),
            "allocated frame region base must be frame aligned"
        );
        assert!(
            self.is_valid_physical_address(address),
            "allocated frame region must start at a valid physical address"
        );

        let end_address = address.wrapping_add(total_size);
        let last_valid_address = end_address.wrapping_sub(1);
        assert!(
            last_valid_address >= address,
            "allocated frame region must not wrap around"
        );
        assert!(
            self.is_valid_physical_address(last_valid_address),
            "allocated frame region must end at a valid physical address"
        );

        Ok(address)
    }

    /// Deallocates a region of `count` frames with a base of `physical_address`.
    ///
    /// # Safety
    ///
    /// These frames must have been allocated by a call to [`Context::allocate_frames()`].
    unsafe fn deallocate_frames(&mut self, physical_address: u64, count: u64) {
        assert_ne!(count, 0, "frame deallocations of size zero are illegal");
        assert!(
            self.is_valid_physical_address(physical_address),
            "frame deallocation region must start at a valid physical address"
        );

        let total_size = count.wrapping_mul(self.frame_size());
        assert!(
            count.checked_mul(self.frame_size()).is_some(),
            "frame deallocation size overflowed"
        );

        let end_address = physical_address.wrapping_add(total_size);
        let last_valid_address = end_address.wrapping_sub(1);
        assert!(
            last_valid_address >= physical_address,
            "frame deallocation region must not wrap around"
        );
        assert!(
            self.is_valid_physical_address(last_valid_address),
            "frame deallocation region must end at a valid physical address"
        );

        // SAFETY:
        //
        // All invariants still carry over.
        unsafe { self.0.deallocate_frames(physical_address, count) }
    }

    /// Returns the size, in bytes, of a page.
    pub fn page_size(&self) -> usize {
        self.0.page_size()
    }

    /// Maps the physical region with a base of `physical_address` and a size of `count *
    /// ContextImpl::page_size()` bytes into the stub's address space.
    fn map_frames(
        &mut self,
        physical_address: u64,
        count: usize,
    ) -> Result<*mut (), FailedMapping> {
        assert_ne!(count, 0, "page allocations of size zero are illegal");

        assert!(
            physical_address.is_multiple_of(self.page_size() as u64),
            "mapped physical address must be page aligned"
        );
        let Some(total_size) = count.checked_mul(self.page_size()) else {
            return Err(FailedMapping);
        };

        let end_physical_address = physical_address.wrapping_add(total_size as u64);
        let last_valid_physical_address = end_physical_address.wrapping_sub(1);
        if last_valid_physical_address < physical_address {
            return Err(FailedMapping);
        }

        let ptr = self.0.map_frames(physical_address, count)?;
        let virtual_address = ptr.addr();

        let end_virtual_address = virtual_address.wrapping_add(total_size);
        let last_valid_virtual_address = end_virtual_address.wrapping_sub(1);
        assert!(
            last_valid_virtual_address >= virtual_address,
            "frame mapping must not wrap around"
        );

        Ok(ptr)
    }

    /// Identity maps the physical region with a base of `physical_address` and a size of `count *
    /// ContextImpl::page_size()` bytes into the stub's address space.
    fn map_frames_identity(
        &mut self,
        physical_address: u64,
        count: usize,
    ) -> Result<*mut (), FailedMapping> {
        assert_ne!(count, 0, "frame mappings of size zero are illegal");

        assert!(
            physical_address.is_multiple_of(self.page_size() as u64),
            "mapped physical address must be page aligned"
        );
        let Some(total_size) = count.checked_mul(self.page_size()) else {
            return Err(FailedMapping);
        };

        let end_physical_address = physical_address.wrapping_add(total_size as u64);
        let last_valid_physical_address = end_physical_address.wrapping_sub(1);
        if last_valid_physical_address < physical_address {
            return Err(FailedMapping);
        }

        let Ok(end_virtual_address) = usize::try_from(end_physical_address) else {
            return Err(FailedMapping);
        };
        if end_virtual_address == 0 {
            return Err(FailedMapping);
        }

        let ptr = self.0.map_frames_identity(physical_address, count)?;
        assert_eq!(
            ptr.addr() as u64,
            physical_address,
            "identity map implementation failed"
        );
        Ok(ptr)
    }

    /// Unmaps `count` pages with a base of `page` from the stub's address space.
    ///
    /// # Safety
    ///
    /// These pages must have been mapped by a call to [`ContextImpl::map_frames()`] or
    /// [`ContextImpl::map_frames_identity()`].
    unsafe fn unmap_frames(&mut self, page: *mut (), count: usize) {
        assert_ne!(count, 0, "page unmappings of size zero are illegal");
        assert!(
            page.addr().is_multiple_of(self.page_size()),
            "unmapping page base must be page aligned"
        );

        let total_size = count.wrapping_mul(self.page_size());
        assert!(
            count.checked_mul(self.page_size()).is_some(),
            "page unmapping size overflowed"
        );

        let end_address = page.addr().wrapping_add(total_size);
        let last_valid_address = end_address.wrapping_sub(1);
        assert!(
            last_valid_address >= end_address,
            "unmapped pages must not wrap"
        );

        // SAFETY:
        //
        // All invariants still carry over.
        unsafe { self.0.unmap_frames(page, count) }
    }

    /// Returns the device tree associated with the device.
    fn device_tree(&mut self) -> Result<*mut u8, NotFound> {
        self.0.device_tree()
    }

    /// Returns the ACPI RSDP associated with the device.
    fn acpi_rsdp(&mut self) -> Result<*mut u8, NotFound> {
        self.0.acpi_rsdp()
    }

    /// Returns the ACPI XSDP associated with the device.
    fn acpi_xsdp(&mut self) -> Result<*mut u8, NotFound> {
        self.0.acpi_xsdp()
    }
}

impl Write for Context<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.write_str(s)
    }
}

/// Provider of various functionalities required in order to carry out the functions of the stub.
pub(super) trait ContextImpl: Write {
    /// Returns the number of bits used to represent a valid physical address.
    fn physical_bits(&self) -> u8;

    /// Returns the size, in bytes, of a frame.
    fn frame_size(&self) -> u64;

    /// Allocates a region of `count` frames according to the given [`AllocationPolicy`].
    fn allocate_frames(&mut self, kind: AllocationPolicy, count: u64) -> Result<u64, OutOfMemory>;

    /// Deallocates a region of `count` frames with a base of `physical_address`.
    ///
    /// # Safety
    ///
    /// These frames must have been allocated by a call to [`ContextImpl::allocate_frames()`].
    unsafe fn deallocate_frames(&mut self, physical_address: u64, count: u64);

    /// Returns the size, in bytes, of a page.
    fn page_size(&self) -> usize;

    /// Maps the physical region with a base of `physical_address` and a size of `count *
    /// ContextImpl::page_size()` bytes into the stub's address space.
    fn map_frames(&mut self, physical_address: u64, count: usize)
    -> Result<*mut (), FailedMapping>;

    /// Identity maps the physical region with a base of `physical_address` and a size of `count *
    /// ContextImpl::page_size()` bytes into the stub's address space.
    fn map_frames_identity(
        &mut self,
        physical_address: u64,
        count: usize,
    ) -> Result<*mut (), FailedMapping>;

    /// Unmaps `count` pages with a base of `page` from the stub's address space.
    ///
    /// # Safety
    ///
    /// These pages must have been mapped by a call to [`ContextImpl::map_frames()`] or
    /// [`ContextImpl::map_frames_identity()`].
    unsafe fn unmap_frames(&mut self, page: *mut (), count: usize);

    /// Returns the device tree associated with the device.
    fn device_tree(&mut self) -> Result<*mut u8, NotFound>;
    /// Returns the ACPI RSDP associated with the device.
    fn acpi_rsdp(&mut self) -> Result<*mut u8, NotFound>;
    /// Returns the ACPI XSDP associated with the device.
    fn acpi_xsdp(&mut self) -> Result<*mut u8, NotFound>;
}

/// Determines how frames must be allocated.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum AllocationPolicy {
    /// Allocates any available range of frames that satisfies the request.
    Any,
    /// Allocates the range of frames that begins at the requested address.
    At(u64),
    /// Allocates any available range of frames that is completely below the given [`u64`].
    ///
    /// Note: The [`u64`] does not need to be a valid physical address.
    Below(u64),
}

/// Indicates that there were no available frame regions that matched the [`AllocationPolicy`].
pub struct OutOfMemory;
/// Indicates that the mapping failed because the region was too large or the physical region is
/// not able to be fully mapped.
pub struct FailedMapping;
/// Indicates that an object was not able to be found.
pub struct NotFound;

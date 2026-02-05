//! Implementation and definitions of frame allocation.

use core::{
    error, fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use stub_api::{AllocationFlags, Status};

use crate::stub_protocol::generic_table;

/// The size, in bytes, of frames from the perspective of `revm`.
static FRAME_SIZE: AtomicU64 = AtomicU64::new(0);

/// Initializes the frame allocator.
pub fn initialize(frame_size: u64) {
    assert!(frame_size.is_power_of_two());
    assert_eq!(
        FRAME_SIZE.load(Ordering::Acquire),
        0,
        "frame size must only be set once"
    );
    FRAME_SIZE.store(frame_size, Ordering::Release);
}

/// Returns the size, in bytes, of each frame from the perspective of `revm`.
pub fn frame_size() -> u64 {
    FRAME_SIZE.load(Ordering::Acquire)
}

/// Allocates a region of `count` frames with an alignment of `alignment`.
///
/// # Errors
///
/// Returns [`OutOfMemory`] if the system cannot allocate the requested frames.
pub fn allocate_frames(count: u64, alignment: u64) -> Result<FrameAllocation, OutOfMemory> {
    assert_ne!(count, 0, "zero-sized frame allocations are not allowed");
    assert!(
        alignment.is_power_of_two(),
        "requested physical alignment must be a power of two"
    );

    if let Some(generic_table_ptr) = generic_table() {
        // SAFETY:
        //
        // The REVM protocol and [`generic_table()`] work together to ensure that the generic table
        // pointer points to a valid table at this time and that the REVM protocol tables have been
        // validated as much as possible.
        let page_frame_size = unsafe { (*generic_table_ptr.as_ptr()).page_frame_size };
        assert_eq!(frame_size(), page_frame_size);

        // SAFETY:
        //
        // The REVM protocol and [`generic_table()`] work together to ensure that the generic table
        // pointer points to a valid table at this time and that the REVM protocol tables have been
        // validated as much as possible.
        let allocate_frames_ptr = unsafe { (*generic_table_ptr.as_ptr()).allocate_frames };

        let mut physical_address = 0;
        // SAFETY:
        //
        // The REVM protocol ensures that the function pointer is valid, while the provided
        // arguments are valid according to the REVM specification.
        let result = unsafe {
            allocate_frames_ptr(
                count,
                alignment,
                AllocationFlags::ANY,
                &mut physical_address,
            )
        };
        if result == Status::SUCCESS {
            Ok(FrameAllocation {
                physical_address,
                count,
            })
        } else {
            Err(OutOfMemory)
        }
    } else {
        todo!()
    }
}

/// Deallocates a region of `count` frames with the starting `physical_address`.
///
/// # Safety
///
/// These frames must not be referenced after the start of this function.
#[expect(
    clippy::missing_panics_doc,
    reason = "panic only if safety invariants are breached"
)]
pub unsafe fn deallocate_frames(physical_address: u64, count: u64) {
    assert!(
        physical_address.is_multiple_of(frame_size()),
        "provided physical address must be frame-aligned"
    );
    assert_ne!(count, 0, "zero-sized frame deallocations are not allowed");

    if let Some(generic_table_ptr) = generic_table() {
        // SAFETY:
        //
        // The REVM protocol and [`generic_table()`] work together to ensure that the generic table
        // pointer points to a valid table at this time and that the REVM protocol tables have been
        // validated as much as possible.
        let page_frame_size = unsafe { (*generic_table_ptr.as_ptr()).page_frame_size };
        assert_eq!(frame_size(), page_frame_size);

        // SAFETY:
        //
        // The REVM protocol and [`generic_table()`] work together to ensure that the generic table
        // pointer points to a valid table at this time and that the REVM protocol tables have been
        // validated as much as possible.
        let deallocate_frames_ptr = unsafe { (*generic_table_ptr.as_ptr()).deallocate_frames };

        // SAFETY:
        //
        // The REVM protocol ensures that the function pointer is valid, while the provided
        // arguments are valid according to the REVM specification.
        let result = unsafe { deallocate_frames_ptr(physical_address, count) };
        if result != Status::SUCCESS {
            crate::warn!(
                "error deallocating frame region: {physical_address:#x}-{:#x}",
                physical_address + count * frame_size()
            );
        }
    } else {
        todo!()
    }
}

/// Wrapper around a region of frames allocated with [`allocate_frames()`].
///
/// This structure automatically frees the region of frames when dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameAllocation {
    /// The physical address of the start of the allocated frame region.
    physical_address: u64,
    /// The number of frames in the allocated frame region.
    count: u64,
}

impl FrameAllocation {
    /// Returns the physical address of the start of the allocated frame region.
    pub const fn physical_address(&self) -> u64 {
        self.physical_address
    }

    /// Returns the number of frames in the allocated frame region.
    pub const fn count(&self) -> u64 {
        self.count
    }

    /// Returns the total number of bytes controlled by the allocated frame region.
    pub fn total_size(&self) -> u64 {
        self.count().strict_mul(FRAME_SIZE.load(Ordering::Acquire))
    }
}

impl Drop for FrameAllocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of frames indicated by `self.physical_address` and `self.count` is under the
        // exclusive control of [`deallocate_frames()`].
        unsafe { deallocate_frames(self.physical_address, self.count) }
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

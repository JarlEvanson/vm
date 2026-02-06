//! Raw view of the table for use across pointer-widths.

#[repr(C)]
#[derive(Clone, Copy)]
#[expect(missing_docs)]
pub struct GenericTable32 {
    pub version: u64,

    pub page_frame_size: u64,

    pub image_physical_address: u64,
    pub image_virtual_address: u64,

    pub write: u32,
    pub allocate_frames: u32,
    pub deallocate_frames: u32,
    pub get_memory_map: u32,
    pub map: u32,
    pub unmap: u32,
    pub takeover: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[expect(missing_docs)]
pub struct GenericTable64 {
    pub version: u64,

    pub page_frame_size: u64,

    pub image_physical_address: u64,
    pub image_virtual_address: u64,

    pub write: u64,
    pub allocate_frames: u64,
    pub deallocate_frames: u64,
    pub get_memory_map: u64,
    pub map: u64,
    pub unmap: u64,
    pub takeover: u64,
}

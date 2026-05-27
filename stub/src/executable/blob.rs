//! Utilities required to extract the embedded blob.

use core::{mem, ptr, slice};

unsafe extern "C" {
    static _blob_start: u8;
}

/// Returns a slice that represents the embedded blob.
///
/// # Panics
///
/// Panics if the blob was not provided or is too large.
pub fn extract_blob() -> &'static [u8] {
    let blob_start_ptr = ptr::addr_of!(_blob_start);
    // SAFETY:
    //
    // When the program is properly packaged, this read is valid since the blob section will be
    // filled with at least 8 bytes.
    let blob_size = unsafe { blob_start_ptr.cast::<u64>().read() };
    let blob_size = usize::try_from(blob_size).expect("blob is too large");

    let blob_ptr = blob_start_ptr.wrapping_byte_add(mem::size_of::<u64>());

    // SAFETY:
    //
    // When the program is properly packaged, this operation is valid since the blob size indicator
    // must be correct and the blob section shall not be editable.
    unsafe { slice::from_raw_parts(blob_ptr, blob_size) }
}

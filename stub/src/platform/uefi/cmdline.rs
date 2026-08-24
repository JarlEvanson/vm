//! Command line extraction and conversion implementation.

use core::{
    error, ffi, fmt,
    ptr::{self, NonNull},
    slice,
};

use conversion::u32_to_usize;
use uefi::{
    data_type::{Handle, Status},
    memory::MemoryType,
    protocol::loaded_image::LoadedImageProtocol,
    table::boot::BootServices1_0,
};

/// Returns the [`Cmdline`] associated with this image.
///
/// # Safety
///
/// - `image_handle` must be the [`Handle`] associated with this image.
/// - `boot_services_ptr` must point to a valid [`BootServices1_0`] structure.
pub unsafe fn acquire_cmdline(
    image_handle: Handle,
    boot_services_ptr: *mut BootServices1_0,
) -> Result<Cmdline, AcquireCmdlineError> {
    // SAFETY:
    //
    // `boot_services_ptr` must point to a valid [`BootServices1_0`] structure and thus must contain
    // a `handle_protocol` function pointer that is non-NULL.
    let handle_protocol = unsafe { (*boot_services_ptr).handle_protocol };

    let guid = LoadedImageProtocol::GUID;
    let mut interface = ptr::null_mut();

    // SAFETY:
    //
    // The provided `image_handle`, `guid`, and `interface` are all valid and thus the invariants of
    // this function are fulfilled.
    let result = unsafe { handle_protocol(image_handle, &guid, &mut interface) };
    if result != Status::SUCCESS {
        return Err(AcquireCmdlineError::AcquireProtocolInterfaceError { status: result });
    }

    let interface = interface.cast::<LoadedImageProtocol>();

    // SAFETY:
    //
    // This [`LoadedImageProtocol`] will be active until this image exits.
    let load_options_size = unsafe { (*interface).load_options_size };
    // SAFETY:
    //
    // This [`LoadedImageProtocol`] will be active until this image exits.
    let load_options_ptr = unsafe { (*interface).load_options };

    let load_options = if !load_options_ptr.is_null() {
        // SAFETY:
        //
        // This [`LoadedImageProtocol`] and its load options will be active until this image exits.
        unsafe {
            slice::from_raw_parts(
                load_options_ptr.cast::<u8>(),
                u32_to_usize(load_options_size),
            )
        }
    } else {
        &[0, 0]
    };

    let mut cmdline_byte_count = 0;
    let raw_chars = load_options
        .as_chunks::<2>()
        .0
        .iter()
        .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]));
    for (index, raw_c) in raw_chars.clone().enumerate() {
        let Some(c) = char::from_u32(u32::from(raw_c)) else {
            return Err(AcquireCmdlineError::InvalidChar {
                position: index * 2,
                value: raw_c,
            });
        };

        // Stop when the first NUL character is encountered.
        if c == '\0' {
            break;
        }

        cmdline_byte_count += c.len_utf8();
    }

    let mut cmdline = if cmdline_byte_count != 0 {
        // SAFETY:
        //
        // `boot_services_ptr` points to a valid [`BootServices1_0`] and thus must contain an
        // `allocate_pool` function pointer that is non-NULL.
        let allocate_pool = unsafe { (*boot_services_ptr).allocate_pool };

        let mut ptr = ptr::null_mut();
        // SAFETY:
        //
        // The provided `MemoryType`, `cmdline_byte_count`, and `ptr` are all valid and thus the
        // invariants of this function are fulfilled.
        let result =
            unsafe { allocate_pool(MemoryType::LOADER_DATA, cmdline_byte_count, &mut ptr) };
        if result != Status::SUCCESS {
            return Err(AcquireCmdlineError::AllocationError {
                status: result,
                size: cmdline_byte_count,
            });
        }

        // SAFETY:
        //
        // The memory region that starts at `ptr` extends for at least `cmdline_byte_count` bytes
        // and is currently owned by this function.
        unsafe { ptr::write_bytes(ptr, 0, cmdline_byte_count) }

        Cmdline {
            ptr: NonNull::new(ptr.cast::<u8>()),
            size: cmdline_byte_count,

            boot_services_ptr,
        }
    } else {
        Cmdline {
            ptr: None,
            size: 0,

            boot_services_ptr,
        }
    };

    let mut buffer = cmdline.buffer_mut();
    for raw_c in raw_chars {
        // Will not panic since this sequence has already been successfully iterated over above.
        let c = char::from_u32(u32::from(raw_c)).unwrap();
        if c == '\0' {
            break;
        }

        let encoded_buffer = c.encode_utf8(buffer);
        let encoded_buffer_len = encoded_buffer.len();
        buffer = &mut buffer[encoded_buffer_len..];
    }

    Ok(cmdline)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcquireCmdlineError {
    /// An attempt to acquire a protocol interface failed.
    AcquireProtocolInterfaceError {
        /// The [`Status`] returned from the failed protocol interface acquisition attempt.
        status: Status,
    },
    /// An invalid UCS-2 character was encountered.
    InvalidChar {
        /// The position, in bytes, of the invalid character.
        position: usize,
        /// The value of the invalid character.
        value: u16,
    },
    /// An allocation attempt failed.
    AllocationError {
        /// The [`Status`] returned from the failed allocation call.
        status: Status,
        /// The number of bytes in the failed allocation.
        size: usize,
    },
}

impl fmt::Display for AcquireCmdlineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::AcquireProtocolInterfaceError { status } => {
                write!(f, "failed to acquire protocol interface: {status}")
            }
            Self::InvalidChar { position, value } => write!(
                f,
                "invalid UCS-2 character '{value}' was encountered {position} bytes into the command line"
            ),
            Self::AllocationError { status, size } => {
                write!(f, "failed to allocate {size} byte(s): {status}")
            }
        }
    }
}

impl error::Error for AcquireCmdlineError {}

pub(super) struct Cmdline {
    ptr: Option<NonNull<u8>>,
    size: usize,

    boot_services_ptr: *mut BootServices1_0,
}

impl Cmdline {
    /// Returns the [`str`] command line.
    pub fn as_str(&self) -> &str {
        // SAFETY:
        //
        // The entire buffer was filled using `encode_utf8` and thus is entirely valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.buffer()) }
    }

    /// Returns the underlying immutable byte buffer.
    fn buffer(&self) -> &[u8] {
        // SAFETY:
        //
        // The memory region that starts at `self.ptr` extends for at least `self.size` bytes, is
        // owned by this [`Cmdline`] structure, and has been initialized.
        unsafe {
            core::slice::from_raw_parts(
                self.ptr.map(NonNull::as_ptr).unwrap_or_default(),
                self.size,
            )
        }
    }

    /// Returns the underlying mutable byte buffer,
    fn buffer_mut(&mut self) -> &mut [u8] {
        // SAFETY:
        //
        // The memory region that starts at `self.ptr` extends for at least `self.size` bytes, is
        // owned by this [`Cmdline`] structure, and has been initialized.
        unsafe {
            core::slice::from_raw_parts_mut(
                self.ptr.map(NonNull::as_ptr).unwrap_or_default(),
                self.size,
            )
        }
    }
}

impl Drop for Cmdline {
    fn drop(&mut self) {
        let Some(ptr) = self.ptr else {
            return;
        };

        // SAFETY:
        //
        // `boot_services_ptr` points to a valid [`BootServices1_0`] and thus must contain a
        // `free_pool` function pointer that is non-NULL.
        let free_pool = unsafe { (*self.boot_services_ptr).free_pool };

        // SAFETY:
        //
        // The provided `ptr` is valid and thus the invariants of this function are fulfilled.
        let result = unsafe { free_pool(ptr.as_ptr().cast::<ffi::c_void>()) };
        if result != Status::SUCCESS {
            crate::warn!("error freeing stub command line: {result}");
        }
    }
}

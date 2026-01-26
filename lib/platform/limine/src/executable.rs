//! Definitions of [`ExecutableFileRequest`] and [`ExecutableFileResponse`].

use crate::{REQUEST_MAGIC_0, REQUEST_MAGIC_1};

/// Magic numbers identifying the request as an [`ExecutableFileRequest`].
pub const EXECUTABLE_FILE_REQUEST_MAGIC: [u64; 4] = [
    REQUEST_MAGIC_0,
    REQUEST_MAGIC_1,
    0xad97e90e83f1ed67,
    0x31eb5d1c5ff23b69,
];

/// Request for the [`File`] associated with the loaded executable file.
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableFileRequest {
    /// Location storing [`EXECUTABLE_FILE_REQUEST_MAGIC`] to identify the request.
    pub id: [u64; 4],
    /// The revision of the [`ExecutableFileRequest`] structure.
    pub revision: u64,
    /// A pointer to the [`ExecutableFileResponse`] structure for this [`ExecutableFileRequest`].
    pub response: u64,
}

// SAFETY:
//
// [`ExecutableFileRequest`] does not interact with threads in any manner.
unsafe impl Send for ExecutableFileRequest {}
// SAFETY:
//
// [`ExecutableFileRequest`] does not interact with threads in any manner.
unsafe impl Sync for ExecutableFileRequest {}

/// Response to an [`ExecutableFileRequest`].
#[repr(C)]
#[derive(Debug)]
pub struct ExecutableFileResponse {
    /// The revision of the [`ExecutableFileResponse`] structure.
    pub revision: u64,
    /// A pointer to the [`File`] structure for the executable file.
    pub file: u64,
}

// SAFETY:
//
// [`ExecutableFileResponse`] does not interact with threads in any manner.
unsafe impl Send for ExecutableFileResponse {}
// SAFETY:
//
// [`ExecutableFileResponse`] does not interact with threads in any manner.
unsafe impl Sync for ExecutableFileResponse {}

/// Description of a file loaded according to the Limine boot protocol.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct File {
    /// The revision of the [`File`] structure.
    pub revision: u64,
    /// The address of the file.
    ///
    /// This is always 4KiB aligned.
    pub address: u64,
    /// The size of the file in bytes.
    pub size: u64,
    /// The path of the file within the volume, with a leading slash.
    pub path: u64,
    /// A command line associated with the file.
    pub command_line: u64,
    /// The type of media the file resides on.
    pub media_type: MediaType,
    /// Currently unused.
    pub _unused: u32,
    /// If non-zero, the IP of the TFTP server the file was loaded from.
    pub tftp_ip: u32,
    /// If non-zero, the port of the TFTP server the file was loaded from.
    pub tftp_port: u32,
    /// The 1-based partition index of the volume from which the file was loaded.
    ///
    /// If 0, the `partition_index` is invalid or the disk was unpartitioned.
    pub partition_index: u32,
    /// If non-zero, the ID of the disk the file was loaded from as reported in its MBR.
    pub mbr_disk_id: u32,
    /// If non-zero, the UUID of the disk the file was loaded from as reported in its GPT.
    pub gpt_disk_uuid: Uuid,
    /// If non-zero, the UUID of the partition the file was loaded from as reported in the GPT.
    pub gpt_part_uuid: Uuid,
    /// If non-zero, the UUID of the filesystem of the partition the file was loaded from.
    pub part_uuid: Uuid,
}

// SAFETY:
//
// [`File`] does not interact with threads in any manner.
unsafe impl Send for File {}
// SAFETY:
//
// [`File`] does not interact with threads in any manner.
unsafe impl Sync for File {}

/// Various media types.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaType(pub u32);

impl MediaType {
    /// Generic media.
    pub const GENERIC: Self = Self(0);
    /// Optical media.
    pub const OPTICAL: Self = Self(1);
    /// TFTP (network) media.
    pub const TFTP: Self = Self(2);
}

#[allow(missing_docs)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}

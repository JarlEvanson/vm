//! Definitions related to the UEFI System Table.

use core::ffi;

use crate::{
    data_type::Handle,
    protocol::console::simple_text::output::SimpleTextOutputProtocol,
    table::{
        TableHeader, boot::BootServices1_0, config::ConfigurationTable, runtime::RuntimeServices1_0,
    },
};

/// The signature located in [`TableHeader`] that indicates that the UEFI table is a UEFI System
/// Table.
pub const SIGNATURE: u64 = 0x5453595320494249;

/// Contains basic system information and pointers to the UEFI Boot Services and Runtime Services
/// tables.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct SystemTable {
    /// Header of [`SystemTable`].
    pub header: TableHeader,

    /// A pointer to a NUL-terminated string that identifies the vendor that produces the system
    /// firmware for the platform.
    pub firmware_vendor: *const u16,
    /// A firmware vendor specific value that identifies the revsion of the system firmware for the
    /// platform.
    pub firmware_revision: u32,

    /// The [`Handle`] for the active console input device.
    pub console_in_handle: Handle,
    /// A pointer to the interface that is associated with [`SystemTable::console_in_handle`].
    pub con_in: *mut ffi::c_void,

    /// The [`Handle`] for the active console output device.
    pub console_out_handle: Handle,
    /// A pointer to the [`SimpleTextOutputProtocol`] interface that is associated with
    /// [`SystemTable::console_out_handle`].
    pub con_out: *mut SimpleTextOutputProtocol,

    /// The [`Handle`] for the active standard error console device.
    pub standard_error_handle: Handle,
    /// A pointer to the [`SimpleTextOutputProtocol`] interface that is associated with
    /// [`SystemTable::standard_error_handle`].
    pub std_err: *mut SimpleTextOutputProtocol,

    /// A pointer to the base revision of the UEFI Runtime Services table.
    pub runtime_services: *mut RuntimeServices1_0,
    /// A pointer to the base revision of the UEFI Boot Services table.
    pub boot_services: *mut BootServices1_0,

    /// The number of system [`ConfigurationTable`] entries in the buffer
    /// [`SystemTable::configuration_table`].
    pub number_of_table_entries: usize,
    /// A pointer to the system [`ConfigurationTable`]s.
    pub configuration_table: *mut ConfigurationTable,
}

//! Definitions related to UEFI Simple Text Output protocol.

use crate::{
    data_type::{Boolean, Guid, Status},
    guid,
};

/// Used to control text-based output devices.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct SimpleTextOutputProtocol {
    /// Resets the output device.
    pub reset: Reset,

    /// Writes a string to the output device.
    pub output_string: WriteString,
    /// Verifies that all characters in a string can be output to the target device.
    pub test_string: TestString,

    /// Returns information for an available text mode that the output device supports.
    pub query_mode: QueryMode,
    /// Sets the output device to a specified mode.
    pub set_mode: SetMode,

    /// Sets the background and foreground colors.
    pub set_attribute: SetAttribute,
    /// Clears the output device to the currently selected background color.
    pub clear_screen: ClearScreen,

    /// Sets the current coordinates of the cursor position.
    pub set_cursor_position: SetCursorPosition,
    /// Makes the cursor visble or invisible.
    pub enable_cursor: EnableCursor,

    /// The current mode of the protocol.
    pub mode: *mut SimpleTextOutputMode,
}

impl SimpleTextOutputProtocol {
    /// The [`Guid`] associated with the [`SimpleTextOutputProtocol`].
    pub const GUID: Guid = guid!("387477c2-69c7-11d2-8e39-00a0c969723b");
}

/// Resets the text output device hardware.
///
/// The cursor position is set to `(0, 0)`, and the screen is cleared to the default background
/// color for the output device.
///
/// If `extended_verification` is true, the firmware may take an extended amount of time to verify
/// the device is operating on reset.
pub type Reset = unsafe extern "efiapi" fn(
    this: *mut SimpleTextOutputProtocol,
    extended_verification: Boolean,
) -> Status;
/// Writes a string to the output device.
///
/// The string is displayed at the current cursor location on the output device and the cursor is
/// advanced according the rules.
pub type WriteString =
    unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol, string: *const u16) -> Status;
/// Verifies that all characters in a string can be output to the target device.
///
/// This function provides a way to know if the desired character codes are supported for rendering
/// on the output devices.
pub type TestString =
    unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol, string: *const u16) -> Status;
/// Returns information for an available text mode that the output device supports.
///
/// It is required that all output devices support at least 80x25 text mode. This mode is defined
/// to be mode 0. If the output devices support 80x50, that is defined to be mode 1. All other
/// text dimensions supported by the device will follow as modes 2 and above. If 80x50 is not
/// supported, but additional modes are supported, then querying for mode 1 will return
/// [`Status::UNSUPPORTED`].
pub type QueryMode = unsafe extern "efiapi" fn(
    this: *mut SimpleTextOutputProtocol,
    mode_number: usize,
    columns: *mut usize,
    rows: *mut usize,
) -> Status;
/// Sets the output device to the requested mode. On success the device is in the geometry for the
/// requested mode and the device has been cleared to the current background color with the cursor
/// at `(0, 0`.
pub type SetMode =
    unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol, mode_number: usize) -> Status;
/// Sets the background and foreground color for [`SimpleTextOutputProtocol::output_string`]
/// and [`SimpleTextOutputProtocol::clear_screen`].
///
/// The colors can be set even when the device is in an invalid mode.
pub type SetAttribute =
    unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol, attribute: usize) -> Status;
/// Clears the output device's display to the currently selected background color. The cursor is
/// set to `(0, 0)`.
pub type ClearScreen = unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol) -> Status;
/// Sets the current coordinates of the cursor position.
pub type SetCursorPosition = unsafe extern "efiapi" fn(
    this: *mut SimpleTextOutputProtocol,
    column: usize,
    row: usize,
) -> Status;
/// Enables or disables the cursor's visibility.
pub type EnableCursor =
    unsafe extern "efiapi" fn(this: *mut SimpleTextOutputProtocol, visible: Boolean) -> Status;

/// Useful information about the [`SimpleTextOutputProtocol`] output device state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleTextOutputMode {
    /// The max number of modes supported by [`SimpleTextOutputProtocol::query_mode`] and
    /// [`SimpleTextOutputProtocol::set_mode`].
    pub max_mode: i32,

    /// The current mode of the [`SimpleTextOutputProtocol`] output device.
    pub mode: i32,
    /// The current character output attribute.
    pub attribute: i32,
    /// The cursor's column.
    pub cursor_column: i32,
    /// The cursor's row.
    pub cursor_row: i32,
    /// Whether the cursor is visible or not.
    pub cursor_visible: Boolean,
}

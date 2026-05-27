//! Definitions related to UEFI Runtime Services.

use core::ffi;

use crate::{
    data_type::{Boolean, Char16, Guid, Status},
    memory::MemoryDescriptor,
    table::TableHeader,
};

/// The signature located in [`TableHeader`] that indicates that the UEFI table is a UEFI Runtime
/// Services Table.
pub const SIGNATURE: u64 = 0x56524553544e5552;

/// Contains function pointers for all of the Runtime Services defined up to and including UEFI
/// 1.0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct RuntimeServices1_0 {
    /// Header of [`RuntimeServices1_0`].
    pub header: TableHeader,

    //
    // Time Services
    //
    /// Returns the current time and data information along with the timekeeping capabilities of
    /// the hardware platform.
    pub get_time: GetTime,
    /// Sets the current time and data information.
    pub set_time: SetTime,
    /// Returns the current wakeup alarm clock setting.
    pub get_wakeup_time: GetWakeupTime,
    /// Sets the system wakeup alarm clock time.
    pub set_wakeup_time: SetWakeupTime,

    //
    // Virtual Memory Services
    //
    /// Changes the runtime addressing mode of UEFI firmware from physical to virtual.
    pub set_virtual_address_map: SetVirtualAddressMap,
    /// Determines the new virtual address that is to be used on subsequent memory accesses.
    pub convert_pointer: ConvertPointer,

    //
    // Variable Services
    //
    /// Returns the value of a variable.
    pub get_variable: GetVariable,
    /// Enumerates the current variable names.
    pub get_next_variable_name: GetNextVariableName,
    /// Sets the value of a variable. This service can be used to create, update, or delete a
    /// variable.
    pub set_variable: SetVariable,

    //
    // Miscellaneous Services
    //
    /// Returns the next high 32-bits of the platform's monotonic counter.
    pub get_next_high_monotonic_count: GetNextHighMonotonicCount,
    /// Resets the entire platform.
    pub reset_system: ResetSystem,
}

/// Contains function pointers for all of the Runtime Services defined up to and including UEFI
/// 2.0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct RuntimeServices2_0 {
    /// Runtime Services defined in UEFI 1.0.
    pub v1_0: RuntimeServices1_0,

    /// Passes capsules to the firmware with both virtual and physical mappings. Depending on the
    /// intended consumption, the firmware may process the capsule immediately or wait until system
    /// reset.
    ///
    /// If the payload should persist across a system reset, the reset value returned from
    /// [`RuntimeServices2_0::query_capsule_capabilities`] must be passed into
    /// [`RuntimeServices1_0::reset_system`] and will cause the capsule to be processed by the
    /// firmware as part of the reset process.
    pub update_capsule: UpdateCapsule,
    /// Returns if the capsule can be supported via [`RuntimeServices2_0::update_capsule`].
    pub query_capsule_capabilities: QueryCapsuleCapabilities,

    /// Returns information about the UEFI variables.
    pub query_variable_info: QueryVariableInfo,
}

/// Returns a time that was valid sometime during the call to the function.
///
/// While the returned `time` contains `time_zone` and daylight savings time information, the
/// actual clock does not maintain these. Instead, those values are the values that were last set
/// via [`RuntimeServices1_0::set_time`].
///
/// During runtime, if a PC-AT CMOS device is presentin the platform, the caller must synchronize
/// access to the device before calling [`RuntimeServices1_0::get_time`].
pub type GetTime =
    unsafe extern "efiapi" fn(time: *mut Time, capabilities: *mut TimeCapabilities) -> Status;
/// Sets the real time clock device to the supplied `time` and records the current time zone and
/// daylight savings time information.
///
/// During runtime, if a PC-AT CMOS device is present in the platform, the caller must synchronize
/// access to the device before calling [`RuntimeServices1_0::set_time`].
pub type SetTime = unsafe extern "efiapi" fn(time: *mut Time) -> Status;
/// The alarm clock time may be rounded from the set alarm clock time to be within the resolution
/// of the alarm clock device, which is defined to be one second.
///
/// `pending` is [`Boolean::TRUE`] if the alarm signal is pending and requires acknowledgement.
///
/// During runtime, if a PC-AT CMOS device is present in the platform, the caller must synchronize
/// access to the device before calling [`RuntimeServices1_0::set_time`].
pub type GetWakeupTime = unsafe extern "efiapi" fn(
    enabled: *mut Boolean,
    pending: *mut Boolean,
    time: *mut Time,
) -> Status;
/// Sets whether the wakeup alarm is enabled and if it is enabled, programs the system to wakeup
/// or power on at the set time. When the alarm fires, the alarm signal is latched until it is
/// acknowledged by calling [`RuntimeServices1_0::set_wakeup_time`] to disable the alarm.
///
/// For an ACPI-aware operating system, this function only handles programming the wakeup alaram
/// for the desired wakeup time. The operating system still controls the wakeup event as it
/// normally would through the ACPI Power Management registers.
///
/// During runtime, if a PC-AT CMOS device is presentin the platform, the caller must synchronize
/// access to the device before calling [`RuntimeServices1_0::set_time`].
pub type SetWakeupTime = unsafe extern "efiapi" fn(enable: Boolean, time: *mut Time) -> Status;
/// This function can only be called at runtime. All [`Event`][e]s of
/// [`EventType::SIGNAL_VIRTUAL_ADDRESS_CHANGE`][etsvac] must be signaled before this function
/// returns.
///
/// This function changes the address of the runtime components of the UEFI firmware to the new
/// virtual address supplied in the `virtual_map`. The supplied `virtual_map` must provide a new
/// virtual address for every entry in the memory map provided at
/// [`BootServices::exit_boot_services`][bsebs] that is marked as being needed for runtime usage.
///
/// All virtual address fields in the `virtual_map` must be aligned on 4KiB boundaries. The call to
/// [`RuntimeServices1_0::set_virtual_address_map`] must be done with the physical mappings. On
/// successful return from this function, the system must then make any future calls with the newly
/// assigned virtual mappings.
///
/// [e]: crate::data_type::Event
/// [etsvac]: crate::table::boot::EventType::SIGNAL_VIRTUAL_ADDRESS_CHANGE
/// [bsebs]: crate::table::boot::BootServices1_0::exit_boot_services
pub type SetVirtualAddressMap = unsafe extern "efiapi" fn(
    memory_map_size: usize,
    descriptor_size: usize,
    descriptor_version: u32,
    virtual_map: *mut MemoryDescriptor,
) -> Status;
/// Updates the current pointer pointed to by `address` to be the proper value for the new address
/// map. All pointers the component has allocated or assigned must be updated.
///
/// Used by a UEFI component during the [`RuntimeServices1_0::set_virtual_address_map`] operation.
pub type ConvertPointer =
    unsafe extern "efiapi" fn(debug_disposition: usize, address: *mut *mut ffi::c_void) -> Status;
/// Returns the variable associatd with the `(vendor_guid, variable_name)` pair, as well as the
/// variable's [`VariableAttributes`].
pub type GetVariable = unsafe extern "efiapi" fn(
    variable_name: *mut Char16,
    vendor_guid: *mut Guid,
    attributes: VariableAttributes,
    data_size: *mut usize,
    data: *mut ffi::c_void,
) -> Status;
/// Returns the next variable in the sequence.
///
/// On each call to this function, the previous results are passed into the interface, and on
/// output the function returns the next variable name data. Once the entire variable list has
/// been returned, the error [`Status::NOT_FOUND`] is returned.
pub type GetNextVariableName = unsafe extern "efiapi" fn(
    variable_name_size: *mut usize,
    variable_name: *mut u16,
    vendor_guid: *mut Guid,
) -> Status;
/// If a variable with matching `vendor_guid`, `variable_name`, and attributes already exists, its
/// value is updated.
///
/// If a variable with matching `vendor_guid`, `variable_name`, and attributes does not already
/// exist, its value is created if `data` is not NULL. Otherwise, the variable is deleted.
pub type SetVariable = unsafe extern "efiapi" fn(
    variable_name: *const u16,
    vendor_guid: *const Guid,
    attributes: VariableAttributes,
    data_size: usize,
    data: *const ffi::c_void,
) -> Status;
/// Returns the next high 32-bits of the platform's monotonic counter.
pub type GetNextHighMonotonicCount = unsafe extern "efiapi" fn(high_count: u32) -> Status;
/// Resets the entire platform, including all processors and devices, and reboots the system.
///
/// The NUL-terminated string followed by binary data pointed to be `reset_data` may optionally be
/// loggged.
pub type ResetSystem = unsafe extern "efiapi" fn(
    reset_type: ResetType,
    reset_status: Status,
    data_size: usize,
    reset_data: *mut ffi::c_void,
);
/// Allows the operating system to pass information to firmware. Each capsule is contained in a
/// contiguous virtual memory range in the operating system, but both a virtual and physical
/// mapping for the capsules are passed to the firmware.
///
/// The behavior of the firmware when processing the capsule is dependent on the [`CapsuleFlags`].
pub type UpdateCapsule = unsafe extern "efiapi" fn(
    capsule_header_array: *mut *mut CapsuleHeader,
    capsule_count: usize,
    scatter_gather_list: u64,
) -> Status;
/// Allows a caller to test to see if a capsule or capsules can be updated via
/// [`RuntimeServices2_0::update_capsule`].
///
/// Returns the maximum size, in bytes, that [`RuntimeServices2_0::update_capsule`] can support as
/// an argument via `capsule_header_array` and `scatter_gather_list` in `maximum_capsule_size`.
///
/// Returns the type of reset required fro the capsule update in `reset_type`.
pub type QueryCapsuleCapabilities = unsafe extern "efiapi" fn(
    capsule_header_array: *mut *mut CapsuleHeader,
    capsule_count: usize,
    maximum_capsule_size: *mut u64,
    reset_type: *mut ResetType,
) -> Status;
/// Returns the maximum size of the storage space, the remaining size of the storage space, and the
/// maximum size of an individual UEFI variable for the UEFI variables associated with the
/// [`VariableAttributes`] specified.
pub type QueryVariableInfo = unsafe extern "efiapi" fn(
    attributes: VariableAttributes,
    maximum_variable_storage_size: *mut u64,
    remaining_variable_storage_size: *mut u64,
    maximum_variable_size: *mut u64,
) -> Status;

/// A snapshot of a moment in time and various options concerning its interpretation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    /// The current year.
    pub year: u16,
    /// The current month.
    pub month: u8,
    /// The current day.
    pub day: u8,
    /// The current hour.
    pub hour: u8,
    /// The current minute.
    pub minute: u8,
    /// The current second.
    pub second: u8,
    /// Padding.
    pub _padding_0: u8,
    /// The current fraction of a second in the device.
    pub nanosecond: u32,
    /// The time's offset in minutes from UTC.
    ///
    /// If the value is [`Time::UNSPECIFIED_TIMEZONE`], the time should be interpreted as local
    /// time.
    pub time_zone: i16,
    /// A bitmask containing the daylight savings time information.
    pub daylight: u8,
    /// Padding.
    pub _padding_1: u8,
}

impl Time {
    /// The [`Time`] should be interpreted as a local time.
    pub const UNSPECIFIED_TIMEZONE: i16 = 0x7ff;

    /// The [`Time`] should be adjusted for daylight savings time.
    pub const ADJUST_DAYLIGHT: u8 = 0x1;
    /// The [`Time`] has been adjusted for daylight savings time.
    pub const IN_DAYLIGHT: u8 = 0x02;
}

/// The capabilities of the real-time clock used to maintain the current time and date for the
/// system.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeCapabilities {
    /// The reporting resultion of the real-time clock device in counts per second.
    pub resolution: u32,
    /// Provides the timekeeping accuracy of the real-time clock in an error rate of 10^-6 per
    /// million.
    pub accuracy: u32,
    /// Set to `true` if a time set operation clears the device's time below the
    /// [`TimeCapabilities::resolution`] threshold.
    pub sets_to_zero: Boolean,
}

/// Type information for the pointer being converted.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DebugDisposition(pub usize);

impl DebugDisposition {
    /// The pointer being converted is allowed to be NULL.
    pub const OPTIONAL_POINTER: Self = Self(0x00000001);
}

/// Various attributes that define how the variable is stored and when it can be accessed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct VariableAttributes(pub u32);

impl VariableAttributes {
    /// The variable should be stored in a non-volatile manner.
    pub const NON_VOLATILE: Self = Self(0x00000001);
    /// The variable should be queryable during UEFI Boot Services.
    pub const BOOT_SERVICES_ACCESS: Self = Self(0x00000002);
    /// The variable should be queryable during runtime.
    pub const RUNTIME_ACCESS: Self = Self(0x00000004);
    /// The variable is a hardware error record.
    pub const HARDWARE_ERROR_RECORD: Self = Self(0x00000008);

    /// Indicates that the variable is authenticated and the variable may only be updated with a
    /// higher time than the signed variable was created with.
    ///
    /// This helps prevent replay of previous updates.
    pub const TIME_BASED_AUTHENTICATED_WRITE_ACCESS: Self = Self(0x00000020);
    /// Indicates to [`RuntimeServices1_0::set_variable`] that the set operation should append
    /// instead of overwriting.
    pub const APPEND_WRITE: Self = Self(0x00000040);
    /// The variable is authenticated.
    ///
    // TODO: Improve documentation.
    pub const ENHANCED_AUTHENTICATED_ACCESS: Self = Self(0x00000080);
}

/// The type of platform reset to perform.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResetType(pub usize);

impl ResetType {
    /// Causes a system-wide reset.
    ///
    /// Should set all circuitry within the system to its initial state.
    pub const COLD: Self = Self(0);
    /// Causes a system-wide initialization.
    ///
    /// All processors are set to their initial state and pending cycles are not corrupted. If the
    /// system does not support this [`ResetType`], an [`RuntimeServices1_0::reset_system`] of
    /// [`ResetType::COLD`] must be performed.
    pub const WARM: Self = Self(1);
    /// Causes the system to enter a power state equivalent to the ACPI G2/S5 or G3 states. If the
    /// system does not support this reset type, then when the system is rebooted, it should
    /// exhibit the [`ResetType::COLD`] attributes.
    pub const SHUTDOWN: Self = Self(2);
    /// Causes a system-wide reset, with the exact type of the reset being defined by the [`Guid`]
    /// that follows the NUL-terminated string passed into `reset_data`.
    pub const PLATFORM_SPECIFIC: Self = Self(3);
}

/// The start of a contiguous set of data.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapsuleHeader {
    /// A [`Guid`] that defines the contents of the capsule.
    pub guid: Guid,
    /// The size of the capsule header. This may be larger than the size of the [`CapsuleHeader`]
    /// since [`CapsuleHeader::guid`] may imply extended header entries.
    pub header_size: u32,
    /// Various flags.
    ///
    /// This lower 16-bits are defined by [`CapsuleHeader::guid`].
    /// The upper 16-bits are defined by UEFI.
    pub flags: CapsuleFlags,
    /// The size, in bytes, of the capsule.
    pub capsule_image_size: u32,
}

/// Flags that influence the interpretation of the associated capsule.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapsuleFlags(pub u32);

impl CapsuleFlags {
    /// Indicates that the firmware should process the capsules after system reset,
    ///
    /// The caller must ensure that the system is reset using the required reset value obtained
    /// from [`RuntimeServices2_0::query_capsule_capabilities`].
    pub const PERSIST_ACROSS_RESET: Self = Self(0x00010000);
    /// Indicates that the firmware should coalesce the contents of the capsule from the
    /// `scatter_gather_list` into a contiguous buffer and then place a pointer to coalesced
    /// capsule in System Table after the system has been reset.
    ///
    /// Requires [`CapsuleFlags::PERSIST_ACROSS_RESET`] to be set.
    pub const POPULATE_SYSTEM_TABLE: Self = Self(0x00020000);
    /// Indicates that the firmware should initiate a reset of the platform which is compatible
    /// with the passed-in capsule request and will not return back to the caller.
    ///
    /// Requires [`CapsuleFlags::PERSIST_ACROSS_RESET`] to be set.
    pub const INITIATE_RESET: Self = Self(0x00040000);
}

/// Basic unit of physical address of `scatter_gather_list`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapsuleBlockDescriptor {
    /// Length, in bytes, of the data pointed to by [`CapsuleBlockDescriptor::physical_address`].
    pub length: u64,
    /// If [`CapsuleBlockDescriptor::length`] is not zero, the physical address of the data block.
    /// If [`CapsuleBlockDescriptor::length`] is zero, the physical address of another
    /// [`CapsuleHeader`] structure.
    pub physical_address: u64,
}

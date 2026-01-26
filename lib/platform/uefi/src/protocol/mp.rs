//! Definitions related to UEFI MP Services protocol.

use core::ffi;

use crate::{
    data_type::{Boolean, Event, Guid, Status},
    guid,
};

/// Used to manage multi-processor systems.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct MpServicesProtocol {
    /// Gets the number of logical processors and the number of enabled logical processors in the
    /// system.
    pub get_number_of_processors: GetNumberOfProcessors,
    /// Gets detailed information on the requested processor at the instant this call is made.
    pub get_processor_info: GetProcessorInfo,
    /// Starts up all the enabled APs in the system to run the function provided by the caller.
    pub startup_all_aps: StartupAllAps,
    /// Starts up the requested AP to run the function provided by the caller.
    pub startup_this_ap: StartupThisAp,
    /// Switches the requested AP to be the BSP from that point onward. This service changes the
    /// BSP for all purposes.
    pub switch_bsp: SwitchBsp,
    /// Enables and disables the given AP from that point onward.
    pub enable_disable_ap: EnableDisableAp,
    /// Gets the handle number of the caller processor.
    pub who_am_i: WhoAmI,
}

impl MpServicesProtocol {
    /// The [`Guid`] associated with the [`MpServicesProtocol`].
    pub const GUID: Guid = guid!("3fdda605-a76e-4f46-ad29-12f4531b3d08");
}

/// Returns the number of logical processors that are present in the system and the number of
/// enabled logical processors in the system at the instant this call is made.
pub type GetNumberOfProcessors = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    number_of_processors: *mut usize,
    number_of_enabled_processors: *mut usize,
) -> Status;

/// Gets detailed information of the requested processor at the instant this calls is made.
///
/// This service may only be called from the BSP.
pub type GetProcessorInfo = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    processor_number: *mut usize,
    processor_info_buffer: *mut ProcessorInformation,
) -> Status;

/// Various pieces of relevant information about a certain processor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ProcessorInformation {
    /// The unique processor ID determined by system hardware.
    pub processor_id: u64,
    /// Flags indicating the health and status of the processor.
    pub status_flag: StatusFlag,
    /// The physical location of the procesor.
    pub location: CpuPhysicalLocation,
    /// The extended information of the processor.
    pub extended_information: ExtendedProcessorInformation,
}

/// Flags indicating the health and status of the processor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct StatusFlag(pub u32);

impl StatusFlag {
    /// If this bit is enabled, then the processor is the BSP.
    pub const BSP: Self = Self(0x1);
    /// If this bit is enabled, then the processor is enabled.
    pub const ENABLED: Self = Self(0x2);
    /// If this bit is enabled, then the processor is healthy. Otherwise, some fault has been
    /// detected for the processor.
    pub const HEALTH_STATUS: Self = Self(0x4);
}

/// The physical location of the processor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct CpuPhysicalLocation {
    /// Zero-based physical package number that identifies the cartridge of the processor.
    pub package: u32,
    /// Zero-based physical core number within package of the processor.
    pub core: u32,
    /// Zero-based logical thread number within core of the processor.
    pub thread: u32,
}

/// A 6-level version of [`CpuPhysicalLocation`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ExtendedProcessorInformation {
    /// Zero-based physical package number that identifies the cartridge of the processor.
    pub package: u32,
    /// Zero-based physical module number within the package of the processor.
    pub module: u32,
    /// Zero-based physical tile number within the module of the processor.
    pub tile: u32,
    /// Zero-based physical die number within the tile of the processor..
    pub die: u32,
    /// Zero-based physical core number within package of the processor.
    pub core: u32,
    /// Zero-based logical thread number within core of the processor.
    pub thread: u32,
}

/// Executes a caller provided function on all enabled APs. The APs can either run simultaneously
/// or one at a time in a sequence. This service supports both blocking and non-blocking requests.
///
/// If `single_thread` is `true`, then all enabled APs execute the function specified by
/// `procedure` one by one, in ascending order of processor handle number. Otherwise, all enabled
/// APs execute the function specified by `procedure` simultaneously.
pub type StartupAllAps = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    procedure: ApProcedure,
    single_thread: Boolean,
    wait_event: Event,
    timeout_in_microseconds: usize,
    procedure_argument: *mut ffi::c_void,
    failed_cpu_list: *mut *mut usize,
) -> Status;

/// The function prototype of functions that may be passed to APs to execute.
pub type ApProcedure = unsafe extern "efiapi" fn(arg: *mut ffi::c_void);

/// Dispatches one enabled AP to the function provided by `procedure` passing in the
/// `procedure_argument`.
pub type StartupThisAp = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    procedure: ApProcedure,
    processor_number: usize,
    wait_event: Event,
    timeout_in_microseconds: usize,
    procedure_argument: *mut ffi::c_void,
    finished: *mut Boolean,
) -> Status;

/// Switches the requested AP to be the BSP from that point onward. This service changes the BSP
/// for all purposes and may only be called from the current BSP.
///
/// The new BSP can take over the execution of the old BSP and continue seemlessly from where the
/// old one left off.
pub type SwitchBsp = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    processor_number: usize,
    enable_old_ap: Boolean,
) -> Status;

/// Allows the caller to enable or disable an AP from this point onwawrd. The caller can optionally
/// specify the health status of the AP using `health_flag`.
///
/// If an AP is being disabled, then the state of the disabled AP is implementation dependent. If
/// an AP is enabled, then the implementation must guarantee that a complete initialization
/// sequence is performed on the AP, so the AP is in a state that is compatible with an MP
/// operating system.
pub type EnableDisableAp = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    processor_number: usize,
    enable_ap: Boolean,
    health_flag: *const u32,
) -> Status;

/// Returns the processor handle number for the calling processor. The returned value is the range
/// 0 to the toal number of logical processors minus 1.
///
/// This function may be called from the BSP and APs.
pub type WhoAmI = unsafe extern "efiapi" fn(
    this: *mut MpServicesProtocol,
    processor_number: *mut usize,
) -> Status;

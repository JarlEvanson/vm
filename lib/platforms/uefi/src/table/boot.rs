//! Definitions related to UEFI Boot Services.

use core::{ffi, ops};

use crate::{
    data_type::{Boolean, Char16, Event, Guid, Handle, Status, TaskPriorityLevel},
    guid,
    memory::{MemoryDescriptor, MemoryType},
    protocol::device_path::DevicePathProtocol,
    table::TableHeader,
};

/// The signature located in [`TableHeader`] that indicates that the UEFI table is a UEFI Boot
/// Services Table.
pub const SIGNATURE: u64 = 0x56524553544f4f42;

/// Contains function pointers for all of the Boot Services defined up to and including UEFI 1.0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct BootServices1_0 {
    /// Header of [`BootServices1_0`].
    pub header: TableHeader,

    //
    // Task Priority Services
    //
    /// Raises a task's priority level and returns its previous level.
    pub raise_tpl: RaiseTpl,
    /// Restores a task's priority level to its previous value.
    pub restore_tpl: RestoreTpl,
    //
    // Memory Services
    //
    /// Allocates memory pages from the system.
    pub allocate_pages: AllocatePages,
    /// Frees memory pages.
    pub free_pages: FreePages,
    /// Returns the current memory map.
    pub get_memory_map: GetMemoryMap,
    /// Allocates pool memory.
    pub allocate_pool: AllocatePool,
    /// Returns pool mmeory to the system.
    pub free_pool: FreePool,

    //
    // Event & Timer Services
    //
    /// Creates an [`Event`].
    pub create_event: CreateEvent,
    /// Sets the type of timer and the trigger time for a timer [`Event`].
    pub set_timer: SetTimer,
    /// Stops execution until an [`Event`] is signaled.
    pub wait_for_event: WaitForEvent,
    /// Signals an [`Event`].
    pub signal_event: SignalEvent,
    /// Closes an [`Event`].
    pub close_event: CloseEvent,
    /// Checks whether an event is in the signaled state.
    pub check_event: CheckEvent,

    //
    // Protocol Handler Services
    //
    /// Installs a protocol interface on a device [`Handle`]. If the [`Handle`] does not exist, it
    /// is created and added to the list of [`Handle`]s in the system.
    pub install_protocol_interface: InstallProtocolInterface,
    /// Reinstalls a protocol interface on a device [`Handle`].
    pub reinstall_protocol_interface: ReinstallProtocolInterface,
    /// Removes a protocol interface from a device handle.
    pub uninstall_protocol_interface: UninstallProtocolInterface,
    /// Queries a [`Handle`] to determine if it supports a specified protocol.
    pub handle_protocol: HandleProtocol,
    /// Reserved.
    pub _reserved: *mut ffi::c_void,
    /// Creates an [`Event`] that is to be signaled whenever an interface is installed for a
    /// specified protocol.
    pub register_protocol_notify: RegisterProtocolNotify,
    /// Returns an array of [`Handle`]s that support a specified protocol.
    pub locate_handle: LocateHandle,
    /// Locates the [`Handle`] to a device on the device path that supports the specified protocol.
    pub locate_device_path: LocateDevicePath,
    /// Adds, updates, or removes a configuration table entry from the UEFI system table.
    pub install_configuration_table: InstallConfigurationTable,

    //
    // Image Services
    //
    /// Loads an UEFI image into memory.
    pub load_image: LoadImage,
    /// Transfers control to a loaded image's entry point.
    pub start_image: StartImage,
    /// Terminates a loaded UEFI image and returns control to [`BootServices1_0`].
    pub exit: Exit,
    /// Unloads an image.
    pub unload_image: UnloadImage,
    /// Terminates all boot services.
    pub exit_boot_services: ExitBootServices1_0,

    //
    // Miscellaneous Services
    //
    /// Returns a monotonically increasing count for the platform.
    pub get_next_monotonic_count: GetNextMonotonicCount,
    /// Induces a fine-grained stall.
    pub stall: Stall,
    /// Sets the system's watchdog timer.
    pub set_watchdog_timer: SetWatchdogTimer,
}

/// Contains function pointers for all of the Boot Services defined up to and including UEFI 1.1.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct BootServices1_1 {
    /// Boot Services defined in UEFI 1.0.
    pub v1_0: BootServices1_0,

    //
    // Driver Support Services
    //
    /// Connects one or more drivers to a controller.
    pub connect_controller: ConnectController,
    /// Disconnects one or more drivers from a controller.
    pub disconnect_controller: DisconnectController,

    //
    // Open & Close Protocol Services
    //
    /// Queries a handle to determine if it supports a specified protocol, if the protocol is
    /// supported by the handle, it opens the protocol on behalf of the calling agent.
    pub open_protocol: OpenProtocol,
    /// Closes a protocol on a handle that was opened using [`BootServices1_1::open_protocol`].
    pub close_protocol: CloseProtocol,
    /// Returns a list of agents that currently have a protocol interface open.
    pub open_protocol_information: OpenProtocolInformation,

    //
    // Library Services
    //
    /// Retrives the list of protocol interface [`Guid`]s that are installed on a handle in a
    /// buffer allocated from pool memory.
    pub protocols_per_handle: ProtocolsPerHandle,
    /// Returns an array of [`Handle`]s that support the requested protocol in a buffer allocated
    /// from pool memory.
    pub locate_handle_buffer: LocateHandleBuffer,
    /// Returns the first instance that matches the given protocol.
    pub locate_protocol: LocateProtocol,
    /// Installs one or more protocol interfaces into the boot services environment.
    pub install_multiple_protocol_interface: InstallMultipleProtocolInterfaces,
    /// Uninstalls one or more protocol interfaces from the boot services environment.
    pub uninstall_multiple_protocol_interfaces: UninstallMultipleProtocolInterfaces,

    //
    // Miscellaneous Services
    //
    /// Computes and returns a 32-bit CRC for a data buffer.
    pub calculate_crc32: CalculateCrc32,
    /// Copies the contents of one buffer to another buffer.
    pub copy_mem: CopyMem,
    /// Fills a buffer with a specified value.
    pub set_mem: SetMem,
}

/// Contains function pointers for all of the Boot Services defined up to and including UEFI 1.1.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash)]
pub struct BootServices2_0 {
    /// Boot Services defined in UEFI 1.1 or lower.
    pub v1_1: BootServices1_1,

    //
    // Event & Timer Services
    //
    /// Creates an event in a group.
    pub create_event_ex: CreateEventEx,
}

/// Raises a task's priority level and returns its previous level.
///
/// The caller must restore the [`TaskPriorityLevel`] with [`BootServices1_0::restore_tpl`] to the
/// previous level before returning.
pub type RaiseTpl = unsafe extern "efiapi" fn(new_tpl: TaskPriorityLevel) -> TaskPriorityLevel;
/// Restores a task's priority level to its previous value.
///
/// Calls to [`RestoreTpl`] should be matched with calls to [`RaiseTpl`].
pub type RestoreTpl = unsafe extern "efiapi" fn(old_tpl: TaskPriorityLevel);

/// Allocates the requested number of pages and returns a pointer to the base address of the page
/// range in the location referenced by `memory`.
///
/// The function scans the memory map to located free pages. When it finds a contiguous block of
/// pages that is large enough and also satisfies the allocation requirements of `allocate_type`,
/// it changes the memory map to indicate that the pages are now of type `memory_type`.
pub type AllocatePages = unsafe extern "efiapi" fn(
    allocate_type: AllocateType,
    memory_type: MemoryType,
    pages: usize,
    memory: *mut u64,
) -> Status;
/// Returns memory allocated by [`BootServices1_0::allocate_pages`] to the firmware.
pub type FreePages = unsafe extern "efiapi" fn(memory: u64, pages: usize) -> Status;
/// Returns a copy of the current memory map.
///
/// If the `memory_map_size` is too small, [`Status::BUFFER_TOO_SMALL`] is returned and
/// `memory_map_size` contains the size of the buffer required to contain the current memory map.
///
/// On success, a `map_key` is returned that identifies the current memory map. [`GetMemoryMap`]
/// also returns the `descriptor_size` and `descriptor_version`. The `descriptor_size` represents
/// the size, in bytes, of a [`MemoryDescriptor`] in the memory map.
///
/// The map is an array of [`MemoryDescriptor`]s, each of which describes a contiguous block of
/// memory. The map describes all of memory, no matter how it is being used. The memory map is only
/// used to describe memory that is present in the system and does not return a
/// [`MemoryDescriptor`] for address space regions that are not backed by physical hardware.
///
/// Regions that are backed by physical hardware, but are not supposed to be accessed by the OS,
/// must be returned as [`MemoryType::RESERVED`].
pub type GetMemoryMap = unsafe extern "efiapi" fn(
    memory_map_size: *mut usize,
    memory_map: *mut MemoryDescriptor,
    map_key: *mut usize,
    descriptor_size: *mut usize,
    descriptor_version: *mut u32,
) -> Status;
/// Allocates a memory region of `size` bytes from the [`MemoryType`] `pool_type` and returns the
/// address of the allocated memory in the location referenced by `buffer`.
///
/// All allocations are 8-byte aligned.
///
/// The allocated pool memory is returned to the available pool with
/// [`BootServices1_0::free_pool`].
pub type AllocatePool = unsafe extern "efiapi" fn(
    pool_type: MemoryType,
    size: usize,
    buffer: *mut *mut ffi::c_void,
) -> Status;
/// Returns the memory specified by `buffer` to the system.
///
/// The buffer that is freed must have been allocated by [`BootServices1_0::allocate_pool`].
pub type FreePool = unsafe extern "efiapi" fn(buffer: *mut ffi::c_void) -> Status;

/// Creates a new [`Event`] of `event_type` and returns it in the location referenced by `event`.
/// The [`Event`]'s notification function, context, and [`TaskPriorityLevel`] are specified.
pub type CreateEvent = unsafe extern "efiapi" fn(
    event_type: u32,
    notify_tpl: TaskPriorityLevel,
    notify_function: Option<EventNotify>,
    notify_context: *mut ffi::c_void,
    event: *mut Event,
) -> Status;
/// Cancels any previous time trigger setting for the event and sets a new trigger time for the
/// event.
///
/// `trigger_time` is in units of 100ns.
pub type SetTimer =
    unsafe extern "efiapi" fn(event: Event, timer_type: TimerType, trigger_time: u64) -> Status;
/// Stops execution until an event in the list of [`Event`]s is signaled.
///
/// On success, `index` indicates the event that was signaled.
pub type WaitForEvent = unsafe extern "efiapi" fn(
    number_of_events: usize,
    event: *mut Event,
    index: *mut usize,
) -> Status;
/// The supplied [`Event`] is placed in the signaled state.
///
/// If the [`Event`] is already in the signaled state, then [`Status::SUCCESS`] is returned.
/// If the event is of type [`EventType::NOTIFY_SIGNAL`], then the [`Event`]'s notification
/// function is scheduled to be invoked.
///
/// If the supplied [`Event`] is part of an [`EventGroup`], then all of the [`Event`]s in the
/// [`EventGroup`] are also signaled and their notification functions scheduled.
pub type SignalEvent = unsafe extern "efiapi" fn(event: Event) -> Status;
/// Removes the caller's reference to the [`Event`], removes it from any [`EventGroup`] to which it
/// belongs, and closes it. Once the [`Event`] is closed, the event is no longer valid and may not
/// be used on any subsequent function calls.
///
/// It is safe to call [`BootServices1_0::close_event`] within the [`Event`]'s notification
/// function.
pub type CloseEvent = unsafe extern "efiapi" fn(event: Event) -> Status;
/// Checks to see whether [`Event`] is in the signaled state.
///
/// If the [`Event`] is of type [`EventType::NOTIFY_SIGNAL`], then [`Status::INVALID_PARAMETER`] is
/// returned. Otherwise, one of three options occurs.
///
/// - If the [`Event`] is in the signaled state, it is cleared and [`Status::SUCCESS`] is returned.
/// - If the [`Event`] is not in the signaled state and has no notification function,
///   [`Status::NOT_READY`] is returned.
/// - If the [`Event`] is not in the signaled state and has a notification function, the
///   notification function is queued. If the execution of the notification function causes
///   [`Event`] to be signaled, then the signaled state is cleared and [`Status::SUCCESS`] is
///   returned. Otherwise, [`Status::NOT_READY`] is returned.
pub type CheckEvent = unsafe extern "efiapi" fn(event: Event) -> Status;
/// Installs a protocol interface (a [`Guid`]/Protocol Interface structure pair) onto a device
/// handle.
///
/// The same [`Guid`] cannot be installed more than one onto the same [`Handle`].  If the provided
/// [`Option<Handle>`] is [`None`] on input, a new [`Handle] is created and returned on output.
///
/// When a protocol interface is installed, the firmware calls all notification functions that have
/// registered to wait for the installation of `protocol`.
pub type InstallProtocolInterface = unsafe extern "efiapi" fn(
    handle: *mut Handle,
    protocol: *const Guid,
    interface_type: InterfaceType,
    interface: *mut ffi::c_void,
) -> Status;
/// Reinstalls a protocol interface on a [`Handle`]. The `old_interface` for `protocol` is replaced
/// by `new_interface`.
///
/// As with [`BootServices1_0::install_protocol_interface`], any process that has registered to
/// wait for the installation of the interface is notified.
///
/// The caller is responsible for ensuring that there are no references to a protocol interface
/// that has been removed.
///
/// ### EFI 1.10 Extension
///
/// The caller is no longer responsible for ensuring that there are no references to a protocol
/// interface.
pub type ReinstallProtocolInterface = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: Guid,
    old_interface: *mut ffi::c_void,
    new_interface: *mut ffi::c_void,
) -> Status;
/// Removes a protocol interface from the [`Handle`] on which it was previously installed.
///
/// If the last protocol interface is removed from a handle, the [`Handle`] is freed and no longer
/// valid.
///
/// The caller is responsible for ensuring that there are no references to a protocol interface
/// that has been removed.
///
/// ### EFI 1.10 Extension
///
/// The caller is no longer responsible for ensuring that there are no references to a protocol
/// interface.
pub type UninstallProtocolInterface = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: *const Guid,
    interface: *const ffi::c_void,
) -> Status;
/// Queries the [`Handle`] to determine if it supports `protocol`. If it does, then it on return
/// `interface` points to a pointer to the corresponding protocol interface.
pub type HandleProtocol = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: *const Guid,
    interface: *mut *mut ffi::c_void,
) -> Status;
/// Creates an [`Event`] that is to be signaled whenever a protocol interface is installed for
/// `protocol`.
///
/// Once `event` has been signaled, [`BootServices1_0::locate_handle`] may be called to identify
/// the newly installed or reinstalled [`Handle`]'s that support `protocol`. `registration`
/// corresponds to `search_key` in [`BootServices1_0::locate_handle`].
pub type RegisterProtocolNotify = unsafe extern "efiapi" fn(
    protocol: *const Guid,
    event: Event,
    registration: *mut *mut ffi::c_void,
) -> Status;
/// Returns an array of [`Handle`]s that match the [`SearchType`].
///
/// If the size of the provided buffer is too small, the [`Status::BUFFER_TOO_SMALL`] is returned
/// and `buffer_size` contains the size of the buffer required to obtain the array.
pub type LocateHandle = unsafe extern "efiapi" fn(
    search_type: SearchType,
    protocol: *const Guid,
    search_key: *const ffi::c_void,
    buffer_size: *mut usize,
    buffer: *mut Handle,
) -> Status;
/// Locates all the devices on `device_path` that support `protocol` and returns the [`Handle`] to
/// the device that is closest to `device_path`.
///
/// `device_path` is advanced over the device path nodes that were matched.
pub type LocateDevicePath = unsafe extern "efiapi" fn(
    protocol: *const Guid,
    device_path: *mut DevicePathProtocol,
    device: *mut Handle,
) -> Status;
/// Manages the list of configuration tables that are stored in the UEFI system table.
///
/// The list is allocated from pool memory of [`MemoryType::RUNTIME_SERVICES_DATA`].
///
/// If `guid` is NULL, [`Status::INVALID_PARAMETER`] is returned. Otherwise, there are four
/// options:
///
/// - If `guid` is not present in the system table and `table` is not NULL, then the pair is added
///   list.
/// - If `guid` is not present in the system table and `table` is NULL, then [`Status::NOT_FOUND`]
///   is returned.
/// - If `guid` is present in the system table and `table` is not NULL, then the pair is updated
///   with the new `table` value.
/// - If `guid` is present in the system table and `table` is NULL, then the entry associated with
///   `guid` is removed from the system table.
pub type InstallConfigurationTable =
    unsafe extern "efiapi" fn(guid: *const Guid, table: *mut ffi::c_void) -> Status;
/// Loads an UEFI image into memory and returns a [`Handle`] to the image.
///
/// If `source_buffer` is not NULL, then the function is a memory-to-memory load in which
/// `source_buffer` points to the image to be loaded and `source_size` indicates the image's size
/// in bytes. After this image is loaded, then `source_buffer` can be freed by the caller.
/// `device_path` is optional in this case.
///
/// If `source_buffer` is NULL, the function is a file copy operation.
pub type LoadImage = unsafe extern "efiapi" fn(
    boot_policy: Boolean,
    parent_image_handle: Handle,
    device_path: *mut DevicePathProtocol,
    source_buffer: *mut ffi::c_void,
    source_size: usize,
    image_handle: *mut Handle,
) -> Status;
/// Transfers control to the entry point of an image loaded by [`BootServices1_0::load_image`]. The
/// image may only be started a single time.
///
/// Control returns from [`BootServices1_0::start_image`] when the loaded image's entry point
/// returns or the image calls [`BootServices1_0::exit`]. When `BootServices::exit`] is called, the
/// `exit_data` buffer and `exit_data_size` from [`BootServices1_0::exit`] are passed back through
/// `exit_data` and `exit_data_size` in this function.
///
/// The caller is responsible for deallocating `exit_data` when the the buffer is no longer needed.
pub type StartImage = unsafe extern "efiapi" fn(
    image_handle: Handle,
    exit_data_size: *mut usize,
    eixt_data: *mut Char16,
) -> Status;
/// Terminates the image referenced by [`Handle`] and returns control to [`BootServices1_0`]. This
/// function may not be called if the image has already returned from its entry point or if it has
/// loaded any child images that have not exited.
///
/// When an application exits a compliant system, firmware frees the memory used to hold the image.
/// The firmware also frees its references to `image_handle` and the [`Handle`] itself. Before
/// exiting, the application is responsible for free any resources it allocated, including memory
/// and open [`Handle`]s. The only exception is the `exit_data` buffer. If the `exit_data` buffer
/// is returned, it must have been allocated by a call to [`BootServices1_0::allocate_pool`].
///
/// When an UEFI boot service or runtime service driver exits, the firmware frees the image only if
/// the `exit_status` is an error code. The driver must not return an error code if it hash
/// installed any protocol handlers or other active callbacks that have not been cleaned up. If a
/// driver exits with an error code, it is responsible for free all resources before exiting.
pub type Exit = unsafe extern "efiapi" fn(
    image_handle: Handle,
    exit_status: Status,
    exit_data_size: usize,
    exit_data: *mut Char16,
) -> Status;
/// Unloads an image previously loaded by [`BootServices1_0::load_image`].
///
/// If the image is not started, the the function unloads the image and returns
/// [`Status::SUCCESS`].
///
/// If the image has been started and has an unload function, control is passed to that entry
/// point. If the function returns [`Status::SUCCESS`], the image is unloaded. Otherwise, the error
/// returned by the image's unload is returned. to the caller.
///
/// If the image is started and does not have an unload function, the function returns
/// [`Status::UNSUPPORTED`].
pub type UnloadImage = unsafe extern "efiapi" fn(image_handle: Handle) -> Status;
/// Terminates all boot services.
///
/// On success, the UEFI application becomes responsible for the continued operation of the system.
/// All [`Event`]s from [`EventGroup::BEFORE_EXIT_BOOT_SERVICES`] and
/// [`EventGroup::EXIT_BOOT_SERVICES`] as well as [`Event`]s of
/// [`EventType::SIGNAL_EXIT_BOOT_SERVICES`] must be signaled before
/// [`BootServices1_0::exit_boot_services`] returns [`Status::SUCCESS`].
///
/// After the first call to [`BootServices1_0::exit_boot_services`], an application should not make
/// calls to any boot service function other than Memory Allocation Services.
///
/// On success, the application owns all available memory in the system. No further calls to boot
/// service functions or UEFI protocols may be used and the boot services watchdog timer is
/// disabled. Several fields of the UEFI system table must be set to NULL.
pub type ExitBootServices1_0 =
    unsafe extern "efiapi" fn(image_handle: Handle, map_key: usize) -> Status;
/// Returns a 64-bit value that is numerically larger than the last time this function was called.
///
/// The platform's monotonic counter is comprised on two parts. The high 32 bits, which is
/// increased by one whenever the system resets of the low 32 bit counter overflow, and the low
/// 32 bits, which is volatile and reset to zero on every system reset.
pub type GetNextMonotonicCount = unsafe extern "efiapi" fn(count: *mut u64) -> Status;
/// Stalls execution on the processor for at least the requested number of microseconds.
///
/// Execution of the processor is not yielded for the duration of the stall.
pub type Stall = unsafe extern "efiapi" fn(microseconds: usize) -> Status;
/// Sets the system's watchdog timer.
///
/// If the watchdog timer expires, the event is logged by the firmware and the platform must
/// eventually be reset. The watchdog timer is armed before the firmware's boot manager invokes an
/// UEFI boot option and is set to a period of 5 minutes. If control is returned to the firmware's
/// boot manager, the watchdog timer must be disabled.
pub type SetWatchdogTimer = unsafe extern "efiapi" fn(
    timeout: usize,
    watchdog_code: u64,
    data_size: usize,
    watchdog_data: *const Char16,
) -> Status;
/// Connects one or more drivers to the controller specified by `controller_handle`.
///
// TODO: Improve documentation
pub type ConnectController = unsafe extern "efiapi" fn(
    controller_handle: Handle,
    driver_image_handle: *mut Handle,
    remaining_device_path: *mut DevicePathProtocol,
    recursive: Boolean,
) -> Status;
/// Disconnects one or more drivers from a controller.
// TODO: Improve documentation
pub type DisconnectController = unsafe extern "efiapi" fn(
    controller_handle: Handle,
    driver_image_handle: Handle,
    child_handle: Handle,
) -> Status;
/// Opens a protocol interface on `handle` for `protocol`.
///
/// The agent that is opening the protocol interface is specified by `agent_handle`,
/// `controller_handle`, and `attributes`. If the protocol interface can be opened, then
/// `agent_handle`, `controller_handle`, and `attributes` are added to the list of agents that are
/// consuming the protocol interface.
///
/// In addition, the protocol interface is returnd in `interface` if `attributes` is not
/// [`OpenAttributes::TEST_PROTOCOL`]. Otherwise, `interface` is optional and can be NULL.
pub type OpenProtocol = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: *const Guid,
    interface: *mut *mut ffi::c_void,
    agent_handle: Handle,
    controller_handle: Handle,
    attributes: OpenAttributes,
) -> Status;
/// Updates the handle database to show that the protocol interface specified by `handle` and
/// `protocol` is no longer required by the agent and controller specified by `agent_handle` and
/// `controller_handle`.
pub type CloseProtocol = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: *const Guid,
    agent_handle: Handle,
    controller_handle: Handle,
) -> Status;
/// Allocates and returns a buffer of [`OpenProtocolInformationEntry`] structures.
///
/// The buffer is returned in `entry_buffer` and the number of entries is returned in
/// `entry_count`.
pub type OpenProtocolInformation = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol: *const Guid,
    entry_buffer: *mut OpenProtocolInformationEntry,
    entry_count: usize,
) -> Status;
/// Returns a list of protocol interface [`Guid`]s that are installed on [`Handle`]. This list is
/// returned in `protocol_buffer`, and the number of [`Guid`] pointers in `protocol_buffer` is
/// returned in `protocol_buffer_count`.
pub type ProtocolsPerHandle = unsafe extern "efiapi" fn(
    handle: Handle,
    protocol_buffer: *mut *mut *mut Guid,
    protocol_buffer_count: *mut usize,
) -> Status;
/// Returns one or more [`Handle`]s that match the [`SearchType`] request. The buffer is allocated
/// from pool memory, and the number of entries is returned in `number_of_handles`.
pub type LocateHandleBuffer = unsafe extern "efiapi" fn(
    search_type: SearchType,
    protocol: *const Guid,
    search_key: *const ffi::c_void,
    number_of_handles: *mut usize,
    buffer: *mut *mut Handle,
) -> Status;
/// Finds the first device [`Handle`] that supports `protocol` and returns a pointer to the
/// protocol interface from that [`Handle`] in `interface`. If no protocol interfaces are found,
/// `interface` is set to NULL.
pub type LocateProtocol = unsafe extern "efiapi" fn(
    protocol: *const Guid,
    registration: *mut ffi::c_void,
    interface: *mut *mut ffi::c_void,
) -> Status;
/// Installs a set of protocol interfaces into the boot services environment. It removes arguments
/// from the variable argument list in pairs, with the first item being a pointer to the protocol's
/// [`Guid`] and the second item being a pointer to the protocol's interface.
///
/// If `handle` is NULL on entry, a new [`Handle`] will be allocated. The pairs of arguments are
/// removed in order from the variable argument list until a NULL [`Guid`] pointer is found.
///
/// If any errors ocurr while the protocol interfaces are being installed, all protocols
/// installed prior to the error will be uninstalled.
pub type InstallMultipleProtocolInterfaces =
    unsafe extern "efiapi" fn(handle: *mut Handle) -> Status;
/// Removes a set of protocol interfaces from the boot services environment. It removes
/// arguments fom the variable argument list in pairs, with the first item being a pointer to the
/// protocol's [`Guid`] and the second item being a pointer to the protocol's interface.
///
/// If any errors occur while the protocol interfaces are being uninstalled, all protocols
/// uninstalled prior to the error will be reinstalled.
pub type UninstallMultipleProtocolInterfaces =
    unsafe extern "efiapi" fn(handle: *mut Handle) -> Status;
/// Computes the 32-bit CRC for the data buffer specified by `data` and `data_size`. If the 32-bit
/// CRC is computed, then it is returned in `crc32`.
pub type CalculateCrc32 = unsafe extern "efiapi" fn(
    data: *const ffi::c_void,
    data_size: usize,
    crc32: *mut u32,
) -> Status;
/// Copies `length` bytes from the buffer `source` to the buffer `destination`.
///
/// The implementation must be reentrant and must handle overlapping `source` and `destination`
/// buffers.
pub type CopyMem = unsafe extern "efiapi" fn(
    destination: *mut ffi::c_void,
    source: *const ffi::c_void,
    length: usize,
);
/// Fills `size` bytes of `buffer` with `value`.
///
/// The implementation must be reentrant.
pub type SetMem = unsafe extern "efiapi" fn(buffer: *mut ffi::c_void, size: usize, value: u8);
/// Creates a new [`Event`] of `event_type` and returns it in the location specified by `event`.
///
/// The event's notification function, context, and [`TaskPriorityLevel`] are specified as in
/// [`CreateEvent`], and the [`Event`] will be added to the group of events identified by
/// `event_group`.
pub type CreateEventEx = unsafe extern "efiapi" fn(
    event_type: EventType,
    notify_tpl: TaskPriorityLevel,
    notify_function: EventNotify,
    notify_context: *const ffi::c_void,
    event_group: *const Guid,
    event: *mut Event,
) -> Status;

/// The type of an allocation by [`BootServices1_0::allocate_pages`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AllocateType(pub u32);

impl AllocateType {
    /// Allocate any available range of pages that satifies the request.
    pub const ANY_PAGES: Self = Self(0);
    /// Allocate any available range of pages whose uppermost address is less than or equal to the
    /// address pointed to by `memory` on input.
    pub const MAX_ADDRESS: Self = Self(1);
    /// Allocate pages at the address pointed to by `memory` on input.
    pub const ADDRESS: Self = Self(2);
}

/// The type of [`Event`] to create and its mode and attributes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventType(pub u32);

impl EventType {
    /// When the [`Event`] is being waited on via [`BootServices1_0::wait_for_event`] or
    /// [`BootServices1_0::check_event`], the [`Event`]'s notification function will be queued if
    /// the [`Event`] is not already in the signaled state.
    pub const NOTIFY_WAIT: Self = Self(0x00000100);
    /// The [`Event`]'s notification function is queued whenever the event is signaled.
    pub const NOTIFY_SIGNAL: Self = Self(0x00000200);
    /// This [`EventType`] is functionally equivalent to the [`EventGroup::EXIT_BOOT_SERVICES`].
    ///
    /// The [`Event`] is of type [`EventType::NOTIFY_SIGNAL`] and should not be combined with any
    /// other [`EventType`]s.
    pub const SIGNAL_EXIT_BOOT_SERVICES: Self = Self(0x00000201);
    /// The [`Event`] is to be signaled when [`RuntimeServices::set_virtual_address_map`][svam] is
    /// called.
    ///
    /// The [`Event`] is of type [`EventType::NOTIFY_SIGNAL`] and [`EventType::RUNTIME`].
    ///
    /// [svam]: crate::table::runtime::RuntimeServices1_0::set_virtual_address_map
    pub const SIGNAL_VIRTUAL_ADDRESS_CHANGE: Self = Self(0x60000202);

    /// The [`Event`] should be allocated from runtime memory.
    pub const RUNTIME: Self = Self(0x40000000);
    /// The [`Event`] is a timer event and may be passed to [`BootServices1_0::set_timer`].
    pub const TIMER: Self = Self(0x80000000);
}

/// A function to handle notifications for an [`Event`].
pub type EventNotify = unsafe extern "efiapi" fn(event: Event, context: *mut ffi::c_void);

/// The type of the timer setting.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerType(pub u32);

impl TimerType {
    /// The [`Event`]'s timer setting is to be cancelled and no timer trigger is to be set.
    ///
    /// `trigger_time` is ignored.
    pub const CANCEL: Self = Self(0);
    /// The [`Event`] is to be signaled periodically at `trigger_time` intervals from the current
    /// time.
    pub const PERIODIC: Self = Self(1);
    /// The [`Event`] is to be signaled in `trigger_time`.
    pub const RELATIVE: Self = Self(2);
}

/// Indicates whether the interface is supplied in native form.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceType(pub u32);

impl InterfaceType {
    /// The interface was supplied in native form.
    pub const NATIVE: Self = Self(0);
}

/// Indicates how and for what the [`BootServices1_0::locate_handle`] should search the [`Handle`]s
/// in the system.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchType(pub u32);

impl SearchType {
    /// [`BootServices1_0::locate_handle`] returns an array of every [`Handle`] in the system.
    ///
    /// `protocol` and `search_key` are ignored for this [`SearchType`].
    pub const ALL: Self = Self(0);
    /// [`BootServices1_0::locate_handle`] returns the next [`Handle`] that is new for the
    /// registration. Only one handle is returned at a time, starting with the first, and the
    /// caller must loop until no more [`Handle`]s are returned.
    ///
    /// `protocol` is ignored for this [`SearchType`].
    pub const BY_REGISTER_NOTIFY: Self = Self(1);
    /// All [`Handle`]s that support `protocol` are returned.
    ///
    /// `search_key` is ignored for this [`SearchType`].
    pub const BY_PROTOCOL: Self = Self(2);
}

/// The open mode of the protocol interface specified by `handle` and `protocol`.
///
///
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenAttributes(pub u32);

impl OpenAttributes {
    /// Used in the implementation of [`BootServices1_0::handle_protocol`].
    pub const BY_HANDLE_PROTOCOL: Self = Self(0x00000001);
    /// Used by a driver to get a protocol interface from a [`Handle`]
    ///
    /// This is dangerous because the driver will not be informed if the protocol interface is
    /// uninstalled or reinstalled.
    ///
    /// The caller is not required to close the protocol interface with
    /// [`BootServices1_1::close_protocol`].
    pub const GET_PROTOCOL: Self = Self(0x00000002);
    /// Used by a driver to test for the existence of a protocol interface on a [`Handle`].
    ///
    /// `interface` is ignored for this attribute value and the caller should only use the
    /// returned [`Status`].
    ///
    /// The caller is not required to close the protocol interface with
    /// [`BootServices1_1::close_protocol`].
    pub const TEST_PROTOCOL: Self = Self(0x00000004);
    /// Used by bus drivers to show that a protocol interface is being used by one of the child
    /// controllers of a bus.
    ///
    /// This information is used by [`BootServices1_1::connect_controller`] to recursively connect
    /// all child controllers and by [`BootServices1_1::disconnect_controller`] to get a list of
    /// child controllers that a bus driver created.
    pub const BY_CHILD_CONTROLLER: Self = Self(0x00000008);
    /// Used by a driver to gain access to a protocol interface.
    ///
    /// When this mode is used, the driver's stop function will be called by
    /// [`BootServices1_1::disconnect_controller`] if the protocol interface is reinstalled or
    /// uninstalled.
    ///
    /// Once a protocol interface is opened by a driver with this attribute, no other drivers will
    /// be allowed to open the same protocol interface with the [`OpenAttributes::BY_DRIVER`]
    /// attribute.
    pub const BY_DRIVER: Self = Self(0x00000010);
    /// Used by applications to gain exclusive access to a protocol interface.
    ///
    /// If any drivers have the protocol interface opened with the attribute
    /// [`OpenAttributes::BY_DRIVER`], then an attempt will be made to remove them by calling the
    /// driver's stop function.
    pub const EXCLUSIVE: Self = Self(0x00000020);
}

impl ops::BitOr for OpenAttributes {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for OpenAttributes {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl ops::BitAnd for OpenAttributes {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for OpenAttributes {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl ops::BitXor for OpenAttributes {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for OpenAttributes {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl ops::Not for OpenAttributes {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

/// Information about the active agents using a [`Handle`] and protocol interface pair.
#[repr(C)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenProtocolInformationEntry {
    /// The agent that opened the protocol interface.
    pub agent_handle: Handle,
    /// The controller that opened the protocol interface.
    pub controller_handle: Handle,
    /// The attributes used to open the protocol interface.
    pub attributes: OpenAttributes,
    /// The number of times that the protocol interface has been opened by the above combination.
    pub open_count: u32,
}

/// A collection of events identified by a shared [`Guid`] which are signaled together.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventGroup(pub Guid);

impl EventGroup {
    /// A group of [`Event`]s that are notified when [`BootServices1_0::exit_boot_services`] is
    /// called right before notifying [`EventGroup::EXIT_BOOT_SERVICES`].
    ///
    /// [`Event`]s in this group must not depend on any kind of delayed procesing.
    pub const BEFORE_EXIT_BOOT_SERVICES: Self = Self(guid!("8be0e274-3970-4b44-80c5-1ab9502f3bfc"));
    /// A group of [`Event`]s that are notified when [`BootServices1_0::exit_boot_services`] is
    /// called after notifying [`EventGroup::BEFORE_EXIT_BOOT_SERVICES`].
    ///
    /// [`Event`]s in this group must not use the Memory Allocation Services or call any functions
    /// that use the Memory Allocation Services. They also must not depend on timer events.
    pub const EXIT_BOOT_SERVICES: Self = Self(guid!("27abf055-b1b8-4c26-8048-748f37baa2df"));

    /// A group of [`Event`]s that are notified when
    /// [`RuntimeServices::set_virtual_address_map`][svam] is invoked.
    ///
    /// [svam]: crate::table::runtime::RuntimeServices1_0::set_virtual_address_map
    pub const VIRTUAL_ADDRESS_CHANGE: Self = Self(guid!("13fa7698-c831-49c7-87ea-8f43fcc25196"));
    /// A group of [`Event`]s that are notified by the system when the memory map has changed.
    ///
    /// [`Event`]s in this group should not use Memory Allocation Services to avoid reentrancy
    /// complications.
    pub const MEMORY_MAP_CHANGED: Self = Self(guid!("78bee926-692f-48fd-9edb-01422ef0d7ab"));
    /// A group of [`Event`]s that are notified when the Boot Manager is about to load and execute a
    /// boot option or platform or OS recovery option and right after notifying
    /// [`EventGroup::READY_TO_BOOT`].
    ///
    /// [`Event`] in this group are the last chance to modify device or system configuration
    /// changes prior to passing control to a boot option.
    pub const READY_TO_BOOT: Self = Self(guid!("7ce88fb3-4bd7-4679-87a8-a8d8dee50d2b"));
    /// A group of [`Event`]s that are notified when the Boot Manager is about to load and execute a
    /// boot option or platform or OS recovery option and right before notifying
    /// [`EventGroup::AFTER_READY_TO_BOOT`].
    ///
    /// [`Event`] in this group are the last chance to survey device or system configuration
    /// changes prior to passing control to a boot option.
    pub const AFTER_READY_TO_BOOT: Self = Self(guid!("3a2a00ad-98b9-4cdf-a478-702777f1c10b"));
    /// A group of [`Event`]s that are notified by the system when
    /// [`RuntimeServices::reset_system`][rsrs] is invoked and the system is about to be reset.
    ///
    /// [`Event`]s in this group are only notified prior to
    /// [`BootServices1_0::exit_boot_services`].
    ///
    /// [rsrs]: crate::table::runtime::RuntimeServices1_0::reset_system
    pub const RESET_SYSTEM: Self = Self(guid!("62da6a56-13fb-485a-a8da-a3dd7912cb6b"));
}

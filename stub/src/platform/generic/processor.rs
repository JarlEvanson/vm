//! Definitions and interfaces that platforms utilize to provide services related to processor
//! management and introspection for use by the rest of the executable.

use core::ptr;

use sync::ControlledModificationCell;

/// The current [`ProcessorManager`].
static PROCESSOR_MANAGER: ControlledModificationCell<Option<&'static dyn ProcessorManager>> =
    ControlledModificationCell::new(None);

/// Initializes the processor management subsystem.
///
/// # Safety
///
/// This function must not be called when any other processor management fuction is active.
pub(in crate::platform) unsafe fn initialize_processor_management(
    manager: &'static dyn ProcessorManager,
) {
    // SAFETY:
    //
    // The invariants of [`initialize_processor_management`] ensure that this operation is safe.
    unsafe { *PROCESSOR_MANAGER.get_mut() = Some(manager) }
}

/// Returns the currently active [`ProcessorManager`].
fn processor_manager() -> &'static dyn ProcessorManager {
    PROCESSOR_MANAGER
        .get()
        .expect("processor management subsystem is uninitialized")
}

/// Returns the processor ID of the main processor (i.e., the processor on which this executable
/// booted).
pub fn main_processor_id() -> u64 {
    processor_manager().main_processor_id()
}

/// Returns the processor ID of the processor on which this function was called.
pub fn current_processor_id() -> u64 {
    processor_manager().current_processor_id()
}

/// Returns the total number of active processors.
pub fn processor_count() -> u64 {
    processor_manager().processor_count()
}

/// Executes the provided function on all processors.
///
/// All processors will return from `procedure` before this function returns.
pub fn run_on_all_processors<T: Sync>(procedure: Procedure, argument: &T) {
    run_on_all_processors_raw(procedure, ptr::from_ref(argument).cast::<()>().cast_mut())
}

/// Executes the provided function on all processors.
///
/// All processors will return from `procedure` before this function returns.
pub fn run_on_all_processors_raw(procedure: Procedure, argument: *mut ()) {
    processor_manager().run_on_all_processors(procedure, argument);
}

/// Trait representing a platform-independent mechanism for processor management.
pub(in crate::platform) trait ProcessorManager: Send + Sync {
    /// Returns the processor ID of the main processor (i.e., the processor on which this executable
    /// booted).
    fn main_processor_id(&self) -> u64;

    /// Returns the processor ID of the processor on which this function was called.
    fn current_processor_id(&self) -> u64;

    /// Returns the total number of active processors.
    ///
    /// # Implementors
    ///
    /// The value returned by this function must not change.
    fn processor_count(&self) -> u64;

    /// Executes the provided function on all processors. All CPUs must return before this function
    /// returns.
    ///
    /// This must include the boot CPU.
    fn run_on_all_processors(&self, procedure: Procedure, argument: *mut ());
}

/// The function prototype required for [`ProcessorManager::run_on_all_processors`]
pub type Procedure = extern "C" fn(cpu_id: u64, arg: *mut ());

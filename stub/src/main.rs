//! The loader for the `revm` platform.
#![no_std]
#![no_main]

use core::{error, fmt};

use sync::Spinlock;

use crate::{
    arch::interface::switch::{SwitchError, switch},
    executable::LoadExecutableError,
};

pub mod arch;
pub mod executable;
pub mod platform;
pub mod util;

/// The platform-specific panic handler function.
static PANIC_HANDLER: Spinlock<fn(&core::panic::PanicInfo) -> !> = Spinlock::new(fallback);

/// Entry point used after all boot protocol and architecture specific code has been run.
fn stub_main(command_line: &str) -> Result<(), StubError> {
    let (scheme, entry_point, image_allocation, slide) = executable::load()?;
    crate::debug!("Executable Entry Point: {entry_point:#x}");

    let executable_command_line = command_line
        .rsplit_once(" -- ")
        .map(|(_, b)| b)
        .unwrap_or_default()
        .trim();

    let image_physical_address = image_allocation.range().start_address();
    switch(
        scheme,
        entry_point,
        image_physical_address,
        slide,
        executable_command_line,
    )?;

    Ok(())
}

/// Various errors that can occur in the architecture-independent phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StubError {
    /// An error occurred while loading the executable.
    LoadExecutableError(LoadExecutableError),
    /// An error occurred while switching to the executable.
    SwitchError(SwitchError),
}

impl From<LoadExecutableError> for StubError {
    fn from(error: LoadExecutableError) -> Self {
        Self::LoadExecutableError(error)
    }
}

impl From<SwitchError> for StubError {
    fn from(error: SwitchError) -> Self {
        Self::SwitchError(error)
    }
}

impl fmt::Display for StubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::LoadExecutableError(error) => write!(f, "error loading the executable: {error}"),
            Self::SwitchError(error) => write!(f, "error switching to executable: {error}"),
        }
    }
}

impl error::Error for StubError {}

/// Generic handler for panics.
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    let lock = PANIC_HANDLER.lock();
    let handler = *lock;
    drop(lock);

    handler(info)
}

/// The panic handler function utilized if no other panic handler is assigned.
fn fallback(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

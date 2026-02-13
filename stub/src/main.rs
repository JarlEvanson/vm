//! The first stage loader for the `revm` platform.
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::{error, fmt};

use sync::Spinlock;

use crate::{
    arch::{SwitchError, switch},
    executable::LoadExecutableError,
};

pub mod arch;
pub mod executable;
pub mod platform;
pub mod util;

/// Entry point used after all boot protocol and architecture specific code has been run.
fn stub_main() -> Result<(), StubError> {
    let (address_space, machine, entry_point, image_allocation, slide) = executable::load()?;
    switch(
        address_space,
        machine,
        entry_point,
        image_allocation.range().start().start_address(),
        slide,
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

/// The platform-specific panic handler function.
static PANIC_FUNC: Spinlock<fn(&core::panic::PanicInfo) -> !> = Spinlock::new(fallback);

/// Generic handler for panics.
#[panic_handler]
#[cfg(not(test))]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    PANIC_FUNC.lock()(info)
}

/// The panic handler function utilized if no other panic handler is assigned.
fn fallback(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

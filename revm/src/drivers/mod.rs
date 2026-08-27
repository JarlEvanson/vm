use core::{error, fmt, mem};

struct Driver {
    name: &'static str,
    probe_options: ProbeOptionsFunc,
}

impl Driver {
    pub const fn new(name: &'static str, probe_options: ProbeOptionsFunc) -> Self {
        Self {
            name,
            probe_options,
        }
    }
}

type ProbeOptionsFunc = fn(options: &str) -> Result<(), DriverInitializationError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriverInitializationError {
    /// A driver matching the requested name was not found.
    NotFound,
}

impl fmt::Display for DriverInitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverInitializationError::NotFound => write!(f, "failed to locate requested driver"),
        }
    }
}

impl error::Error for DriverInitializationError {}

pub fn probe_drivers(name: &str, options: &str) -> Result<(), DriverInitializationError> {
    for driver in drivers() {
        if driver.name != name {
            continue;
        }

        return (driver.probe_options)(options);
    }

    Err(DriverInitializationError::NotFound)
}

fn drivers() -> &'static [Driver] {
    unsafe extern "C" {
        #[link_name = "_drivers_start"]
        static DRIVERS_START: u8;

        #[link_name = "_drivers_end"]
        static DRIVERS_END: u8;
    }

    let start = (&raw const DRIVERS_START).cast::<Driver>();
    let end = &raw const DRIVERS_END;
    let length = end.addr().strict_sub(start.addr()) / mem::size_of::<Driver>();

    unsafe { core::slice::from_raw_parts(start, length) }
}

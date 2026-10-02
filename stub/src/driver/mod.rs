use core::{mem, slice};

pub mod arch;

pub trait Driver: Sync {
    fn driver_name(&self) -> &'static str;

    fn validate_preparedness(&self, arg: &'static str) -> bool;

    fn connect(&self, arg: &'static str) -> Result<(), ()>;
}

unsafe extern "Rust" {
    #[link_name = "_drivers_begin"]
    static DRIVERS_BEGIN: &'static dyn Driver;
    #[link_name = "_drivers_end"]
    static DRIVERS_END: &'static dyn Driver;
}

pub fn drivers() -> &'static [&'static dyn Driver] {
    let start = &raw const DRIVERS_BEGIN;
    let end = &raw const DRIVERS_END;

    let len = (end.addr() - start.addr()) / mem::size_of::<&'static dyn Driver>();

    // SAFETY:
    //
    // The linker script associated with this subproject ensures that `DRIVERS_BEGIN` and
    // `DRIVERS_END` are properly defined.
    unsafe {
        slice::from_raw_parts(start, len)
    }
}

#[macro_export]
macro_rules! declare_driver {
    ($driver:expr) => {
        #[used]
        #[unsafe(link_section = ".drivers")]
        static DRIVER_REF: &'static dyn $crate::driver::Driver = &$driver;
    };
}

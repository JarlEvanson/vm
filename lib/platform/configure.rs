use crate::config::Config;

#[path = "linux/configure.rs"]
mod linux;
#[path = "uefi/configure.rs"]
mod uefi;

pub fn configure(config: &mut Config) {
    linux::configure(config);
    uefi::configure(config);
}

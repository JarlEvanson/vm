use crate::config::Config;

#[path = "uefi/configure.rs"]
mod uefi;

pub fn configure(config: &mut Config) {
    uefi::configure(config);
}

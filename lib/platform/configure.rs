use crate::config::{Config, Subproject};

#[path = "limine/configure.rs"]
mod limine;
#[path = "linux/configure.rs"]
mod linux;
#[path = "uefi/configure.rs"]
mod uefi;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    limine::configure(config, subprojects);
    linux::configure(config, subprojects);
    uefi::configure(config, subprojects)
}

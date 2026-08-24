use crate::config::{Config, Subproject};

#[path = "limine/configure.rs"]
mod limine;
#[path = "uefi/configure.rs"]
mod uefi;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    limine::configure(config, subprojects);
    uefi::configure(config, subprojects)
}

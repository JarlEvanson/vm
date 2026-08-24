use crate::config::{Config, Subproject};

#[path = "uefi/configure.rs"]
mod uefi;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    uefi::configure(config, subprojects)
}

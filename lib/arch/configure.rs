use crate::config::{Config, Subproject};

#[path = "aarch64/configure.rs"]
mod aarch64;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    aarch64::configure(config, subprojects);
}

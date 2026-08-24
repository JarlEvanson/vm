use crate::config::{Config, Subproject};

#[path = "aarch64/configure.rs"]
mod aarch64;
#[path = "x86/configure.rs"]
mod x86;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    aarch64::configure(config, subprojects);
    x86::configure(config, subprojects);
}

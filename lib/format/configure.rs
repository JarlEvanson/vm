use crate::config::{Config, Subproject};

#[path = "elf/configure.rs"]
mod elf;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    elf::configure(config, subprojects);
}

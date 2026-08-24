use crate::config::{Config, Subproject};

#[path = "elf/configure.rs"]
mod elf;
#[path = "pe/configure.rs"]
mod pe;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    elf::configure(config, subprojects);
    pe::configure(config, subprojects);
}

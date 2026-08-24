use crate::config::{Config, Subproject};

#[path = "conversion/configure.rs"]
mod conversion;
#[path = "memory/configure.rs"]
mod memory;
#[path = "sync/configure.rs"]
mod sync;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    conversion::configure(config, subprojects);
    memory::configure(config, subprojects);
    sync::configure(config, subprojects);
}

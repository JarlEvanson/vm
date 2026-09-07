use crate::config::Config;

#[path = "conversion/configure.rs"]
mod conversion;
#[path = "memory/configure.rs"]
mod memory;
#[path = "sync/configure.rs"]
mod sync;

pub fn configure(config: &mut Config) {
    conversion::configure(config);
    memory::configure(config);
    sync::configure(config);
}

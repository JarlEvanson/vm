use crate::config::Config;

#[path = "conversion/configure.rs"]
mod conversion;
#[path = "sync/configure.rs"]
mod sync;

pub fn configure(config: &mut Config) {
    conversion::configure(config);
    sync::configure(config);
}

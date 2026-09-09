use crate::config::Config;

#[path = "conversion/configure.rs"]
mod conversion;
#[path = "logbuffer/configure.rs"]
mod logbuffer;

pub fn configure(config: &mut Config) {
    conversion::configure(config);
    logbuffer::configure(config);
}

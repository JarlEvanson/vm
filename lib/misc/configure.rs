use crate::config::Config;

#[path = "conversion/configure.rs"]
mod conversion;

pub fn configure(config: &mut Config) {
    conversion::configure(config);
}

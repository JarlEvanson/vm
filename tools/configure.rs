use crate::config::Config;

#[path = "build-system/configure.rs"]
mod build_system;

pub fn configure(config: &mut Config) {
    build_system::configure(config);
}

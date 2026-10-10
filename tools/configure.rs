use crate::config::Config;

#[path = "build-system/configure.rs"]
mod build_system;
#[path = "package/configure.rs"]
mod package;

pub fn configure(config: &mut Config) {
    build_system::configure(config);
    package::configure(config);
}

use crate::config::Config;

#[path = "configure/configure.rs"]
mod configure;
#[path = "package/configure.rs"]
mod package;

pub fn configure(config: &mut Config) {
    configure::configure(config);
    package::configure(config);
}

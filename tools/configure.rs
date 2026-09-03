use crate::config::Config;

#[path = "configure/configure.rs"]
mod configure;

pub fn configure(config: &mut Config) {
    configure::configure(config);
}

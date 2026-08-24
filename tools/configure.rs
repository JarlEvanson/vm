use crate::config::{Config, Subproject};

#[path = "configure/configure.rs"]
mod configure;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    configure::configure(config, subprojects);
}

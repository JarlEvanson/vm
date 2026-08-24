use crate::config::{Config, Subproject};

#[path = "device-tree/configure.rs"]
mod device_tree;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    device_tree::configure(config, subprojects);
}

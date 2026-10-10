use crate::config::Config;

#[path = "device-tree/configure.rs"]
mod device_tree;

pub fn configure(config: &mut Config) {
    device_tree::configure(config);
}

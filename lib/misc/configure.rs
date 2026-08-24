use crate::config::{Config, Subproject};

#[path = "conversion/configure.rs"]
mod conversion;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    conversion::configure(config, subprojects);
}

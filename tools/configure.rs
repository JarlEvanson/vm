use crate::config::{Config, Subproject};

#[path = "configure/configure.rs"]
mod configure;
#[path = "font-converter/configure.rs"]
mod font_converter;
#[path = "package/configure.rs"]
mod package;

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    configure::configure(config, subprojects);
    font_converter::configure(config, subprojects);
    package::configure(config, subprojects);
}

use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("arch");
    root_module.push("x86");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("x86", root_module);
    subproject.add_libraries("conversion");

    subprojects.push(subproject);
}

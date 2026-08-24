use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("platform");
    root_module.push("uefi");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("uefi", root_module);
    subproject.add_libraries("conversion");

    subprojects.push(subproject);
}

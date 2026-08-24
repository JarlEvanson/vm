use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("stub-api");
    root_module.push("src");
    root_module.push("lib.rs");

    let subproject = Subproject::new("stub-api", root_module);
    subprojects.push(subproject);
}

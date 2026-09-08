use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("firmware");
    root_module.push("device-tree");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("device-tree", root_module);
    subproject.add_libraries("conversion");

    config.subprojects.push(subproject);
}

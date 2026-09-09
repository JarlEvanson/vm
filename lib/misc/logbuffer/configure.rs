use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("misc");
    root_module.push("logbuffer");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("logbuffer", root_module);
    subproject.add_libraries("conversion");

    config.subprojects.push(subproject);
}

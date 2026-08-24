use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("tools");
    root_module.push("font-converter");
    root_module.push("src");
    root_module.push("main.rs");

    let mut subproject = Subproject::new("font-converter", root_module);
    subproject.set_binary(true);
    subproject.disable_revm();
    subproject.disable_revm_stub();

    subproject.add_libraries("font");

    subprojects.push(subproject);
}

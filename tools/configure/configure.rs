use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("tools");
    root_module.push("configure");
    root_module.push("src");
    root_module.push("main.rs");

    let mut subproject = Subproject::new("configure", root_module);
    subproject.set_binary(true);
    subproject.disable_host();
    subproject.disable_revm();
    subproject.disable_revm_stub();

    subprojects.push(subproject);
}

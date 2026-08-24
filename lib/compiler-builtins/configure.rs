use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("compiler-builtins");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("compiler-builtins", root_module);
    subproject.disable_build();
    subproject.disable_host();

    subprojects.push(subproject);
}

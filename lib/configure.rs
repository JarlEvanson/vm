use crate::config::{Config, Subproject};

#[path = "compiler-builtins/configure.rs"]
mod compiler_builtins;

pub fn configure(config: &mut Config) {
    core_configure(config);
    compiler_builtins::configure(config);
}

fn core_configure(config: &mut Config) {
    let mut root_module = config.rustc_sysroot.join("lib");
    root_module.push("rustlib");
    root_module.push("src");
    root_module.push("rust");
    root_module.push("library");
    root_module.push("core");
    root_module.push("src");
    root_module.push("lib.rs");

    let mut subproject = Subproject::new("core", root_module);
    subproject.disable_build();
    subproject.disable_host();
    subproject.set_external();

    config.subprojects.push(subproject);
}

use crate::config::{Config, Subproject};

#[path = "compiler-builtins/configure.rs"]
mod compiler_builtins;
#[path = "firmware/configure.rs"]
mod firmware;
#[path = "format/configure.rs"]
mod format;
#[path = "misc/configure.rs"]
mod misc;
#[path = "platform/configure.rs"]
mod platform;

pub fn configure(config: &mut Config) {
    core_configure(config);

    let mut root_module = config.arguments.source_dir.join("lib");
    root_module.push("test");
    root_module.push("src");
    root_module.push("main.rs");

    let mut subproject = Subproject::new("test", root_module);
    subproject.set_binary(true);

    subproject.add_libraries("logbuffer");

    config.subprojects.push(subproject);

    compiler_builtins::configure(config);
    firmware::configure(config);
    format::configure(config);
    misc::configure(config);
    platform::configure(config);
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

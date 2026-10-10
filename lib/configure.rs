use crate::config::{
    Config,
    subproject::{Library, Subproject, Target},
};

#[path = "compiler-builtins/configure.rs"]
mod compiler_builtins;
#[path = "format/configure.rs"]
mod format;
#[path = "misc/configure.rs"]
mod misc;

pub fn configure(config: &mut Config) {
    core_configure(config);

    compiler_builtins::configure(config);
    format::configure(config);
    misc::configure(config);
}

fn core_configure(config: &mut Config) {
    let mut root_module_path = config.rustc_sysroot.join("lib");
    root_module_path.push("rustlib");
    root_module_path.push("src");
    root_module_path.push("rust");
    root_module_path.push("library");
    root_module_path.push("core");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let mut library = Library::new(root_module_path);
    library.library_target_disable(Target::Build);
    library.library_target_disable(Target::Host);

    let mut subproject = Subproject::new_library("core", library);
    subproject.set_external();

    config.add_subproject(subproject);
}

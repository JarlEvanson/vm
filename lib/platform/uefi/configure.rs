use crate::config::{
    Config, Subproject,
    subproject::{Library, Target},
};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("lib");
    root_module_path.push("platform");
    root_module_path.push("uefi");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let mut library = Library::new(root_module_path);
    library.add_dependency("conversion");

    library
        .library_target_mut(Target::Revm)
        .add_dependency("core");
    library
        .library_target_mut(Target::Revm)
        .add_dependency("compiler-builtins");
    library
        .library_target_mut(Target::RevmStub)
        .add_dependency("core");
    library
        .library_target_mut(Target::RevmStub)
        .add_dependency("compiler-builtins");

    let subproject = Subproject::new_library("uefi", library);
    config.add_subproject(subproject);
}

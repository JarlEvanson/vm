use crate::config::{
    Config,
    subproject::{Library, Subproject, Target},
};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("lib");
    root_module_path.push("compiler-builtins");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let mut library = Library::new(root_module_path);
    library.library_target_disable(Target::Build);
    library.library_target_disable(Target::Host);

    let library_target = library.library_target_mut(Target::Revm);
    library_target.disable_default_dependencies();
    library_target.add_dependency("core");

    let library_target = library.library_target_mut(Target::RevmStub);
    library_target.disable_default_dependencies();
    library_target.add_dependency("core");

    config.add_subproject(Subproject::new_library("compiler-builtins", library));
}

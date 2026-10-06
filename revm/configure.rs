use crate::config::{
    Config,
    subproject::{Binary, Subproject, Target},
};

pub fn configure(config: &mut Config) {
    let subproject_path = config.cli.source_dir_path.join("revm");

    let mut root_module_path = subproject_path.join("src");
    root_module_path.push("main.rs");

    let mut binary = Binary::new("revm", root_module_path);
    binary.binary_target_disable(Target::Build);
    binary.binary_target_disable(Target::Host);
    binary.binary_target_disable(Target::RevmStub);

    let binary_target = binary.binary_target_mut(Target::Revm);
    binary_target.set_linker_script(subproject_path.join("linker-script.ld"));
    binary_target.add_dependency("core");
    binary_target.add_dependency("compiler-builtins");

    config.add_subproject(Subproject::new_binary("revm", binary));
}

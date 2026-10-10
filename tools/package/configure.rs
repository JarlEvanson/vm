use crate::config::{
    Config, Subproject,
    subproject::{Binary, Target},
};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("tools");
    root_module_path.push("package");
    root_module_path.push("src");
    root_module_path.push("main.rs");

    let mut binary = Binary::new("package", root_module_path);
    binary.binary_target_disable(Target::Host);
    binary.binary_target_disable(Target::Revm);
    binary.binary_target_disable(Target::RevmStub);

    binary.add_dependency("conversion");
    binary.add_dependency("elf");
    binary.add_dependency("pe");

    let subproject = Subproject::new_binary("package", binary);
    config.add_subproject(subproject);
}

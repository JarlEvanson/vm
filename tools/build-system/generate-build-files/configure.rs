use crate::config::{
    Config,
    subproject::{Binary, Subproject, Target},
};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("tools");
    root_module_path.push("build-system");
    root_module_path.push("generate-build-files");
    root_module_path.push("src");
    root_module_path.push("main.rs");

    let mut binary = Binary::new("generate-build-files", root_module_path);
    binary.binary_target_disable(Target::Host);
    binary.binary_target_disable(Target::Revm);
    binary.binary_target_disable(Target::RevmStub);

    config.add_subproject(Subproject::new_binary("generate-build-files", binary));
}

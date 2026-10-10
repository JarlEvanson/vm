use crate::config::{
    Config,
    subproject::{Binary, Subproject, Target},
};

pub fn configure(config: &mut Config) {
    let subproject_path = config.cli.source_dir_path.join("stub");

    let mut root_module_path = subproject_path.join("src");
    root_module_path.push("main.rs");

    let mut binary = Binary::new("revm-stub", root_module_path);
    binary.binary_target_disable(Target::Build);
    binary.binary_target_disable(Target::Host);
    binary.binary_target_disable(Target::Revm);

    let binary_target = binary.binary_target_mut(Target::RevmStub);
    binary_target.set_linker_script(subproject_path.join("linker-script.ld"));
    binary_target.add_dependency("core");
    binary_target.add_dependency("compiler-builtins");

    if config.kconfig.contains_key("CONFIG_STUB_PLATFORM_UEFI") {
        binary.add_dependency("uefi");
    }

    if !config.kconfig.contains_key("CONFIG_STUB_PLATFORMS_VALID") {
        eprintln!("=========================================================");
        eprintln!("ERROR: Configuration validation failed!");
        eprintln!("You must select at least one boot platform for revm-stub.");
        eprintln!("=========================================================");
        std::process::exit(1)
    }

    config.add_subproject(Subproject::new_binary("revm-stub", binary));
}

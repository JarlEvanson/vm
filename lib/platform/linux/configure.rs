use crate::config::{Config, Subproject, subproject::Library};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("lib");
    root_module_path.push("platform");
    root_module_path.push("linux");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let library = Library::new(root_module_path);
    let subproject = Subproject::new_library("linux", library);
    config.add_subproject(subproject);
}

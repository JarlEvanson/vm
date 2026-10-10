use crate::config::{Config, Subproject, subproject::Library};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("lib");
    root_module_path.push("firmware");
    root_module_path.push("device-tree");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let mut library = Library::new(root_module_path);
    library.add_dependency("conversion");

    let subproject = Subproject::new_library("device-tree", library);
    config.add_subproject(subproject);
}

use crate::config::{Config, Subproject, subproject::Library};

pub fn configure(config: &mut Config) {
    let mut root_module_path = config.cli.source_dir_path.join("lib");
    root_module_path.push("misc");
    root_module_path.push("conversion");
    root_module_path.push("src");
    root_module_path.push("lib.rs");

    let library = Library::new(root_module_path);
    let subproject = Subproject::new_library("conversion", library);
    config.add_subproject(subproject);
}

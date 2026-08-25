use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("revm");
    root_module.push("src");
    root_module.push("main.rs");

    let mut subproject = Subproject::new("revm", root_module);

    let mut linker_script = config.arguments.source_dir.join("revm");
    linker_script.push("linker-script.ld");
    subproject.set_linker_script(linker_script);

    subproject.set_binary(false);
    subproject.disable_build();
    subproject.disable_host();
    subproject.disable_revm_stub();

    subproject.add_libraries("conversion");
    subproject.add_libraries("stub-api");

    subprojects.push(subproject);
}

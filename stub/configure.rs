use crate::config::{Config, Subproject};

pub fn configure(config: &mut Config, subprojects: &mut Vec<Subproject>) {
    let mut root_module = config.arguments.source_dir.join("stub");
    root_module.push("src");
    root_module.push("main.rs");

    let mut subproject = Subproject::new("revm-stub", root_module);

    let mut linker_script = config.arguments.source_dir.join("stub");
    linker_script.push("linker-script.ld");
    subproject.set_linker_script(linker_script);

    subproject.set_binary(false);
    subproject.disable_build();
    subproject.disable_host();
    subproject.disable_revm();

    subproject.add_libraries("conversion");
    subproject.add_libraries("memory");
    subproject.add_libraries("stub-api");
    subproject.add_libraries("sync");

    subprojects.push(subproject);
}

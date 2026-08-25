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
    subproject.add_libraries("memory");
    subproject.add_libraries("stub-api");
    subproject.add_libraries("sync");

    if config.kconfig.contains_key("CONFIG_STUB_ARCH_AARCH64") {
        subproject.add_libraries("aarch64");
    } else if config.kconfig.contains_key("CONFIG_STUB_ARCH_I686")
        || config.kconfig.contains_key("CONFIG_STUB_ARCH_X86_64")
    {
        subproject.add_libraries("x86");
    }

    subprojects.push(subproject);
}

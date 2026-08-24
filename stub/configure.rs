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
    subproject.add_libraries("elf");
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

    if config.kconfig.contains_key("CONFIG_STUB_GRAPHICS") {
        subproject.add_libraries("font");
    }

    if config.kconfig.contains_key("CONFIG_STUB_PLATFORM_LIMINE") {
        subproject.add_libraries("limine");
    }
    if config.kconfig.contains_key("CONFIG_STUB_PLATFORM_UEFI") {
        subproject.add_libraries("uefi");
    }

    if !config.kconfig.contains_key("CONFIG_STUB_PLATFORMS_VALID") {
        eprintln!("=========================================================");
        eprintln!("ERROR: Configuration validation failed!");
        eprintln!("You must select at least one boot platform for revm-stub.");
        eprintln!("=========================================================");
        std::process::exit(1)
    }

    subprojects.push(subproject);
}

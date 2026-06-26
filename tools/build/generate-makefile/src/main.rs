use std::{
    fmt::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use config::{Config, parse_config};

fn main() -> ExitCode {
    let mut arguments = std::env::args();

    let executable_name = arguments
        .next()
        .unwrap_or_else(|| String::from("generate-makefile"));
    if arguments.len() != 2 {
        eprintln!("Usage: {executable_name} <CONFIG_PATH> <MAKEFILE_PATH>");
        return ExitCode::FAILURE;
    }

    let config_path = PathBuf::from(arguments.next().unwrap());
    let makefile_path = PathBuf::from(arguments.next().unwrap());

    let config_string = match std::fs::read_to_string(&config_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "error reading configuration file {}: {error}",
                config_path.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let config = match parse_config(&config_string) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error parsing configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut makefile = String::new();
    match generate_makefile(&mut makefile, &config, &config_path, &makefile_path) {
        Ok(()) => {}
        Err(error) => {
            eprintln!(
                "error generate makefile from '{}': {error}",
                config_path.display()
            );
            return ExitCode::FAILURE;
        }
    }

    match std::fs::write(&makefile_path, makefile) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "error writing generated makefile to {}: {error}",
                makefile_path.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn generate_makefile(
    s: &mut String,
    config: &Config,
    config_path: &Path,
    makefile_path: &Path,
) -> fmt::Result {
    let resolved_root_module = {
        let root_module = PathBuf::from(config.root_module());

        let mut resolved_root_module = PathBuf::from(config_path);
        resolved_root_module.pop();
        resolved_root_module.join(root_module)
    };

    let (output_artifact, dep_artifact) = if config.binary() {
        let crate_name = config.crate_name().replace("_", "-");

        (crate_name.clone(), format!("{crate_name}.d",))
    } else {
        (
            format!("lib{}.rlib", config.crate_name()),
            format!("lib{}.d", config.crate_name()),
        )
    };

    writeln!(s, "BUILD_CONFIG_TARGETS += {}", config_path.display())?;
    if config.format() {
        writeln!(
            s,
            "RUST_FMT_TARGETS += rustfmt-{}",
            resolved_root_module.display()
        )?;
    }

    if config.regenerate() {
        writeln!(
            s,
            "REGENERATE_TARGETS += regenerate-{}",
            makefile_path.display()
        )?;
    }

    if config.binary() && config.tool() {
        if !s.is_empty() {
            writeln!(s)?;
        }

        let build_config = BuildConfig {
            build_type: BuildType::Native {
                tool: config.tool(),
            },
            build_mode: BuildMode::Compile,
        };

        write!(s, "$(TOOLS_DIR)/{output_artifact}:")?;
        write!(s, " $(TOOLS_DIR_STAMP)")?;
        writeln!(s, " $({})/{output_artifact}", build_config.build_dir())?;

        writeln!(
            s,
            "\tcp $({})/{output_artifact} $@",
            build_config.build_dir()
        )?;
    }

    emit_build_rules(
        s,
        config,
        &resolved_root_module,
        &output_artifact,
        &dep_artifact,
    )?;

    if config.format() {
        if !s.is_empty() {
            writeln!(s)?;
        }

        writeln!(s, ".PHONY: rustfmt-{}", resolved_root_module.display())?;
        writeln!(s, "rustfmt-{}:", resolved_root_module.display())?;
        writeln!(
            s,
            "\trustfmt --edition 2024 {}",
            resolved_root_module.display()
        )?;
    }

    if config.regenerate() {
        if !s.is_empty() {
            writeln!(s)?;
        }

        writeln!(s, ".PHONY: regenerate-{}", makefile_path.display())?;

        write!(s, "regenerate-{}:", makefile_path.display())?;
        writeln!(s, " $(TOOLS_DIR)/generate-makefile")?;

        writeln!(s, "\t$(TOOLS_DIR)/generate-makefile \\")?;
        writeln!(s, "\t\t{} \\", config_path.display())?;
        writeln!(s, "\t\t{}", makefile_path.display())?;
    }

    Ok(())
}

fn emit_build_rules(
    s: &mut String,
    config: &Config,
    resolved_root_module: &Path,
    output_artifact: &str,
    dep_artifact: &str,
) -> fmt::Result {
    if config.native() {
        if !s.is_empty() {
            writeln!(s)?;
        }

        emit_build_rule(
            s,
            config,
            BuildType::Native {
                tool: config.tool(),
            },
            resolved_root_module,
            output_artifact,
            dep_artifact,
        )?;
    }

    if config.target() {
        if !s.is_empty() {
            writeln!(s)?;
        }

        if config.crate_name() != "revm" {
            emit_build_rule(
                s,
                config,
                BuildType::Stub,
                resolved_root_module,
                output_artifact,
                dep_artifact,
            )?;
        }

        if config.crate_name() != "revm_stub" {
            writeln!(s)?;
            emit_build_rule(
                s,
                config,
                BuildType::Revm,
                resolved_root_module,
                output_artifact,
                dep_artifact,
            )?;
        }
    }

    Ok(())
}

fn emit_build_rule(
    s: &mut String,
    config: &Config,
    build_type: BuildType,
    resolved_root_module: &Path,
    output_artifact: &str,
    dep_artifact: &str,
) -> fmt::Result {
    emit_build_rule_raw(
        s,
        config,
        BuildConfig {
            build_type,
            build_mode: BuildMode::Compile,
        },
        resolved_root_module,
        output_artifact,
        dep_artifact,
    )?;

    writeln!(s)?;
    emit_build_rule_raw(
        s,
        config,
        BuildConfig {
            build_type,
            build_mode: BuildMode::Clippy,
        },
        resolved_root_module,
        output_artifact,
        dep_artifact,
    )?;

    writeln!(s)?;
    emit_build_rule_raw(
        s,
        config,
        BuildConfig {
            build_type,
            build_mode: BuildMode::Document,
        },
        resolved_root_module,
        output_artifact,
        dep_artifact,
    )
}

fn emit_build_rule_raw(
    s: &mut String,
    config: &Config,
    build_config: BuildConfig,
    resolved_root_module: &Path,
    output_artifact: &str,
    dep_artifact: &str,
) -> fmt::Result {
    let build_dir = build_config.build_dir();
    let crate_name = config.crate_name();
    let crate_type = if config.binary() { "bin" } else { "lib" };

    if matches!(
        build_config.build_mode,
        BuildMode::Compile | BuildMode::Clippy
    ) {
        if build_config.build_mode == BuildMode::Clippy {
            if config.crate_name() != "core" {
                writeln!(s, ".PHONY: $({build_dir})/{output_artifact}")?;
            }
            writeln!(s, "CLIPPY_TARGETS += $({build_dir})/{output_artifact}")?;
        }
        write!(s, "$({build_dir})/{output_artifact}:")?;
        write!(s, " $({build_dir}_STAMP)")?;
        write!(s, " .config.cfg")?;
        write!(s, " .config.mk")?;

        if config.library() {
            write!(s, " $(TOOLS_DIR)/fix-dependencies")?;
        }

        if !build_config.native() && crate_name != "core" {
            write!(s, " $({build_dir})/libcore.rlib")?;

            if crate_name != "compiler_builtins" {
                write!(s, " $({build_dir})/libcompiler_builtins.rlib")?;
            }
        }
        for library in config.libraries() {
            write!(s, " $({build_dir})/lib{library}.rlib")?;
        }
        writeln!(s)?;

        let driver =
            if build_config.build_mode == BuildMode::Clippy && config.crate_name() != "core" {
                "CLIPPY"
            } else {
                "RUSTC"
            };
        writeln!(s, "\t$({driver}) \\")?;

        let target_type = build_config.target_type();
        writeln!(s, "\t\t$({target_type}_FLAGS) \\")?;
        writeln!(s, "\t\t$(INTERNAL_{target_type}_FLAGS) \\")?;

        if let Some(subtype) = build_config.target_subtype() {
            writeln!(s, "\t\t$({subtype}_FLAGS) \\")?;
            writeln!(s, "\t\t$(INTERNAL_{subtype}_FLAGS) \\")?;
        }

        writeln!(s, "\t\t--crate-name {crate_name} \\",)?;
        writeln!(s, "\t\t--crate-type {crate_type} \\")?;
        writeln!(s, "\t\t--edition 2024 \\")?;

        writeln!(s, "\t\t-L $({build_dir}) \\")?;
        for library in config.libraries() {
            writeln!(s, "\t\t--extern {library} \\")?;
        }

        writeln!(s, "\t\t--emit dep-info=$({build_dir})/{dep_artifact} \\",)?;
        writeln!(s, "\t\t--emit link=$@ \\")?;
        writeln!(s, "\t\t@.config.cfg \\")?;
        writeln!(s, "\t\t{}", resolved_root_module.display())?;

        if config.library() {
            writeln!(s, "\t$(TOOLS_DIR)/fix-dependencies \\")?;
            writeln!(s, "\t\t$({build_dir})/{dep_artifact} \\")?;
            writeln!(s, "\t\t$@")?;
        }

        write!(s, "\n$({build_dir})/{dep_artifact}:")?;
        writeln!(s, " $({build_dir}_STAMP)")?;
        writeln!(s, "\t@touch $@")?;

        writeln!(s, "\ninclude $({build_dir})/{dep_artifact}")?;
    } else {
        let compile_build_dir = BuildConfig {
            build_type: build_config.build_type,
            build_mode: BuildMode::Compile,
        }
        .build_dir();

        writeln!(s, "DOC_TARGETS += $({build_dir})/{crate_name}/index.html")?;

        write!(s, "$({build_dir})/{crate_name}/index.html:")?;
        write!(s, " $({build_dir}_STAMP)")?;

        if !build_config.native() && crate_name != "core" {
            write!(s, " $({compile_build_dir})/libcore.rlib")?;

            if crate_name != "compiler_builtins" {
                write!(s, " $({compile_build_dir})/libcompiler_builtins.rlib")?;
            }
        }
        for library in config.libraries() {
            write!(s, " $({compile_build_dir})/lib{library}.rlib")?;
        }
        writeln!(s)?;

        writeln!(s, "\t$(RUSTDOC) \\")?;

        writeln!(s, "\t\t--crate-name {crate_name} \\")?;
        writeln!(s, "\t\t--crate-type {crate_type} \\")?;
        writeln!(s, "\t\t--edition 2024 \\")?;

        let target_type = build_config.target_type();
        writeln!(s, "\t\t$({target_type}_FLAGS) \\")?;
        writeln!(s, "\t\t$(INTERNAL_{target_type}_FLAGS) \\")?;

        if let Some(subtype) = build_config.target_subtype() {
            writeln!(s, "\t\t$({subtype}_FLAGS) \\")?;
            writeln!(s, "\t\t$(INTERNAL_{subtype}_FLAGS) \\")?;
        }

        writeln!(s, "\t\t-L $({compile_build_dir}) \\")?;
        for library in config.libraries() {
            writeln!(
                s,
                "\t\t--extern {library}=$({compile_build_dir})/lib{library}.rlib \\"
            )?;
        }

        if crate_name != "core" {
            writeln!(s, "\t\t--document-private-items \\")?;
        }
        writeln!(s, "\t\t-o $({build_dir}) \\")?;
        writeln!(s, "\t\t{}", resolved_root_module.display())?;
    }

    Ok(())
}

struct BuildConfig {
    build_type: BuildType,
    build_mode: BuildMode,
}

impl BuildConfig {
    fn native(&self) -> bool {
        matches!(self.build_type, BuildType::Native { tool: _ })
    }

    fn build_dir(&self) -> String {
        let base = match self.build_type {
            BuildType::Native { tool: _ } => "BUILD_DIR_NATIVE",
            BuildType::Stub => "BUILD_DIR_STUB",
            BuildType::Revm => "BUILD_DIR_REVM",
        };

        match self.build_mode {
            BuildMode::Compile => String::from(base),
            BuildMode::Clippy => format!("{base}_CLIPPY"),
            BuildMode::Document => format!("{base}_DOC"),
        }
    }

    const fn target_type(&self) -> &str {
        match self.build_type {
            BuildType::Native { tool: _ } => "NATIVE",
            BuildType::Stub | BuildType::Revm => "TARGET",
        }
    }

    const fn target_subtype(&self) -> Option<&str> {
        match self.build_type {
            BuildType::Native { tool: false } => None,
            BuildType::Native { tool: true } => Some("TOOL"),
            BuildType::Stub => Some("STUB"),
            BuildType::Revm => Some("REVM"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildType {
    Native { tool: bool },
    Stub,
    Revm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildMode {
    Compile,
    Clippy,
    Document,
}

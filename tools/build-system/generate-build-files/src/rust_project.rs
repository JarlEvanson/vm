//! `rust-project.json` file generation functionality.

use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{self, Write},
    path::Path,
};

use crate::{
    config::{
        Config,
        subproject::{Subproject, Target},
    },
    convert_name_to_rust_name,
};

/// Generates the `rust-project.json` contents.
pub fn generate(output: &mut String, config: &Config) -> fmt::Result {
    let mut units = Vec::new();

    for subproject in &config.subprojects {
        for target in Target::ALL_TARGETS {
            'library: {
                if let Some(library) = subproject.library() {
                    let Some(library_target) = library.library_target(target) else {
                        break 'library;
                    };

                    units.push((
                        true,
                        subproject.name(),
                        library.root_module(),
                        library.dependencies().chain(library_target.dependencies()),
                        subproject,
                        target,
                    ));
                }

                'binary: for binary in subproject.binaries() {
                    let Some(binary_target) = binary.binary_target(target) else {
                        continue 'binary;
                    };

                    units.push((
                        false,
                        binary.name(),
                        binary.root_module(),
                        binary.dependencies().chain(binary_target.dependencies()),
                        subproject,
                        target,
                    ))
                }
            }
        }
    }

    units.sort_unstable_by(|a, b| {
        let mut result = a.5.cmp(&b.5);
        if result != Ordering::Equal {
            return result;
        }

        result = a.4.name().cmp(b.4.name());
        if result != Ordering::Equal {
            return result;
        };

        result = a.0.cmp(&b.0);
        if result != Ordering::Equal {
            return result;
        }

        a.1.cmp(b.1)
    });

    let mut dep_positions = HashMap::new();
    for (index, &(is_library, name, _, _, _, target)) in units.iter().enumerate() {
        if !is_library {
            continue;
        }

        dep_positions.insert((name, target), index);
    }

    writeln!(output, "{{")?;

    writeln!(output, "\t\"rustc\": {:?},", config.cli.rustc)?;
    writeln!(output, "\t\"sysroot\": {:?},", config.rustc_sysroot)?;

    let mut rustc_sysroot_src = config.rustc_sysroot.join("lib");
    rustc_sysroot_src.push("rustlib");
    rustc_sysroot_src.push("src");
    rustc_sysroot_src.push("rust");
    rustc_sysroot_src.push("library");
    writeln!(output, "\t\"sysroot_src\": {:?},", rustc_sysroot_src)?;

    writeln!(output, "\t\"cfg_groups\": {{")?;

    let groups = [
        ("CFG_GROUP_BUILD", &config.build_cfgs),
        ("CFG_GROUP_HOST", &config.host_cfgs),
        ("CFG_GROUP_REVM", &config.revm_cfgs),
        ("CFG_GROUP_REVM_STUB", &config.revm_stub_cfgs),
    ];
    for (index, (name, cfgs)) in groups.into_iter().enumerate() {
        writeln!(output, "\t\t{:?}: [", name)?;

        for (index, cfg) in cfgs.iter().enumerate() {
            if index == cfgs.len() - 1 {
                writeln!(output, "\t\t\t\"{}\"", cfg.escape_default())?;
            } else {
                writeln!(output, "\t\t\t\"{}\",", cfg.escape_default())?;
            }
        }

        if index == groups.len() - 1 {
            writeln!(output, "\t\t]")?;
        } else {
            writeln!(output, "\t\t],")?;
        }
    }
    writeln!(output, "\t}},")?;

    writeln!(output, "\t\"crates\": [")?;
    let unit_count = units.len();
    for (index, (is_library, name, root_module, dependencies, subproject, target)) in
        units.into_iter().enumerate()
    {
        generate_unit(
            output,
            config,
            is_library,
            name,
            root_module,
            dependencies,
            subproject,
            target,
            &dep_positions,
        )?;

        if index == unit_count - 1 {
            writeln!(output)?;
        } else {
            writeln!(output, ",")?;
        }
    }
    writeln!(output, "\t]")?;

    writeln!(output, "}}")
}

/// Generates the configuration for a single analysis unit in the `rust-project.json` file.
#[allow(clippy::too_many_arguments)]
fn generate_unit<'a>(
    output: &mut String,
    config: &Config,
    is_library: bool,
    name: &str,
    root_module: &Path,
    dependencies: impl Iterator<Item = &'a str> + Clone,
    subproject: &'a Subproject,
    target: Target,
    dep_positions: &HashMap<(&str, Target), usize>,
) -> fmt::Result {
    writeln!(output, "\t\t{{")?;

    let display_name = format!("{name} ({})", target.display());
    writeln!(output, "\t\t\t\"display_name\": {display_name:?},")?;
    writeln!(output, "\t\t\t\"root_module\": {root_module:?},")?;
    writeln!(output, "\t\t\t\"edition\": \"2024\",")?;

    let mut is_workspace_member = subproject.is_workspace_member();
    if name == "compiler-builtins" {
        // TODO: figure out why this is required to prevent `rust-analyzer` failures.
        is_workspace_member = false;
    }
    writeln!(
        output,
        "\t\t\t\"is_workspace_member\": {},",
        is_workspace_member
    )?;

    let dependency_count = dependencies.clone().count();
    if dependency_count == 0 {
        writeln!(output, "\t\t\t\"deps\": [],")?;
    } else {
        let library_opt = if !is_library && subproject.library().is_some() {
            Some(subproject.name())
        } else {
            None
        };

        writeln!(output, "\t\t\t\"deps\": [")?;
        for (dep_index, dep) in library_opt.into_iter().chain(dependencies).enumerate() {
            writeln!(output, "\t\t\t\t{{")?;

            let rust_dep = convert_name_to_rust_name(dep);
            let index = dep_positions[&(dep, target)];
            writeln!(output, "\t\t\t\t\t\"crate\": {index},")?;
            writeln!(output, "\t\t\t\t\t\"name\": {rust_dep:?}")?;

            if dep_index == dependency_count - 1 {
                writeln!(output, "\t\t\t\t}}")?;
            } else {
                writeln!(output, "\t\t\t\t}},")?;
            }
        }
        writeln!(output, "\t\t\t],")?;
    }

    let cfg_group = match target {
        Target::Build => "CFG_GROUP_BUILD",
        Target::Host => "CFG_GROUP_HOST",
        Target::Revm => "CFG_GROUP_REVM",
        Target::RevmStub => "CFG_GROUP_REVM_STUB",
    };
    write!(output, "\t\t\t\"cfg_groups\": [ {:?} ]", cfg_group)?;

    if name == "generate-build-files" {
        writeln!(output, ",")?;

        writeln!(output, "\t\t\t\"source\": {{")?;

        write!(output, "\t\t\t\t\"include_dirs\": [")?;
        write!(output, " {:?} ", config.cli.source_dir_path)?;
        writeln!(output, "],")?;

        writeln!(output, "\t\t\t\t\"exclude_dirs\": []")?;

        writeln!(output, "\t\t\t}}")?;
    } else {
        writeln!(output)?;
    }

    write!(output, "\t\t}}")
}

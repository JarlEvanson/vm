use std::{
    collections::HashMap,
    fmt::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use config::{Config, parse_config};

fn main() -> ExitCode {
    let mut arguments = std::env::args();

    let executable_name = arguments
        .next()
        .unwrap_or_else(|| String::from("generate-rust-project"));
    let build_config_paths = arguments.collect::<Vec<_>>();
    if build_config_paths.is_empty() {
        eprintln!("Usage: {executable_name} <BUILD_CONFIG_FILES>...");
        return ExitCode::FAILURE;
    }

    let build_configs = {
        let mut encountered_error = false;

        let mut accumulator = HashMap::new();
        for path in build_config_paths {
            let contents = match std::fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    eprintln!("error reading '{path}': {error}");
                    encountered_error = true;
                    continue;
                }
            };

            match parse_config(&contents) {
                Ok(config) => {
                    let resolved_root_module = {
                        let root_module = PathBuf::from(config.root_module());

                        let mut resolved_root_module = PathBuf::from(&path);
                        resolved_root_module.pop();
                        resolved_root_module
                            .join(root_module)
                            .canonicalize()
                            .unwrap()
                            .into_string()
                            .unwrap()
                    };

                    let duplicate = accumulator
                        .insert(config.crate_name().clone(), (resolved_root_module, config));
                    if let Some((_, duplicate)) = duplicate {
                        eprintln!(
                            "error handling '{path}': '{path}' defines a duplicate of '{}'",
                            duplicate.crate_name()
                        );
                        encountered_error = true;
                        continue;
                    }
                }
                Err(error) => {
                    eprintln!("error parsing '{path}': {error}");
                    encountered_error = true;
                    continue;
                }
            };
        }

        if encountered_error {
            return ExitCode::FAILURE;
        }

        accumulator
    };

    let mut output = String::new();
    match generate_rust_project(&mut output, build_configs) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("error generating 'rust-project.json': {error}");
            return ExitCode::FAILURE;
        }
    }

    match std::fs::write("rust-project.json", &output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error writing 'rust-project.json': {error}");
            ExitCode::FAILURE
        }
    }
}

fn generate_rust_project(
    output: &mut String,
    build_configs: HashMap<String, (String, Config)>,
) -> fmt::Result {
    let crates_graph = generate_crate_graph(build_configs);
    let mut crates = crates_graph.iter().collect::<Vec<_>>();
    crates.sort_unstable_by_key(|value| value.1.index);

    writeln!(output, "{{")?;

    let sysroot_src = {
        let core_description = CrateDescription {
            crate_name: String::from("core"),
            crate_type: CrateType::Stub,
        };
        let mut core_root = PathBuf::from(&crates_graph[&core_description].root_module);
        core_root.pop();
        core_root.pop();
        core_root.pop();
        core_root.into_string().unwrap()
    };
    writeln!(output, "\t\"sysroot_src\": {sysroot_src:?},")?;
    writeln!(output, "\t\"crates\": [")?;
    let crate_count = crates.len();
    for (index, (description, krate)) in crates.iter().enumerate() {
        writeln!(output, "\t\t{{")?;

        writeln!(
            output,
            "\t\t\t\"display_name\": {:?},",
            description.crate_name
        )?;
        writeln!(output, "\t\t\t\"root_module\": {:?},", krate.root_module)?;
        writeln!(output, "\t\t\t\"edition\": \"2024\",")?;

        if krate.deps.is_empty() {
            writeln!(output, "\t\t\t\"deps\": []")?;
        } else {
            writeln!(output, "\t\t\t\"deps\": [")?;

            for (dep_index, dep) in krate.deps.iter().enumerate() {
                writeln!(output, "\t\t\t\t{{")?;

                writeln!(output, "\t\t\t\t\t\"name\": {:?},", dep.crate_name)?;
                let crate_index = crates_graph[dep].index;
                writeln!(output, "\t\t\t\t\t\"crate\": {crate_index}")?;

                if dep_index == krate.deps.len() - 1 {
                    writeln!(output, "\t\t\t\t}}")?;
                } else {
                    writeln!(output, "\t\t\t\t}},")?;
                }
            }

            writeln!(output, "\t\t\t]")?;
        }

        if index != crate_count - 1 {
            writeln!(output, "\t\t}},")?;
        } else {
            writeln!(output, "\t\t}}")?;
        }
    }
    writeln!(output, "\t]")?;

    writeln!(output, "}}")?;
    Ok(())
}

fn generate_crate_graph(
    build_configs: HashMap<String, (String, Config)>,
) -> HashMap<CrateDescription, Crate> {
    let mut crates = HashMap::new();

    let mut build_configs = build_configs.into_iter().collect::<Vec<_>>();
    build_configs.sort_unstable_by_key(|(name, _)| String::from(name));

    for (crate_name, (root_module, config)) in build_configs.iter() {
        if config.native() {
            let (description, krate) = generate_crate(
                crate_name,
                CrateType::Native,
                root_module,
                config.libraries().iter().map(|s| s.as_str()),
                crates.len(),
            );
            crates.insert(description, krate);
        }

        if config.target() {
            let implicit_core = if config.crate_name() != "core" {
                Some(String::from("core"))
            } else {
                None
            };
            let implicit_compiler_builtins =
                if implicit_core.is_some() && config.crate_name() != "compiler_builtins" {
                    Some(String::from("compiler_builtins"))
                } else {
                    None
                };

            let iter = implicit_core
                .iter()
                .chain(implicit_compiler_builtins.iter())
                .chain(config.libraries().iter())
                .map(|s| s.as_str());

            if config.crate_name() != "revm" {
                let (description, krate) = generate_crate(
                    crate_name,
                    CrateType::Stub,
                    root_module,
                    iter.clone(),
                    crates.len(),
                );
                crates.insert(description, krate);
            }

            if config.crate_name() != "stub" {
                let (description, krate) = generate_crate(
                    crate_name,
                    CrateType::Revm,
                    root_module,
                    iter.clone(),
                    crates.len(),
                );
                crates.insert(description, krate);
            }
        }
    }

    crates
}

fn generate_crate<'a, I: Iterator<Item = &'a str>>(
    crate_name: &str,
    crate_type: CrateType,
    root_module: &str,
    deps: I,
    index: usize,
) -> (CrateDescription, Crate) {
    let description = CrateDescription {
        crate_name: String::from(crate_name),
        crate_type,
    };

    let mut crate_deps = Vec::new();
    for dep in deps {
        let description = CrateDescription {
            crate_name: String::from(dep),
            crate_type,
        };

        crate_deps.push(description);
    }

    let krate = Crate {
        index,
        root_module: String::from(root_module),
        deps: crate_deps,
    };

    (description, krate)
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CrateDescription {
    crate_name: String,
    crate_type: CrateType,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum CrateType {
    Native,
    Stub,
    Revm,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Crate {
    index: usize,
    root_module: String,
    deps: Vec<CrateDescription>,
}

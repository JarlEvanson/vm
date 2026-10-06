//! Generator for build files (`build.ninja` and `rust-project.json`).

use std::process::ExitCode;

use crate::config::{Action, cli};

mod config;
mod ninja;
mod platform;
mod rust_project;

fn main() -> ExitCode {
    let config = match config::load() {
        Ok(action) => match action {
            Action::Help => {
                cli::help();
                return ExitCode::SUCCESS;
            }
            Action::Generate(config) => config,
        },
        Err(error) => {
            eprintln!("error while loading configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    if config.cli.verbose {
        println!("{config:#?}");
    }

    let mut rust_project = String::new();
    if let Err(error) = rust_project::generate(&mut rust_project, &config) {
        eprintln!(
            "error generating '{}': {error}",
            config.cli.rust_project_path.display()
        );
        return ExitCode::FAILURE;
    }

    if config.cli.verbose {
        println!();
        println!("{rust_project}");
    }

    if let Err(error) = std::fs::write(&config.cli.rust_project_path, &rust_project) {
        eprintln!(
            "error writing '{}': {error}",
            config.cli.rust_project_path.display()
        );
        return ExitCode::FAILURE;
    }

    let mut ninja = Vec::new();
    ninja::generate(&mut ninja, &config);

    match std::fs::write(&config.cli.build_ninja_path, &ninja) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "error writing '{}': {error}",
                config.cli.build_ninja_path.display()
            );
            ExitCode::FAILURE
        }
    }
}

/// Converts a [`Subproject`][s] or [`Binary`][b] name into its corresponding `crate-name`.
///
/// [s]: crate::config::subproject::Subproject
/// [b]: crate::config::subproject::Binary
fn convert_name_to_rust_name(s: &str) -> String {
    s.replace('-', "_")
}

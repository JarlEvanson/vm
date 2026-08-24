use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    process::ExitCode,
};

use crate::config::{LoadConfigAction, cli};

mod config;
mod ninja;
mod rust_project;

fn main() -> ExitCode {
    let config = match config::load() {
        Ok(action) => match action {
            LoadConfigAction::Help => {
                cli::usage();
                return ExitCode::SUCCESS;
            }
            LoadConfigAction::Run(config) => config,
        },
        Err(error) => {
            eprintln!("error while loading config: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut rust_project = String::new();
    let rust_project_json_path = config.arguments.source_dir.join("rust-project.json");
    if let Err(error) = rust_project::generate(&mut rust_project, &config) {
        eprintln!(
            "error generating '{}': {error}",
            rust_project_json_path.display()
        );
        return ExitCode::FAILURE;
    }

    if let Err(error) = std::fs::write(&rust_project_json_path, &rust_project) {
        eprintln!(
            "error writing '{}': {error}",
            rust_project_json_path.display()
        );
        return ExitCode::FAILURE;
    }

    let mut ninja = Vec::new();
    ninja::generate(&mut ninja, &config);

    match std::fs::write(&config.arguments.ninja_path, &ninja) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "error writing '{}': {error}",
                config.arguments.ninja_path.display()
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
fn bytes_to_os_str<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> OsString {
    use std::os::unix::ffi::OsStringExt;

    OsString::from_vec(bytes.into().into_owned())
}

#[cfg(unix)]
fn os_str_to_bytes<'a>(os_str: impl Into<Cow<'a, OsStr>>) -> Vec<u8> {
    use std::os::unix::ffi::OsStringExt;

    os_str.into().into_owned().into_vec()
}

fn convert_name_to_rust_name(s: &str) -> String {
    s.replace('-', "_")
}

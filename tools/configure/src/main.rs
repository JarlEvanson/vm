use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    process::ExitCode,
};

use crate::config::{LoadConfigAction, cli};

mod config;

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

    ExitCode::SUCCESS
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

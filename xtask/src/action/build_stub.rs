//! Helper function to build `revm_stub` given a [`BuildStubConfig`].

use std::path::PathBuf;

use anyhow::Result;

use crate::{action::run_cmd, cli::build_stub::BuildStubConfig};

/// Builds `revm_stub` as specified by `config`, returning the path to the final binary on
/// success.
///
/// # Errors
///
/// Returns errors when the `cargo build` command fails.
pub fn build_revm_stub(config: BuildStubConfig) -> Result<PathBuf> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build");

    cmd.args(["--package", "revm-stub"]);
    cmd.args(["--target", config.arch.as_target_spec()]);
    cmd.args(["-Z", "build-std=core,compiler_builtins"]);
    cmd.args(["-Z", "build-std-features=compiler-builtins-mem"]);
    cmd.args(["-Z", "json-target-spec"]);
    cmd.args(["--profile", config.profile.as_str()]);

    run_cmd(cmd)?;

    let mut target_string = PathBuf::from(config.arch.as_target_spec());
    target_string.set_extension("");
    let Some(target_string) = target_string.file_name() else {
        unreachable!()
    };

    let mut binary_location = PathBuf::with_capacity(50);
    binary_location.push("target");
    binary_location.push(target_string);
    binary_location.push(config.profile.target_string());
    binary_location.push("revm-stub");

    Ok(binary_location)
}

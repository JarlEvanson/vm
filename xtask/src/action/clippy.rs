//! Helper function to run `cargo clippy` on all packages given a [`ClippyConfig`].

use anyhow::Result;

use crate::{DEPENDENCIES, action::run_cmd, cli::clippy::ClippyConfig};

/// Runs `cargo clippy` on all packages.
///
/// # Errors
///
/// Returns errors when the `cargo build` command fails.
pub fn clippy(config: ClippyConfig) -> Result<()> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("clippy");

    for package in DEPENDENCIES {
        cmd.args(["--package", package]);
    }
    cmd.arg("--no-deps");

    run_cmd(cmd)?;

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("clippy");

    cmd.args(["--package", "revm-stub"]);
    cmd.arg("--no-deps");
    cmd.args(["--target", config.arch.as_target_spec()]);
    cmd.args(["-Z", "build-std=core,compiler_builtins"]);
    cmd.args(["-Z", "build-std-features=compiler-builtins-mem"]);
    cmd.args(["-Z", "json-target-spec"]);

    run_cmd(cmd)?;

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("clippy");

    cmd.args(["--package", "revm"]);
    cmd.arg("--no-deps");
    cmd.args(["--target", config.arch.as_target_spec()]);
    cmd.args(["-Z", "build-std=core,compiler_builtins"]);
    cmd.args(["-Z", "build-std-features=compiler-builtins-mem"]);
    cmd.args(["-Z", "json-target-spec"]);

    run_cmd(cmd)?;

    Ok(())
}

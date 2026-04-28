//! Helper function to document all packages given a [`DocConfig`].

use anyhow::Result;

use crate::{DEPENDENCIES, action::run_cmd, cli::doc::DocConfig};

/// Runs `cargo doc` on all packages.
///
/// # Errors
///
/// Returns errors if a `cargo doc` command fails.
pub fn doc(config: DocConfig) -> Result<()> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("doc");

    for package in DEPENDENCIES {
        cmd.args(["--package", package]);
    }
    cmd.arg("--no-deps");

    run_cmd(cmd)?;

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("doc");

    cmd.args(["--package", "revm-stub"]);
    cmd.arg("--no-deps");
    cmd.args(["--target", config.arch.as_target_spec()]);
    cmd.args(["-Z", "build-std=core,compiler_builtins"]);
    cmd.args(["-Z", "build-std-features=compiler-builtins-mem"]);
    cmd.args(["-Z", "json-target-spec"]);

    run_cmd(cmd)?;

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("doc");

    cmd.args(["--package", "revm"]);
    cmd.arg("--no-deps");
    cmd.args(["--target", config.arch.as_target_spec()]);
    cmd.args(["-Z", "build-std=core,compiler_builtins"]);
    cmd.args(["-Z", "build-std-features=compiler-builtins-mem"]);
    cmd.args(["-Z", "json-target-spec"]);

    run_cmd(cmd)?;

    Ok(())
}

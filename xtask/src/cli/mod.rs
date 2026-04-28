//! Command line parsing and [`Action`] construction.

use clap::Command;

use crate::cli::{
    build_revm::BuildRevmConfig, build_stub::BuildStubConfig, clippy::ClippyConfig, doc::DocConfig,
    package::PackageConfig, run::RunConfig,
};

pub mod build_revm;
pub mod build_stub;
pub mod clippy;
pub mod doc;
pub mod package;
pub mod run;

/// The action to carry out.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    /// Build `revm_stub` with a specific configuration.
    BuildStub(BuildStubConfig),
    /// Build `revm` with a specific configuration.
    BuildRevm(BuildRevmConfig),
    /// Packages `revm-stub` and `revm` into a single binary ready for execution.
    Package(PackageConfig),
    /// Runs a packaged `revm-stub` and `revm`.
    Run(RunConfig),
    /// Runs `cargo clippy` on all packages.
    Clippy(ClippyConfig),
    /// Runs `cargo doc` on all packages.
    Doc(DocConfig),
}

/// Parses `xtask`'s arguments to construct an [`Action`].
pub fn get_action() -> Action {
    let matches = command_parser().get_matches();

    let Some((subcommand_name, subcommand_matches)) = matches.subcommand() else {
        unreachable!("subcommand is required");
    };
    match subcommand_name {
        "build-stub" => Action::BuildStub(build_stub::parse_arguments(subcommand_matches)),
        "build-revm" => Action::BuildRevm(build_revm::parse_arguments(subcommand_matches)),
        "package" => Action::Package(package::parse_arguments(subcommand_matches)),
        "run" => Action::Run(run::parse_arguments(subcommand_matches)),
        "clippy" => Action::Clippy(clippy::parse_arguments(subcommand_matches)),
        "doc" => Action::Doc(doc::parse_arguments(subcommand_matches)),
        _ => unreachable!("unexpected subcommand: {subcommand_name:?}"),
    }
}

/// Returns the command parser for all [`Action`]s.
fn command_parser() -> Command {
    Command::new("xtask")
        .about("Developer utility for running various tasks on `revm-stub` and `revm`")
        .subcommand(build_stub::subcommand_parser())
        .subcommand(build_revm::subcommand_parser())
        .subcommand(package::subcommand_parser())
        .subcommand(run::subcommand_parser())
        .subcommand(clippy::subcommand_parser())
        .subcommand(doc::subcommand_parser())
        .subcommand_required(true)
        .arg_required_else_help(true)
}

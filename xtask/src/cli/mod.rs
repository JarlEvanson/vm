//! Command line parsing and [`Action`] construction.

use clap::Command;

use crate::cli::build_stub::BuildStubConfig;

pub mod build_stub;

/// The action to carry out.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    /// Build `revm_stub` with a specific configuration.
    BuildStub(BuildStubConfig),
}

/// Parses `xtask`'s arguments to construct an [`Action`].
pub fn get_action() -> Action {
    let matches = command_parser().get_matches();

    let Some((subcommand_name, subcommand_matches)) = matches.subcommand() else {
        unreachable!("subcommand is required");
    };
    match subcommand_name {
        "build-stub" => Action::BuildStub(build_stub::parse_arguments(subcommand_matches)),
        _ => unreachable!("unexpected subcommand: {subcommand_name:?}"),
    }
}

/// Returns the command parser for all [`Action`]s.
fn command_parser() -> Command {
    Command::new("xtask")
        .about("Developer utility for running various tasks on tvm_loader and tvm")
        .subcommand(build_stub::subcommand_parser())
        .subcommand_required(true)
        .arg_required_else_help(true)
}

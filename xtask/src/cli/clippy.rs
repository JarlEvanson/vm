//! Command line parsing and [`Action::Clippy`][ac] construction.
//!
//! [ac]: crate::cli::Action::Clippy

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser};

use crate::common::Arch;

/// Description of various parameters used for `cargo clippy`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ClippyConfig {
    /// The [`Arch`] for which `revm` should be built.
    pub arch: Arch,
}

/// Parses the arguments required to produce a valid [`ClippyConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> ClippyConfig {
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or_else(|| unreachable!("`arch` is a required argument"));

    ClippyConfig { arch }
}

/// Returns the command parser for an [`Action::Clippy`][ac].
///
/// [ac]: crate::cli::Action::Clippy
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new())
        .required(true);

    Command::new("clippy")
        .about("Run clippy on all packages")
        .arg(arch)
}

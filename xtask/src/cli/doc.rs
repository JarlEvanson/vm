//! Command line parsing and [`Action::Doc`][ac] construction.
//!
//! [ac]: crate::cli::Action::Doc

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser};

use crate::common::Arch;

/// Description of various parameters used for `cargo doc`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct DocConfig {
    /// The [`Arch`] for which `revm` should be built.
    pub arch: Arch,
}

/// Parses the arguments required to produce a valid [`DocConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> DocConfig {
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or_else(|| unreachable!("`arch` is a required argument"));

    DocConfig { arch }
}

/// Returns the command parser for an [`Action::Doc`][ac].
///
/// [ac]: crate::cli::Action::Doc
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new())
        .required(true);

    Command::new("doc")
        .about("Run doc on all packages")
        .arg(arch)
}

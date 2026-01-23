//! Command line parsing and [`Action::BuildRevm`][abr] construction.
//!
//! [abr]: crate::cli::Action::BuildRevm

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser};

use crate::common::{Arch, Profile};

/// Description of various parameters of the `revm` build process and the built-in
/// configuration of `revm`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BuildRevmConfig {
    /// The [`Arch`] for which `revm` should be built.
    pub arch: Arch,
    /// The [`Profile`] with which `revm` should be built.
    pub profile: Profile,
}

/// Parses the arguments required to produce a valid [`BuildRevmConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> BuildRevmConfig {
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or_else(|| unreachable!("`arch` is a required argument"));

    let profile = matches
        .get_one::<Profile>("profile")
        .copied()
        .unwrap_or_else(|| unreachable!("`profile` should have a default value"));

    BuildRevmConfig { arch, profile }
}

/// Returns the command parser for an [`Action::BuildRevm`][abr].
///
/// [abr]: crate::cli::Action::BuildRevm
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new())
        .required(true);

    let profile = Arg::new("profile")
        .long("profile")
        .value_parser(EnumValueParser::<Profile>::new())
        .default_value("dev");

    Command::new("build-revm")
        .about("Builds `revm`")
        .arg(arch)
        .arg(profile)
}

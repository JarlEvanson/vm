//! Command line parsing and [`Action::BuildStub`][abs] construction.
//!
//! [abs]: crate::cli::Action::BuildStub

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser};

use crate::common::{Arch, Profile};

/// Description of various parameters of the `revm_stub` build process and the built-in
/// configuration of `revm_stub`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BuildStubConfig {
    /// The [`Arch`] for which `revm_stub` should be built.
    pub arch: Arch,
    /// The [`Profile`] with which `revm_stub` should be built.
    pub profile: Profile,
}

/// Parses the arguments required to produce a valid [`BuildStubConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> BuildStubConfig {
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or_else(|| unreachable!("`arch` is a required argument"));

    let profile = matches
        .get_one::<Profile>("profile")
        .copied()
        .unwrap_or_else(|| unreachable!("`profile` should have a default value"));

    BuildStubConfig { arch, profile }
}

/// Returns the command parser for an [`Action::BuildStub`][abs].
///
/// [abs]: crate::cli::Action::BuildStub
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new())
        .required(true);

    let profile = Arg::new("profile")
        .long("profile")
        .value_parser(EnumValueParser::<Profile>::new())
        .default_value("dev");

    Command::new("build-stub")
        .about("Builds `revm-stub`")
        .arg(arch)
        .arg(profile)
}

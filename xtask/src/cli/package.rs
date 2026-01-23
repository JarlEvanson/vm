//! Command line parsing and [`Action::Package`][ap] construction.
//!
//! [ap]: crate::cli::Action::Package

use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser, value_parser};

use crate::common::{Arch, Profile};

/// Description of various parameters of the `revm` and `revm_stub` build process and the built-in
/// configuration of `revm` and `revm_stub`.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PackageConfig {
    /// The configuration for packaging `revm-stub`.
    pub stub: CrateConfig,
    /// The configuration for packaging `revm`.
    pub revm: CrateConfig,
    /// The location at which the packaged executable should be placed.
    pub output_path: PathBuf,
}

/// Description of configuration related to either `revm` or `revm-stub`.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CrateConfig {
    /// Use the artifact located using given [`PathBuf`].
    Path(PathBuf),
    /// Build a new artifact targeted for [`Arch`] with [`Profile`].
    Build {
        /// The [`Arch`] for which the crate should be built.
        arch: Arch,
        /// The [`Profile`] with which the crate should be built.
        profile: Profile,
    },
}

/// Parses the arguments required to produce a valid [`PackageConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> PackageConfig {
    let profile = matches
        .get_one::<Profile>("profile")
        .copied()
        .unwrap_or_else(|| unreachable!("`profile` should have a default value"));

    // We can safely default to [`Arch::Aarch64`] because the argument will have been specified by
    // the user if this value is used.
    //
    // In other words, the default value will never actually be used.
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or(Arch::Aarch64);

    let stub = match matches.get_one::<PathBuf>("stub-path") {
        Some(path) => CrateConfig::Path(path.clone()),
        None => CrateConfig::Build { arch, profile },
    };

    let revm = match matches.get_one::<PathBuf>("stub-path") {
        Some(path) => CrateConfig::Path(path.clone()),
        None => CrateConfig::Build { arch, profile },
    };

    let output_path = matches
        .get_one("output-path")
        .cloned()
        .unwrap_or_else(|| unreachable!("`output-path` should be a required argument"));

    PackageConfig {
        stub,
        revm,
        output_path,
    }
}

/// Returns the command parser for an [`Action::Package`][ap].
///
/// [ap]: crate::cli::Action::Package
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new());

    let profile = Arg::new("profile")
        .long("profile")
        .value_parser(EnumValueParser::<Profile>::new())
        .default_value("dev");

    let stub_path = Arg::new("stub-path")
        .long("stub-path")
        .value_parser(value_parser!(PathBuf))
        .required_unless_present("arch");

    let revm_path = Arg::new("revm-path")
        .long("revm-path")
        .value_parser(value_parser!(PathBuf))
        .required_unless_present("arch");

    let output_path = Arg::new("output-path")
        .long("output-path")
        .visible_alias("output")
        .value_parser(value_parser!(PathBuf))
        .required(true);

    Command::new("package")
        .about("Packages `revm-stub` and `revm`")
        .arg(stub_path)
        .arg(revm_path)
        .arg(arch)
        .arg(profile)
        .arg(output_path)
}

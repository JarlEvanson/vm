//! Command line parsing and [`Action::Run`][ar] construction.
//!
//! [ar]: crate::cli::Action::Run

use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command, builder::EnumValueParser, value_parser};

use crate::{
    cli::package::CrateConfig,
    common::{Arch, Profile},
};

/// Description of various parameters that describe how to obtain a packaged `revm`, where
/// artifacts required to run `revm` should be obtained, and where the run directory should be
/// located.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RunConfig {
    /// The architecture on which to run the packaged `revm`.
    pub arch: Arch,
    /// The configuration for obtaining a packaged `revm`.
    pub package: PackageConfig,
    /// The directory containing pre-built OVMF artifacts.
    pub ovmf_dir: PathBuf,
    /// The directory containing pre-built Limine artifacts.
    pub limine_dir: PathBuf,
    /// The location at which the run artifacts should be placed.
    pub run_dir: PathBuf,
}

/// Description of configuration related to how a packaged `revm` should be obtained.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum PackageConfig {
    /// Use the artifact located using the given [`PathBuf`].
    Path(PathBuf),
    /// Package a new artifact targeted for [`Arch`] with [`Profile`].
    Package {
        /// The configuration for obtaining `revm-stub`.
        stub: CrateConfig,
        /// The configuration for obtaining `revm`.
        revm: CrateConfig,
    },
}

/// Parses the arguments required to produce a valid [`RunConfig`].
pub fn parse_arguments(matches: &ArgMatches) -> RunConfig {
    let profile = matches
        .get_one::<Profile>("profile")
        .copied()
        .unwrap_or_else(|| unreachable!("`profile` should have a default value"));

    // We can safely default to [`Arch::Aarch64`] because the argument will have been specified by
    // the user if this value is used.
    let arch = matches
        .get_one::<Arch>("arch")
        .copied()
        .unwrap_or_else(|| unreachable!("`arch` should be a required argument"));

    let stub = match matches.get_one::<PathBuf>("stub-path") {
        Some(path) => CrateConfig::Path(path.clone()),
        None => CrateConfig::Build { arch, profile },
    };

    let revm = match matches.get_one::<PathBuf>("stub-path") {
        Some(path) => CrateConfig::Path(path.clone()),
        None => CrateConfig::Build { arch, profile },
    };

    let package = match matches.get_one::<PathBuf>("package-path") {
        Some(path) => PackageConfig::Path(path.clone()),
        None => PackageConfig::Package { stub, revm },
    };

    let ovmf_dir = matches
        .get_one("ovmf-dir")
        .cloned()
        .unwrap_or_else(|| unreachable!("`ovmf-dir` should be a required argument"));

    let limine_dir = matches
        .get_one("limine-dir")
        .cloned()
        .unwrap_or_else(|| unreachable!("`limine-dir` should be a required argument"));

    let run_dir = matches
        .get_one("run-dir")
        .cloned()
        .unwrap_or_else(|| unreachable!("`run-dir` should be a required argument"));

    RunConfig {
        arch,
        package,
        ovmf_dir,
        limine_dir,
        run_dir,
    }
}

/// Returns the command parser for an [`Action::Run`][ar]
///
/// [ar]: crate::cli::Action::Run
pub fn subcommand_parser() -> Command {
    let arch = Arg::new("arch")
        .long("arch")
        .value_parser(EnumValueParser::<Arch>::new())
        .required(true);

    let profile = Arg::new("profile")
        .long("profile")
        .value_parser(EnumValueParser::<Profile>::new())
        .default_value("dev");

    let stub_path = Arg::new("stub-path")
        .long("stub-path")
        .value_parser(value_parser!(PathBuf));

    let revm_path = Arg::new("revm-path")
        .long("revm-path")
        .value_parser(value_parser!(PathBuf));

    let package_path = Arg::new("package-path")
        .long("package-path")
        .value_parser(value_parser!(PathBuf))
        .conflicts_with_all(["stub-path", "revm_path"]);

    let ovmf_dir = Arg::new("ovmf-dir")
        .long("ovmf-dir")
        .env("OVMF_DIR")
        .value_parser(value_parser!(PathBuf))
        .required(true);

    let limine_dir = Arg::new("limine-dir")
        .long("limine-dir")
        .env("LIMINE_DIR")
        .value_parser(value_parser!(PathBuf))
        .required(true);

    let run_dir = Arg::new("run-dir")
        .long("run-dir")
        .value_parser(value_parser!(PathBuf))
        .required(true);

    Command::new("run")
        .about("Runs `revm-stub` and `revm`")
        .arg(stub_path)
        .arg(revm_path)
        .arg(arch)
        .arg(profile)
        .arg(package_path)
        .arg(ovmf_dir)
        .arg(limine_dir)
        .arg(run_dir)
}

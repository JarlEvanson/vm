//! Configuration extraction functionality.
//!
//! Extracts the desired configurations from Kconfig's `.config` and the command line.

use std::{
    collections::HashMap, error, ffi::OsString, fmt, io, path::PathBuf, process::Command,
    string::FromUtf8Error,
};

use crate::{
    config::{
        cli::{CliAction, CliConfig, ParseArgumentsError},
        compiler_options::CompilerOptions,
        kconfig::ParseKconfigError,
        subproject::{Subproject, Target},
        targets::AcquireTripletsError,
    },
    platform::bytes_to_os_string,
};

pub mod cli;
mod compiler_options;
mod kconfig;
pub mod subproject;
mod targets;

#[allow(clippy::missing_docs_in_private_items)]
#[path = "../../../../../lib/configure.rs"]
mod configure_lib;
#[allow(clippy::missing_docs_in_private_items)]
#[path = "../../../../../revm/configure.rs"]
mod configure_revm;
#[allow(clippy::missing_docs_in_private_items)]
#[path = "../../../../../stub/configure.rs"]
mod configure_revm_stub;
#[allow(clippy::missing_docs_in_private_items)]
#[path = "../../../../../tools/configure.rs"]
mod configure_tools;

/// Loads the configuration from the command line and the associated Kconfig's `.config` file (can
/// be changed on the command line).
#[allow(clippy::result_large_err)]
pub fn load() -> Result<Action, LoadConfigError> {
    let cli_config = match cli::parse_arguments()? {
        CliAction::Help => return Ok(Action::Help),
        CliAction::Generate(config) => config,
    };

    let rustc_sysroot = acquire_sysroot(&cli_config)?;

    let kconfig = kconfig::parse(&cli_config.config_path)?;
    let (revm_triplet_path, revm_stub_triplet_path) =
        targets::acquire_triplets(&kconfig, &cli_config.source_dir_path)?;

    let general_opts = compiler_options::locate(&kconfig, "GENERAL");
    let revm_opts = compiler_options::locate(&kconfig, "REVM");
    let revm_stub_opts = compiler_options::locate(&kconfig, "STUB");

    let mut config = Config {
        cli: cli_config,

        kconfig,

        rustc_sysroot,
        revm_triplet_path,
        revm_stub_triplet_path,

        general_opts,
        revm_opts,
        revm_stub_opts,

        build_cfgs: Vec::new(),
        host_cfgs: Vec::new(),
        revm_cfgs: Vec::new(),
        revm_stub_cfgs: Vec::new(),

        subprojects: Vec::new(),
    };

    config.build_cfgs = acquire_cfgs(&config, Target::Build)?;
    config.host_cfgs = acquire_cfgs(&config, Target::Host)?;
    config.revm_cfgs = acquire_cfgs(&config, Target::Revm)?;
    config.revm_stub_cfgs = acquire_cfgs(&config, Target::RevmStub)?;

    configure_lib::configure(&mut config);
    configure_revm::configure(&mut config);
    configure_revm_stub::configure(&mut config);
    configure_tools::configure(&mut config);

    Ok(Action::Generate(config))
}

/// The various actions that this executable can perform.
#[expect(clippy::large_enum_variant)]
pub enum Action {
    /// The help message should be printed.
    Help,
    /// The build files should be generated in accordance with the provided [`Config`].
    Generate(Config),
}

/// The configuration that should be used for the build process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The portion of the configuration that was extracted from the command line arguments.
    pub cli: CliConfig,

    /// The key-value pairs read from Kconfig's `.config` file.
    pub kconfig: HashMap<String, String>,

    /// The path to the sysroot for the `rustc` provided on the command line.
    pub rustc_sysroot: PathBuf,
    /// The path to the target specification used for compilation of `revm` and its dependencies.
    pub revm_triplet_path: PathBuf,
    /// The path to the target specification used for compilation of `revm-stub` and its
    /// dependencies.
    pub revm_stub_triplet_path: PathBuf,

    /// The [`CompilerOptions`] for general compilation.
    pub general_opts: CompilerOptions,
    /// The [`CompilerOptions`] for compilation of `revm` and its dependencies.
    pub revm_opts: CompilerOptions,
    /// The [`CompilerOptions`] for compilation of `revm-stub` and its dependencies.
    pub revm_stub_opts: CompilerOptions,

    /// The `cfg` options that should be set when compiling for the `build` target.
    pub build_cfgs: Vec<String>,
    /// The `cfg` options that should be set when compiling for the `host` target.
    pub host_cfgs: Vec<String>,
    /// The `cfg` options that should be set when compiling for the `revm` target.
    pub revm_cfgs: Vec<String>,
    /// The `cfg` options that should be set when compiling for the `revm-stub` target.
    pub revm_stub_cfgs: Vec<String>,

    /// The [`Subproject`]s this build system will build.
    pub subprojects: Vec<Subproject>,
}

impl Config {
    /// Adds the provided [`Subproject`] to the list of [`Subproject`]s.
    fn add_subproject(&mut self, subproject: Subproject) {
        self.subprojects.push(subproject);
    }
}

/// Various errors that can occur while loading the configuration.
#[derive(Debug)]
pub enum LoadConfigError {
    /// An error occurred while parsing the command line arguments.
    ParseArgumentsError(ParseArgumentsError),
    /// The execution of a command failed.
    CommandExecutionFailed {
        /// The command whose execution failed.
        command: Command,
        /// The error that occurred.
        error: io::Error,
    },
    /// The command returned a failing exit code.
    CommandFailedCode {
        /// The command whose execution returned a failing exit code.
        command: Command,
        /// The exit code returned by the command.
        code: i32,
    },
    /// The command failed without an exit code.
    CommandFailedWithoutCode {
        /// The command that failed.
        command: Command,
    },
    /// An error occurred while loading and parsing the Kconfig `.config` file.
    ParseKconfigError(ParseKconfigError),
    /// An error occurred while acquiring the triplets.
    AcquireTripletsError(AcquireTripletsError),
    /// Parsing the command's stdout as UTF-8 failed.
    Utf8Command {
        /// The command whose stdout parsing failed.
        command: Command,
        /// The error returned from the parsing function.
        error: FromUtf8Error,
    },
}

impl From<ParseArgumentsError> for LoadConfigError {
    fn from(error: ParseArgumentsError) -> Self {
        Self::ParseArgumentsError(error)
    }
}

impl From<ParseKconfigError> for LoadConfigError {
    fn from(error: ParseKconfigError) -> Self {
        Self::ParseKconfigError(error)
    }
}

impl From<AcquireTripletsError> for LoadConfigError {
    fn from(error: AcquireTripletsError) -> Self {
        Self::AcquireTripletsError(error)
    }
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseArgumentsError(error) => {
                write!(f, "failed to parse command line arguments: {error}")
            }
            Self::CommandExecutionFailed { command, error } => {
                write!(f, "failed to execute {command:?}: {error}")
            }
            Self::CommandFailedCode { command, code } => {
                write!(f, "{command:?} failed with status code: {code}")
            }
            Self::CommandFailedWithoutCode { command } => {
                write!(f, "{command:?} failed without a status code")
            }
            Self::ParseKconfigError(error) => {
                write!(f, "error parsing Kconfig .config file: {error}")
            }
            Self::AcquireTripletsError(error) => {
                write!(f, "error acquiring triplets: {error}")
            }
            Self::Utf8Command { command, error } => {
                write!(f, "{command:?} returned a non UTF-8 output: {error}")
            }
        }
    }
}

impl error::Error for LoadConfigError {}

/// Returns the path of the sysroot for the `rustc` provided on the command line.
#[expect(clippy::result_large_err)]
fn acquire_sysroot(cli_config: &CliConfig) -> Result<PathBuf, LoadConfigError> {
    let mut command = Command::new(&cli_config.rustc);
    command.args(["--print", "sysroot"]);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return Err(LoadConfigError::CommandExecutionFailed { command, error });
        }
    };

    if !output.status.success() {
        match output.status.code() {
            Some(code) => return Err(LoadConfigError::CommandFailedCode { command, code }),
            None => return Err(LoadConfigError::CommandFailedWithoutCode { command }),
        }
    }

    let rustc_sysroot = PathBuf::from(bytes_to_os_string(output.stdout.trim_ascii()));
    Ok(rustc_sysroot)
}

/// Acquires the `cfg` options that will be set when compiling for the provided [`Target`].
#[expect(clippy::result_large_err)]
fn acquire_cfgs(config: &Config, target: Target) -> Result<Vec<String>, LoadConfigError> {
    let mut command = Command::new(&config.cli.rustc);
    for (name, value) in &config.kconfig {
        command.arg("--cfg");
        command.arg(name);

        command.arg("--cfg");

        let mut cfg_string = format!("{name}=\"");
        for c in value.chars() {
            if c == '\"' {
                cfg_string.push('\\');
                cfg_string.push('\"');
            } else {
                cfg_string.push(c);
            }
        }
        cfg_string.push('\"');
        command.arg(cfg_string);
    }

    match target {
        Target::Build => {}
        Target::Host => {
            if let Some(triplet) = config.cli.host_triplet.as_ref() {
                command.arg("--target");
                command.arg(triplet);
            }
        }
        Target::Revm | Target::RevmStub => {
            let triplet = if target == Target::Revm {
                &config.revm_triplet_path
            } else {
                &config.revm_stub_triplet_path
            };

            command.arg("-Zunstable-options");
            command.arg("--target");
            command.arg(triplet);
        }
    }

    let opts = match target {
        Target::Build | Target::Host => &config.general_opts,
        Target::Revm => &config.revm_opts,
        Target::RevmStub => &config.revm_stub_opts,
    };

    command.arg("-C");
    command.arg(format!("opt-level={}", opts.opt_level.option()));

    if opts.incremental {
        let mut incremental_dir = config.cli.build_dir_path.join(target.folder());
        incremental_dir.push("cache");

        command.arg("-C");

        let mut incremental = OsString::from("incremental=");
        incremental.push(incremental_dir.as_os_str());
        command.arg(incremental);
    }

    command.arg("-C");
    command.arg(format!("debug-assertions={}", opts.debug_assertions));

    command.arg("-C");
    let debuginfo = if opts.debug_info { "full" } else { "none" };
    command.arg(format!("debuginfo={debuginfo}"));

    command.arg("-C");
    command.arg(format!("lto={}", opts.lto.option()));

    command.args(["--print", "cfg"]);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Err(LoadConfigError::CommandExecutionFailed { command, error }),
    };

    if !output.status.success() {
        match output.status.code() {
            Some(code) => return Err(LoadConfigError::CommandFailedCode { command, code }),
            None => return Err(LoadConfigError::CommandFailedWithoutCode { command }),
        }
    }

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(error) => return Err(LoadConfigError::Utf8Command { command, error }),
    };
    let cfgs = stdout.lines().map(String::from).collect::<Vec<_>>();
    Ok(cfgs)
}

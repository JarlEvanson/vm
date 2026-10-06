//! Command line parsing and CLI help functionality.

use std::{env::args_os, error, ffi::OsString, fmt, path::PathBuf};

/// Parses the command line arguments for this function.
pub(super) fn parse_arguments() -> Result<CliAction, ParseArgumentsError> {
    let mut args = args_os();

    let executable_name = args.next().unwrap_or_else(|| OsString::from("configure"));

    let mut source_dir_path = None;
    let mut build_dir_path = None;
    let mut out_dir_path = None;

    let mut rustc = None;
    let mut host_triplet = None;

    let mut config_path = None;

    let mut build_ninja_path = None;
    let mut rust_project_path = None;

    let mut verbose = false;

    while let Some(arg) = args.next() {
        if arg == "--help" {
            return Ok(CliAction::Help);
        } else if arg == "--verbose" {
            verbose = true;
            continue;
        }

        let assignment = if arg == "--source-dir" {
            &mut source_dir_path
        } else if arg == "--build-dir" {
            &mut build_dir_path
        } else if arg == "--out-dir" {
            &mut out_dir_path
        } else if arg == "--rustc" {
            &mut rustc
        } else if arg == "--host-triplet" {
            &mut host_triplet
        } else if arg == "--config-path" {
            &mut config_path
        } else if arg == "--build-ninja-path" {
            &mut build_ninja_path
        } else if arg == "--rust-project-path" {
            &mut rust_project_path
        } else {
            return Err(ParseArgumentsError::UnknownOption {
                option: arg,
                executable_name,
            });
        };

        let Some(next_arg) = args.next() else {
            return Err(ParseArgumentsError::RequiredValueMissing {
                option: arg.to_str().unwrap().into(),
            });
        };

        *assignment = Some(next_arg);
    }

    let source_dir_path = PathBuf::from(source_dir_path.unwrap_or_else(|| OsString::from("")));
    let build_dir_path = PathBuf::from(build_dir_path.unwrap_or_else(|| OsString::from("build")));
    let out_dir_path = PathBuf::from(out_dir_path.unwrap_or_else(|| OsString::from("out")));

    let rustc = rustc.unwrap_or_else(|| OsString::from("rustc"));

    let config_path = PathBuf::from(config_path.unwrap_or_else(|| OsString::from(".config")));

    let build_ninja_path =
        PathBuf::from(build_ninja_path.unwrap_or_else(|| OsString::from("build.ninja")));
    let rust_project_path =
        PathBuf::from(rust_project_path.unwrap_or_else(|| OsString::from("rust-project.json")));

    let config = CliConfig {
        source_dir_path,
        build_dir_path,
        out_dir_path,

        rustc,
        host_triplet,

        config_path,

        build_ninja_path,
        rust_project_path,

        verbose,
    };

    Ok(CliAction::Generate(config))
}

/// The various actions that the CLI can invoke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CliAction {
    /// The help message should be printed.
    Help,
    /// The build files should be generated in accordance with the provided [`CliConfig`].
    Generate(CliConfig),
}

/// The portion of the configuration extracted from the command line arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliConfig {
    /// The path to the directory that contains the source code of this repository.
    pub source_dir_path: PathBuf,
    /// The path to the directory in which all build artifacts should be located.
    pub build_dir_path: PathBuf,
    /// The path to the directory in which all final artifacts should be placed.
    pub out_dir_path: PathBuf,

    /// The executable/command executed to compile source code and locate the other tools utilized
    /// by the build process.
    pub rustc: OsString,
    /// The triplet used to build the host artifacts (i.e., the target for which the `revm`
    /// interaction tools should be compiled).
    pub host_triplet: Option<OsString>,

    /// The path to Kconfig's `.config`.
    pub config_path: PathBuf,

    /// The path at which the `build.ninja` file should be placed.
    pub build_ninja_path: PathBuf,
    /// The path at which the `rust-project.json` file should be placed.
    pub rust_project_path: PathBuf,

    /// Indicates whether verbose logging should be performed.
    pub verbose: bool,
}

/// Various errors that can occur while parsing the command line arguments.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ParseArgumentsError {
    /// An unknown CLI option was encountered.
    UnknownOption {
        /// The unknown CLI option that was encountered.
        option: OsString,
        /// The name or path of this executable that was used for this execution.
        executable_name: OsString,
    },
    /// A value required by a CLI option was not provided.
    RequiredValueMissing {
        /// The CLI option that requires an argument that was missing.
        option: OsString,
    },
}

impl fmt::Display for ParseArgumentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOption {
                option,
                executable_name,
            } => {
                if let Some(option) = option.to_str() {
                    write!(f, "unknown option '{option}'")?;
                } else {
                    let option = option.display();
                    writeln!(f, "unknown option {option}")?;
                }
                if let Some(executable_name) = executable_name.to_str() {
                    write!(f, "Run '{executable_name} --help' for more information")
                } else {
                    let executable_name = executable_name.display();
                    write!(f, "Run '{executable_name} --help' for more information")
                }
            }
            Self::RequiredValueMissing { option } => {
                if let Some(option) = option.to_str() {
                    write!(f, "'{option}' requires an value to be provided")
                } else {
                    let option = option.display();
                    write!(f, "'{option}' requires an value to be provided")
                }
            }
        }
    }
}

impl error::Error for ParseArgumentsError {}

/// Prints the help message.
pub fn help() {
    let executable_name = args_os()
        .next()
        .unwrap_or_else(|| OsString::from("configure"));

    if let Some(executable_name) = executable_name.to_str() {
        println!("Usage: {executable_name} [OPTIONS]");
    } else {
        let executable_name = executable_name.display();
        println!("Usage: {executable_name} [OPTIONS]");
    }
    println!();

    let options = [
        (
            "--source-dir",
            "DIR",
            "Directory in which all source files are located [default: the working directory]",
        ),
        (
            "--build-dir",
            "DIR",
            "Directory to write intermediate build artifacts [default: build]",
        ),
        (
            "--out-dir",
            "DIR",
            "Directory to write final compiled tools and binaries [default: out]",
        ),
        (
            "--rustc",
            "STRING",
            "Path or command to use for rustc executions [default: rustc]",
        ),
        (
            "--host-triplet",
            "TRIPLET",
            "Target TRIPLET for host tool binaries [default: detected build triplet]",
        ),
        (
            "--config-path",
            "PATH",
            "Path to build configuration file [default: .config]",
        ),
        (
            "--build-ninja-path",
            "PATH",
            "Output path for generated build.ninja file [default: build.ninja]",
        ),
        (
            "--rust-project-path",
            "PATH",
            "Output path for generated rust-project.json file [default: rust-project.json]",
        ),
        ("--verbose", "", "Use verbose output"),
        ("--help", "", "Display this message"),
    ];

    println!("Options:");

    const MAX_LINE_LEN: usize = 80;
    const INDENT: &str = "    ";
    const GAP: &str = "  ";

    // Calculate column width for the options and their value names dynamically.
    let left_col_width = options
        .iter()
        .map(|(opt, val, _)| {
            if val.is_empty() {
                opt.len()
            } else {
                opt.len() + 1 + val.len()
            }
        })
        .max()
        .unwrap_or(0);

    let desc_indent = INDENT.len() + left_col_width + GAP.len();
    let max_desc_width = MAX_LINE_LEN.saturating_sub(desc_indent);

    // Format and print each option in its own row.
    for (opt, val, desc) in options {
        let flag_str = if val.is_empty() {
            opt.to_string()
        } else {
            format!("{opt} {val}")
        };

        print!("{INDENT}{flag_str:<0$}{GAP}", left_col_width);

        // Ensure that description text wraps at the boundary.
        let mut current_line_len = 0;
        let mut is_first_word = true;

        for word in desc.split_whitespace() {
            if !is_first_word && current_line_len + 1 + word.len() > max_desc_width {
                println!();
                print!("{:desc_indent$}", "");
                current_line_len = 0;
                is_first_word = true;
            }

            if !is_first_word {
                print!(" ");
                current_line_len += 1;
            }

            print!("{word}");
            current_line_len += word.len();
            is_first_word = false;
        }

        println!();
    }
}

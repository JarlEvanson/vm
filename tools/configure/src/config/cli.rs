use std::{env::args_os, error, ffi::OsString, fmt, path::PathBuf};

pub(in crate::config) fn parse() -> Result<ParseArgumentsAction, ParseArgumentsError> {
    let mut args = args_os();

    let executable_name = args
        .next()
        .and_then(|s| s.to_str().map(String::from))
        .unwrap_or_else(|| String::from("configure"));

    let mut rustc = None;
    let mut host_triplet = None;
    let mut config_path = None;
    let mut ninja_path = None;
    let mut source_dir = None;
    let mut build_dir = None;
    let mut out_dir = None;
    let mut font_path = None;
    let mut verbose = false;
    while let Some(arg) = args.next() {
        let assignment = if arg == "--help" {
            return Ok(ParseArgumentsAction::Help);
        } else if arg == "--rustc" {
            &mut rustc
        } else if arg == "--host-triplet" {
            &mut host_triplet
        } else if arg == "--config-path" {
            &mut config_path
        } else if arg == "--ninja-path" {
            &mut ninja_path
        } else if arg == "--source-dir" {
            &mut source_dir
        } else if arg == "--build-dir" {
            &mut build_dir
        } else if arg == "--out-dir" {
            &mut out_dir
        } else if arg == "--font-path" {
            &mut font_path
        } else if arg == "--verbose" {
            verbose = true;
            continue;
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

    let rustc = rustc.unwrap_or_else(|| OsString::from("rustc"));
    let config_path = PathBuf::from(config_path.unwrap_or_else(|| OsString::from(".config")));
    let ninja_path = PathBuf::from(ninja_path.unwrap_or_else(|| OsString::from("build.ninja")));
    let source_dir = PathBuf::from(source_dir.unwrap_or_else(|| OsString::from(".")));
    let build_dir = PathBuf::from(build_dir.unwrap_or_else(|| OsString::from("build")));
    let out_dir = PathBuf::from(out_dir.unwrap_or_else(|| OsString::from("out")));

    let font_path = font_path.map(PathBuf::from).unwrap_or_else(|| {
        let mut font_path = PathBuf::from(".");
        font_path.push("assets");
        font_path.push("Tamsyn8x16r.psf");
        font_path
    });

    let arguments = Arguments {
        rustc,
        host_triplet,

        config_path,
        ninja_path,

        source_dir,
        build_dir,
        out_dir,

        font_path,

        verbose,
    };

    Ok(ParseArgumentsAction::Run(arguments))
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Arguments {
    pub rustc: OsString,
    pub host_triplet: Option<OsString>,

    pub config_path: PathBuf,
    pub ninja_path: PathBuf,

    pub source_dir: PathBuf,
    pub build_dir: PathBuf,
    pub out_dir: PathBuf,

    pub font_path: PathBuf,

    pub verbose: bool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(in crate::config) enum ParseArgumentsAction {
    Help,
    Run(Arguments),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ParseArgumentsError {
    UnknownOption {
        option: OsString,
        executable_name: String,
    },
    RequiredValueMissing {
        option: String,
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
                    writeln!(f, "unknown option {option:?}")?;
                }
                write!(f, "Run '{executable_name} --help' for more information")
            }
            Self::RequiredValueMissing { option } => {
                write!(f, "'{option}' requires an value to be provided")
            }
        }
    }
}

impl error::Error for ParseArgumentsError {}

pub fn usage() {
    let executable_name = args_os()
        .next()
        .and_then(|s| s.to_str().map(String::from))
        .unwrap_or_else(|| String::from("configure"));

    println!("Usage: {executable_name} [OPTIONS]");
    println!();

    let options = [
        ("--help", "", "Display this message"),
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
            "--ninja-path",
            "PATH",
            "Output path for generated build.ninja file [default: build.ninja]",
        ),
        (
            "--source-dir",
            "DIR",
            "Directory in which all source files are located [default: .]",
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
            "--font-path",
            "PATH",
            "Path to a font file utilize (default: assets/Tamsyn8x16r.psf)",
        ),
        ("--verbose", "", "Use verbose output"),
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

//! Parser for the Kconfig `.config` file.

use std::{
    collections::HashMap,
    error, fmt, io,
    path::{Path, PathBuf},
};

/// Parses the Kconfig `.config` file.
pub(super) fn parse(path: &Path) -> Result<HashMap<String, String>, ParseKconfigError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(ParseKconfigError::LoadError {
                path: PathBuf::from(path),
                error,
            });
        }
    };

    let mut kconfig = HashMap::new();
    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();

        // Ignore empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check for '=' separator.
        let (key, val) = line
            .split_once('=')
            .ok_or_else(|| ParseKconfigError::MissingEquals {
                line_number,
                content: raw_line.to_string(),
            })?;

        let key = key.trim();
        let val = val.trim();

        // Validate and strip quotes.
        let val = parse_quoted_value(val).ok_or_else(|| ParseKconfigError::UnmatchedQuotes {
            line_number,
            content: raw_line.to_string(),
        })?;

        kconfig.insert(String::from(key), String::from(val));
    }

    Ok(kconfig)
}

/// Various errors that can occur when parsing the Kconfig `.config` file.
#[derive(Debug)]
pub enum ParseKconfigError {
    /// An error occurred while loading the Kconfig `.config` file.
    LoadError {
        /// The path to the Kconfig `.config` file.
        path: PathBuf,
        /// The error that occurred.
        error: io::Error,
    },
    /// Line is missing the '=' key-value separator.
    MissingEquals {
        /// The line number at which this error occurred.
        line_number: usize,
        /// The line that failed to be parsed.
        content: String,
    },
    /// Value starts with a quote but lacks a closing quote (or vice versa).
    UnmatchedQuotes {
        /// The line number at which this error occurred.
        line_number: usize,
        /// The line that failed to be parsed.
        content: String,
    },
}

impl fmt::Display for ParseKconfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadError { path, error } => {
                write!(f, "error loading '{}': {error}", path.display())
            }
            Self::MissingEquals {
                line_number,
                content,
            } => {
                write!(
                    f,
                    "Syntax error on line {line_number}: missing '=' in '{content}'"
                )
            }
            Self::UnmatchedQuotes {
                line_number,
                content,
            } => {
                write!(
                    f,
                    "Syntax error on line {line_number}: unmatched quotes in '{content}'"
                )
            }
        }
    }
}

impl error::Error for ParseKconfigError {}

/// Helper to handle quote stripping and mismatched quote validation
fn parse_quoted_value(val: &str) -> Option<&str> {
    let starts_quote = val.starts_with('"');
    let ends_quote = val.ends_with('"');

    match (starts_quote, ends_quote) {
        // Correctly quoted: strip both
        (true, true) if val.len() >= 2 => Some(&val[1..val.len() - 1]),
        // Single quote.
        (true, true) => None,
        // Unmatched quotes
        (true, false) | (false, true) => None,
        // No quotes present
        (false, false) => Some(val),
    }
}

use std::{collections::HashMap, error, fmt};

/// Parses a raw configuration string into a strongly typed [`Config`] struct.
///
/// # Errors
///
/// Returns a [`ParseConfigError`] containing a list of the syntactic errors encountered during
/// parsing or the validation errors encountered while building [`Config`].
pub fn parse_config(s: &str) -> Result<Config, ParseConfigError> {
    let key_values = parse_key_value_list(s)?;

    let mut crate_name = None;
    let mut libraries = Vec::new();
    let mut binary = None;
    let mut native_enabled = None;
    let mut target_enabled = None;
    let mut tool = None;
    let mut format = true;
    let mut regenerate = false;
    let mut root_module = None;

    let mut errors = Vec::new();
    for (key, values) in key_values {
        if key == "crate-name" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::String(s) => crate_name = Some(s.clone()),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "string",
                    }),
                }
            }
        } else if key == "library" {
            for v in values {
                match v {
                    DataType::String(s) => libraries.push(s),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "string",
                    }),
                }
            }
        } else if key == "binary" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => binary = Some(*b),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "tool" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => tool = Some(*b),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "native" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => native_enabled = Some(*b),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "target" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => target_enabled = Some(*b),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "format" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => format = *b,
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "regenerate" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::Bool(b) => regenerate = *b,
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "boolean",
                    }),
                }
            }
        } else if key == "root-module" {
            if values.len() > 1 {
                errors.push(ParseConfigErrorDetail::DuplicateKey(String::from(key)));
            }
            if let Some(first) = values.first() {
                match first {
                    DataType::String(s) => root_module = Some(s.clone()),
                    _ => errors.push(ParseConfigErrorDetail::TypeMismatch {
                        key: String::from(key),
                        expected: "string",
                    }),
                }
            }
        } else {
            errors.push(ParseConfigErrorDetail::UnknownKey(String::from(key)));
        }
    }

    // Check for missing required keys
    if crate_name.is_none() {
        errors.push(ParseConfigErrorDetail::MissingRequiredKey(String::from(
            "crate-name",
        )));
    }
    if binary.is_none() {
        errors.push(ParseConfigErrorDetail::MissingRequiredKey(String::from(
            "binary",
        )));
    }
    if root_module.is_none() {
        errors.push(ParseConfigErrorDetail::MissingRequiredKey(String::from(
            "root_module",
        )));
    }

    let build = match (tool, native_enabled, target_enabled) {
        (Some(true), Some(false), _) | (Some(true), _, Some(false)) => {
            if native_enabled.is_some() {
                errors.push(ParseConfigErrorDetail::ConflictingKeys {
                    key1: String::from("tool"),
                    key2: String::from("native"),
                });
            }
            if target_enabled.is_some() {
                errors.push(ParseConfigErrorDetail::ConflictingKeys {
                    key1: String::from("tool"),
                    key2: String::from("target"),
                });
            }

            BuildConfiguration::Tool
        }
        (Some(true), _, _) => BuildConfiguration::Tool,
        (_, native, target) => BuildConfiguration::General {
            native: native.unwrap_or(false),
            target: target.unwrap_or(false),
        },
    };

    if !errors.is_empty() {
        return Err(ParseConfigError::ValidationErrors(errors));
    }

    let config = Config {
        crate_name: crate_name.unwrap(),
        libraries,
        binary: binary.unwrap(),
        build,
        format,
        regenerate,
        root_module: root_module.unwrap(),
    };

    Ok(config)
}

/// The valid configuration structure.
#[derive(Debug)]
pub struct Config {
    crate_name: String,
    libraries: Vec<String>,
    binary: bool,
    build: BuildConfiguration,
    format: bool,
    regenerate: bool,
    root_module: String,
}

impl Config {
    pub const fn crate_name(&self) -> &String {
        &self.crate_name
    }

    pub const fn libraries(&self) -> &Vec<String> {
        &self.libraries
    }

    pub const fn binary(&self) -> bool {
        self.binary
    }

    pub const fn library(&self) -> bool {
        !self.binary
    }

    pub const fn tool(&self) -> bool {
        matches!(self.build, BuildConfiguration::Tool)
    }

    pub const fn native(&self) -> bool {
        match self.build {
            BuildConfiguration::Tool => true,
            BuildConfiguration::General {
                native: true,
                target: _,
            } => true,
            BuildConfiguration::General {
                native: false,
                target: _,
            } => false,
        }
    }

    pub const fn target(&self) -> bool {
        match self.build {
            BuildConfiguration::Tool => false,
            BuildConfiguration::General {
                native: _,
                target: true,
            } => true,
            BuildConfiguration::General {
                native: _,
                target: false,
            } => false,
        }
    }

    pub const fn format(&self) -> bool {
        self.format
    }

    pub const fn regenerate(&self) -> bool {
        self.regenerate
    }

    pub const fn root_module(&self) -> &String {
        &self.root_module
    }
}

/// The possible uses for the build.
#[derive(Debug)]
enum BuildConfiguration {
    /// The crate is a tool.
    Tool,
    /// The crate is for general use.
    General {
        /// The crate should support compilation for the native environment.
        native: bool,
        /// The crate should support compilation for the `revm-stub` and `revm` environments.
        target: bool,
    },
}

/// Various errors that can occur while parsing a [`Config`] input.
#[derive(Debug)]
pub enum ParseConfigError {
    /// Syntax errors.
    ParseKeyValueErrors(Vec<ParseKeyValueError>),
    /// Structural validation errors.
    ValidationErrors(Vec<ParseConfigErrorDetail>),
}

impl From<Vec<ParseKeyValueError>> for ParseConfigError {
    fn from(errors: Vec<ParseKeyValueError>) -> Self {
        Self::ParseKeyValueErrors(errors)
    }
}

impl fmt::Display for ParseConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ParseKeyValueErrors(errors) => {
                writeln!(f, "syntax errors")?;
                for error in errors {
                    writeln!(f, "{error}")?;
                }
            }
            Self::ValidationErrors(errors) => {
                writeln!(f, "validation errors")?;
                for error in errors {
                    writeln!(f, "{error}")?;
                }
            }
        }

        Ok(())
    }
}

impl error::Error for ParseConfigError {}

/// Configuration violations identified during the validation pass.
#[derive(Debug)]
pub enum ParseConfigErrorDetail {
    /// A key that expects at least one value was not supplied a value.
    MissingRequiredKey(String),
    /// A key that expects a single value was supplied more than one.
    DuplicateKey(String),
    /// The supplied value for the key does is not of a type supported by said key.
    TypeMismatch {
        /// The key of the value that is a mismatch.
        key: String,
        /// The expected type of the value.
        expected: &'static str,
    },
    /// Conflicting keys were specified together.
    ConflictingKeys {
        /// The first of the conflicting keys.
        key1: String,
        /// The second of the conflicting keys.
        key2: String,
    },
    /// An unrecognized or undefined key was provided in the input.
    UnknownKey(String),
}

impl fmt::Display for ParseConfigErrorDetail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MissingRequiredKey(key) => {
                write!(f, "missing required key: '{key}'")
            }
            Self::DuplicateKey(key) => {
                write!(
                    f,
                    "duplicate key provided: '{key}' expected only a single value"
                )
            }
            Self::TypeMismatch { key, expected } => {
                write!(f, "type mismatch for key '{key}': expected a {expected}")
            }
            Self::ConflictingKeys { key1, key2 } => {
                write!(
                    f,
                    "conflicting keys specified: '{key1}' cannot be used with '{key2}'"
                )
            }
            Self::UnknownKey(key) => {
                write!(f, "unrecognized key: '{key}'")
            }
        }
    }
}

/// Parses the key-value list line by line, returning the list of all values associated with each
/// key.
///
/// # Errors
///
/// - [`ParseKeyValueError::InvalidKeyValuePair`]: Returned if the provided key-value pair is
///   invalid.
/// - [`ParseKeyValueError::UnsupportedValueType`]: Returned if the provided value is of an
///   unsupported type.
fn parse_key_value_list(s: &str) -> Result<HashMap<&str, Vec<DataType>>, Vec<ParseKeyValueError>> {
    let mut map = HashMap::new();
    let mut errors = Vec::new();

    for line in s.lines() {
        // Strip out trailing inline comments starting with '#'.
        let line = line.split_once('#').map(|(s, _)| s).unwrap_or_else(|| line);
        let line = line.trim();

        // Skip lines that are empty or that only contain comments.
        if line.is_empty() {
            continue;
        }

        // Isolate the key and value using the assignment delimiter.
        let Some((key, value)) = line.split_once("::=") else {
            errors.push(ParseKeyValueError::InvalidKeyValuePair(String::from(line)));
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        // Classify the value into the supported primitive types.
        let value = if let Some(value) = value.strip_circumfix('"', '"') {
            DataType::String(String::from(value))
        } else if value == "true" {
            DataType::Bool(true)
        } else if value == "false" {
            DataType::Bool(false)
        } else {
            errors.push(ParseKeyValueError::UnsupportedValueType(String::from(
                value,
            )));
            continue;
        };

        // Group values by their associated keys in order to handle multi-value keys.
        map.entry(key).or_insert(Vec::new()).push(value);
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(map)
}

/// The various datatypes that the key-value list supports.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DataType {
    /// Represents a standard enclosed string literal.
    String(String),
    /// Represents `true` or `false` values.
    Bool(bool),
}

/// Various errors that can occur while parsing a line of the configuration file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseKeyValueError {
    /// The provided key-value pair is invalid.
    InvalidKeyValuePair(String),
    /// The provided value type is unsupported.
    UnsupportedValueType(String),
}

impl fmt::Display for ParseKeyValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidKeyValuePair(s) => {
                write!(f, "invalid key-value seperator: expected '::=', got {s}")
            }
            Self::UnsupportedValueType(s) => write!(
                f,
                "invalid value type: only strings, bools, and integers are supported: {s}"
            ),
        }
    }
}

impl error::Error for ParseKeyValueError {}

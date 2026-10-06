//! Utility structures to ensure properly formatted output.

use std::{
    borrow::Cow,
    ffi::OsStr,
    path::{Component, Path},
};

use crate::platform::os_string_to_bytes;

/// The overall structure of the `build.ninja` file.
#[derive(Clone, Debug)]
pub struct NinjaFile<'a> {
    /// The top-level [`Variable`]s that are available for all [`Build`], [`Rule`], [`Variable`],
    /// and default statements.
    variables: Vec<Variable<'a>>,
    /// The [`Rule`]s that this [`NinjaFile`] provides.
    rules: Vec<Rule<'a>>,
    /// The [`Build`]s that this [`NinjaFile`] provides.
    build: Vec<Build<'a>>,
    /// The [`FilePath`] that should be built whenever no [`FilePath`] is provided to `ninja`.
    default: Vec<FilePath>,
}

impl<'a> NinjaFile<'a> {
    /// Creates an empty [`NinjaFile`].
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            rules: Vec::new(),
            build: Vec::new(),
            default: Vec::new(),
        }
    }

    /// Adds the provided `variable` to the list of top-level [`Variable`]s.
    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    /// Adds the provided `rule` to the list of [`Rule`]s.
    pub fn add_rule(&mut self, rule: Rule<'a>) {
        self.rules.push(rule)
    }

    /// Adds the provided `build` to the list of [`Build`]s.
    pub fn add_build(&mut self, build: Build<'a>) {
        self.build.push(build)
    }

    /// Adds the provided `default` to the list of [`FilePath`]s.
    pub fn add_default(&mut self, path: FilePath) {
        self.default.push(path)
    }

    /// Writes the formatted contents of this [`NinjaFile`] to the provided `output`.
    pub fn write_out(&self, output: &mut Vec<u8>) {
        let mut needs_newline = false;

        for variable in &self.variables {
            variable.write_out(output);
            needs_newline = true;
        }

        if needs_newline && !self.rules.is_empty() {
            output.push(b'\n');
            needs_newline = false;
        }

        for (index, rule) in self.rules.iter().enumerate() {
            rule.write_out(output);
            if index != self.rules.len() - 1 {
                output.push(b'\n');
            }

            needs_newline = true;
        }

        if needs_newline && !self.build.is_empty() {
            output.push(b'\n');
            needs_newline = false;
        }

        for (index, build) in self.build.iter().enumerate() {
            build.write_out(output);
            if index != self.build.len() - 1 {
                output.push(b'\n');
            }

            needs_newline = true;
        }

        if needs_newline && !self.default.is_empty() {
            output.push(b'\n');
        }

        if !self.default.is_empty() {
            output.extend_from_slice("default".as_bytes());
            for default in &self.default {
                output.push(b' ');
                default.write_out(output);
            }

            output.push(b'\n');
        }
    }
}

/// A `ninja` rule statement.
#[derive(Clone, Debug)]
pub struct Rule<'a> {
    /// The name of the [`Rule`].
    name: Cow<'a, str>,
    /// The list of [`Variable`] that determine how each [`Rule`] works.
    variables: Vec<Variable<'a>>,
}

impl<'a> Rule<'a> {
    /// Creates a new [`Rule`] with the provided `name`.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            variables: Vec::new(),
        }
    }

    /// Adds the provided `variable` to the list of [`Variable`]s.
    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    /// Writes the formatted contents of this [`Rule`] to the provided `output`.
    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice("rule ".as_bytes());
        output.extend_from_slice(self.name.as_bytes());
        output.push(b'\n');

        for variable in &self.variables {
            output.extend_from_slice("  ".as_bytes());
            variable.write_out(output);
        }
    }
}

/// A `ninja` build statement.
#[derive(Clone, Debug)]
pub struct Build<'a> {
    /// The name of the [`Rule`] that this [`Build`] statement utilizes to do the processing
    /// required to generate the outputs from the inputs.
    rule_name: Cow<'a, str>,

    /// The [`FilePath`] outputs produced by this [`Build`] statement that should be added to the
    /// `out` [`Variable`].
    explicit_outputs: Vec<FilePath>,
    /// The [`FilePath`] outputs produced by this [`Build`] statement that should not be added to
    /// the `out` [`Variable`].
    implicit_outputs: Vec<FilePath>,

    /// The [`FilePath`] inputs to this [`Build`] statement that should be added to the `in`
    /// [`Variable`].
    explicit_inputs: Vec<FilePath>,
    /// The [`FilePath`] inputs to this [`Build`] statement that should not be added to the `in`
    /// [`Variable`].
    implicit_inputs: Vec<FilePath>,

    /// The list of [`Variable`]s utilized when executing the [`Rule`].
    variables: Vec<Variable<'a>>,
}

impl<'a> Build<'a> {
    /// Creates a new [`Build`] statement that utilizes the [`Rule`] with the provided `rule_name`.
    pub fn new(rule_name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            rule_name: rule_name.into(),

            explicit_outputs: Vec::new(),
            implicit_outputs: Vec::new(),

            explicit_inputs: Vec::new(),
            implicit_inputs: Vec::new(),

            variables: Vec::new(),
        }
    }

    /// Adds the provided [`FilePath`] as an explicit input.
    pub fn add_input(&mut self, path: FilePath) {
        self.explicit_inputs.push(path)
    }

    /// Adds the provided [`FilePath`] as an explict output.
    pub fn add_output(&mut self, path: FilePath) {
        self.explicit_outputs.push(path);
    }

    /// Adds the provided [`FilePath`] as an implicit input.
    pub fn add_implicit_input(&mut self, path: FilePath) {
        self.implicit_inputs.push(path)
    }

    /// Adds the provided [`FilePath`] as an implict output.
    #[expect(dead_code)]
    pub fn add_implicit_output(&mut self, path: FilePath) {
        self.implicit_outputs.push(path);
    }

    /// Adds the provided `variable` to the list of [`Variable`]s.
    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    /// Writes the formatted contents of this [`Build`] to the provided `output`.
    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice("build".as_bytes());

        for output_path in &self.explicit_outputs {
            output.push(b' ');
            output_path.write_out(output);
        }

        if !self.implicit_outputs.is_empty() {
            output.push(b' ');
            output.push(b'|');
            output.push(b' ');

            for output_path in &self.implicit_outputs {
                output.push(b' ');
                output_path.write_out(output);
            }
        }

        output.push(b':');
        output.push(b' ');

        output.extend_from_slice(self.rule_name.as_bytes());

        for explicit in &self.explicit_inputs {
            output.push(b' ');
            explicit.write_out(output);
        }

        if !self.implicit_inputs.is_empty() {
            output.push(b' ');
            output.push(b'|');

            for implicit in &self.implicit_inputs {
                output.push(b' ');
                implicit.write_out(output);
            }
        }

        output.push(b'\n');
        for variable in &self.variables {
            output.extend_from_slice("  ".as_bytes());
            variable.write_out(output);
        }
    }
}

/// A `ninja` variable definition.
#[derive(Clone, Debug)]
pub struct Variable<'a> {
    /// The name of the [`Variable`].
    name: Cow<'a, str>,
    /// The raw bytes that make up the value associated with this [`Variable`].
    value: Vec<u8>,
}

impl<'a> Variable<'a> {
    /// Creates a new [`Variable`] with the provided `name`.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            value: Vec::new(),
        }
    }

    /// Returns `true` if the [`Variable`]'s value is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Adds the provided [`str`] to the [`Variable`]'s value in a literal fashion (i.e., without
    /// accounting for any escaping `ninja` will perform).
    ///
    /// This is primarily useful for injecting the value of another [`Variable`] into this
    /// [`Variable`]'s content. This injection will be performed at ninja execution time.
    pub fn push_literal_str(&mut self, s: &str) {
        self.value.extend_from_slice(s.as_bytes())
    }

    /// Adds the provided [`str`] to the [`Variable`]'s value, escaping any bytes such that the
    /// portion of the [`Variable`]'s value associated with this call, after escaping, will be the
    /// same as the provided [`str`].
    ///
    /// This is the primary mechanism that should be utilized to add [`str`]s to the [`Variable`]'s
    /// value.
    pub fn push_escaped_str(&mut self, s: &str) {
        self.push_escaped_bytes(s.as_bytes());
    }

    /// Adds the provided [`Path`] to the [`Variable`]'s value, escaping any bytes such that the
    /// portion of the [`Variable`]'s value associated with this call, after escaping, will be
    /// recognizable to `ninja` as a path (valid for items such as `builddir` and other `ninja` file
    /// paths (since `ninja` doesn't support '\' paths except in rule executions.
    pub fn push_escaped_path(&mut self, path: impl AsRef<Path>) {
        let starting_index = self.value.len();
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(_) => todo!("implement support for path prefixes"),
                Component::RootDir => {
                    assert_eq!(
                        self.value.len(),
                        starting_index,
                        "interior root directories are not allowed"
                    );
                    self.push_escaped_str("/")
                }
                Component::CurDir => {
                    if self.value.len() != starting_index
                        && self.value.last().is_some_and(|&b| b != b'/')
                    {
                        self.push_escaped_str("/");
                    }
                    self.push_escaped_str(".")
                }
                Component::ParentDir => {
                    if self.value.len() != starting_index
                        && self.value.last().is_some_and(|&b| b != b'/')
                    {
                        self.push_escaped_str("/");
                    }
                    self.push_escaped_str("..")
                }
                Component::Normal(path) => {
                    if self.value.len() != starting_index
                        && self.value.last().is_some_and(|&b| b != b'/')
                    {
                        self.push_escaped_str("/");
                    }

                    self.push_escaped_bytes(&os_string_to_bytes(path))
                }
            }
        }
    }

    /// Adds the escaped [`Argument`] to the [`Variable`]'s value, escaping any bytes such that the
    /// exact contents of the [`Argument`] will be provided to `ninja`'s execution method.
    pub fn push_escaped_argument(&mut self, arg: &Argument) {
        #[cfg(unix)]
        {
            self.value.push(b'\'');
            for &byte in arg.0.iter() {
                if byte == b'\'' {
                    self.push_escaped_byte(b'\'');
                    self.push_escaped_byte(b'\\');
                    self.push_escaped_byte(b'\'');
                    self.push_escaped_byte(b'\'');
                } else {
                    self.push_escaped_byte(byte);
                }
            }
            self.value.push(b'\'');
        }
    }

    /// Adds the provided `bytes` to the [`Variable`]'s value, escaping any bytes such that the
    /// portion of the [`Variable`]'s value associated with this call, after escaping, will be the
    /// same as the provided `bytes` slice.
    fn push_escaped_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.push_escaped_byte(byte);
        }
    }

    /// Adds the provided `byte` to the [`Variable`]'s value, escaping the added portion such that
    /// the protion of the [`Variable`]'s value associated with this call, after escaping, will be
    /// the same as the provided `byte`.
    fn push_escaped_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.value.push(b'$');
                self.value.push(b'\n');
            }
            b'$' => {
                self.value.push(b'$');
                self.value.push(b'$');
            }
            byte => self.value.push(byte),
        }
    }

    /// Writes the formatted contents of this [`Variable`] to the provided `output`.
    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(self.name.as_bytes());
        output.extend_from_slice(" = ".as_bytes());
        output.extend_from_slice(&self.value);
        output.push(b'\n');
    }
}

/// An argument properly escaped to be passed correctly through to the [`Rule`]'s execution
/// statement.
#[derive(Clone, Debug)]
pub struct Argument(Vec<u8>);

impl Argument {
    /// Creates a new empty [`Argument`].
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates a new [`Argument`] that starts with the provided [`str`].
    pub fn new_str(s: &str) -> Self {
        let mut arg = Self::new();
        arg.push_str(s);
        arg
    }

    /// Creates a new [`Argument`] that starts with the provided [`OsStr`].
    pub fn new_os_str(os_str: impl AsRef<OsStr>) -> Self {
        let mut arg = Self::new();
        arg.push_os_str(os_str);
        arg
    }

    /// Creates a new [`Argument`] that starts with the provided [`Path`].
    pub fn new_path(path: impl AsRef<Path>) -> Self {
        let mut arg = Self::new();
        arg.push_path(path);
        arg
    }

    /// Adds the provided [`str`] to the [`Argument`]'s contents.
    pub fn push_str(&mut self, s: &str) {
        self.push_bytes(s.as_bytes());
    }

    /// Adds the provided [`OsStr`] to the [`Argument`]'s contents.
    pub fn push_os_str(&mut self, os_str: impl AsRef<OsStr>) {
        let bytes = os_string_to_bytes(os_str.as_ref());
        self.push_bytes(&bytes);
    }

    /// Adds the provided [`Path`] to the [`Argument`]'s contents.
    pub fn push_path(&mut self, path: impl AsRef<Path>) {
        self.push_os_str(path.as_ref().as_os_str());
    }

    /// Adds the provided `bytes` to the [`Argument`]'s contents.
    fn push_bytes(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

/// A path used in a `build` statement.
#[derive(Clone, Debug)]
pub struct FilePath(Vec<u8>);

impl FilePath {
    /// Creates a new empty [`FilePath`].
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates a new [`FilePath`] from the provided literal bytes.
    pub fn from_literal(bytes: impl AsRef<[u8]>) -> Self {
        let mut file_path = Self::new();
        file_path.push_literal(bytes);

        file_path
    }

    /// Creates a new [`FilePath`] from the provided `path`.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let mut file_path = Self::new();

        for component in path.as_ref().components() {
            match component {
                Component::Prefix(_) => todo!("implement support for path prefixes"),
                Component::RootDir => {
                    assert!(
                        file_path.0.is_empty(),
                        "interior root directories are not allowed"
                    );
                    file_path.push_escaped("/")
                }
                Component::CurDir => {
                    if file_path.0.last().is_some_and(|&b| b != b'/') {
                        file_path.push_escaped("/");
                    }
                    file_path.push_escaped(".");
                }
                Component::ParentDir => {
                    if file_path.0.last().is_some_and(|&b| b != b'/') {
                        file_path.push_escaped("/");
                    }
                    file_path.push_escaped("..");
                }
                Component::Normal(path) => {
                    if file_path.0.last().is_some_and(|&b| b != b'/') {
                        file_path.push_escaped("/");
                    }

                    file_path.push_escaped(os_string_to_bytes(path))
                }
            }
        }

        file_path
    }

    /// Adds the provided `byte` into this [`FilePath`]'s contents.
    pub fn push_literal(&mut self, bytes: impl AsRef<[u8]>) {
        self.0.extend_from_slice(bytes.as_ref());
    }

    /// Adds the provided `bytes` into this [`FilePath`]'s contents, escaping if necessary.
    pub fn push_escaped(&mut self, bytes: impl AsRef<[u8]>) {
        for &byte in bytes.as_ref() {
            match byte {
                b'\n' => {
                    self.0.push(b'$');
                    self.0.push(b'\n');
                }
                b' ' => {
                    self.0.push(b'$');
                    self.0.push(b' ');
                }
                b':' => {
                    self.0.push(b'$');
                    self.0.push(b':');
                }
                b'$' => {
                    self.0.push(b'$');
                    self.0.push(b'$');
                }
                _ => self.0.push(byte),
            }
        }
    }

    /// Writes the formatted contents of this [`FilePath`] to the provided `output`.
    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.0);
    }
}

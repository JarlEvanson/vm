//! Utility structures for outputting a structured `ninja` file.

use std::{borrow::Cow, path::Path};

use crate::os_str_to_bytes;

#[derive(Clone, Debug)]
pub struct NinjaFile<'a> {
    variables: Vec<Variable<'a>>,
    rules: Vec<Rule<'a>>,
    build: Vec<Build<'a>>,
    default: Vec<FilePath>,
}

impl<'a> NinjaFile<'a> {
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            rules: Vec::new(),
            build: Vec::new(),
            default: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    pub fn add_rule(&mut self, rule: Rule<'a>) {
        self.rules.push(rule)
    }

    pub fn add_build(&mut self, build: Build<'a>) {
        self.build.push(build)
    }

    pub fn add_default(&mut self, path: FilePath) {
        self.default.push(path)
    }

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

#[derive(Clone, Debug)]
pub struct Rule<'a> {
    name: Cow<'a, str>,
    variables: Vec<Variable<'a>>,
}

impl<'a> Rule<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(name.into())
    }

    fn new_inner(name: Cow<'a, str>) -> Self {
        Self {
            name,
            variables: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

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

#[derive(Clone, Debug)]
pub struct Build<'a> {
    rule_name: Cow<'a, str>,

    explicit_outputs: Vec<FilePath>,
    implicit_outputs: Vec<FilePath>,

    explicit: Vec<FilePath>,
    implicit: Vec<FilePath>,

    variables: Vec<Variable<'a>>,
}

impl<'a> Build<'a> {
    pub fn new(rule_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(rule_name.into())
    }

    fn new_inner(rule_name: Cow<'a, str>) -> Self {
        Self {
            rule_name,

            explicit_outputs: Vec::new(),
            implicit_outputs: Vec::new(),

            explicit: Vec::new(),
            implicit: Vec::new(),

            variables: Vec::new(),
        }
    }

    pub fn add_output(&mut self, path: FilePath) {
        self.explicit_outputs.push(path);
    }

    pub fn add_implicit_output(&mut self, path: FilePath) {
        self.implicit_outputs.push(path)
    }

    pub fn add_input(&mut self, path: FilePath) {
        self.explicit.push(path)
    }

    pub fn add_implicit_input(&mut self, path: FilePath) {
        self.implicit.push(path)
    }

    pub fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    pub fn write_out(&self, output: &mut Vec<u8>) {
        assert!(!self.explicit_outputs.is_empty());

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

        for explicit in &self.explicit {
            output.push(b' ');
            explicit.write_out(output);
        }

        if !self.implicit.is_empty() {
            output.push(b' ');
            output.push(b'|');

            for implicit in &self.implicit {
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

#[derive(Clone, Debug)]
pub struct Variable<'a> {
    name: Cow<'a, str>,
    value: Vec<u8>,
}

impl<'a> Variable<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(name.into())
    }

    fn new_inner(name: Cow<'a, str>) -> Self {
        Self {
            name,
            value: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

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
            _ => self.value.push(byte),
        }
    }

    pub fn push_escaped_iter(&mut self, bytes: impl Iterator<Item = u8>) {
        for byte in bytes {
            self.push_escaped_byte(byte);
        }
    }

    pub fn push_literal(&mut self, bytes: impl AsRef<[u8]>) {
        self.value.extend_from_slice(bytes.as_ref())
    }

    pub fn push_escaped(&mut self, bytes: impl AsRef<[u8]>) {
        self.push_escaped_iter(bytes.as_ref().iter().copied());
    }

    pub fn push_escaped_path(&mut self, path: impl AsRef<Path>) {
        let os_str = os_str_to_bytes(path.as_ref().as_os_str());
        self.push_escaped(os_str);
    }

    pub fn push_command_escaped(&mut self, bytes: impl AsRef<[u8]>) {
        self.value.push(b'\'');
        for &byte in bytes.as_ref() {
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

    pub fn push_command_escaped_path(&mut self, path: impl AsRef<Path>) {
        let os_str = os_str_to_bytes(path.as_ref().as_os_str());

        self.value.push(b'\'');
        for byte in os_str {
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

    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(self.name.as_bytes());
        output.extend_from_slice(" = ".as_bytes());
        output.extend_from_slice(&self.value);
        output.push(b'\n');
    }
}

#[derive(Clone, Debug)]
pub struct FilePath(Vec<u8>);

impl FilePath {
    fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_literal(bytes: impl AsRef<[u8]>) -> Self {
        let mut file_path = Self::new();
        file_path.push_literal(bytes);

        file_path
    }

    pub fn from_escaped(bytes: impl AsRef<[u8]>) -> Self {
        let mut file_path = Self::new();
        file_path.push_escaped(bytes);

        file_path
    }

    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let mut file_path = Self::new();
        file_path.push_escaped(os_str_to_bytes(path.as_ref().as_os_str()));

        file_path
    }

    pub fn push_literal(&mut self, bytes: impl AsRef<[u8]>) {
        self.0.extend_from_slice(bytes.as_ref());
    }

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

    pub fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.0);
    }
}

//! Command line parsing and manipulation utilities.

use core::iter::Take;

/// Parsed representation of the provided command line, with special handling for the command line
/// to be passed to `revm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandLineArguments<'a> {
    /// The underlying command line.
    command_line: &'a str,
    /// The number of arguments before the start of `revm`'s command line.
    revm_arguments_start: Option<usize>,
}

impl<'a> CommandLineArguments<'a> {
    /// Creates a new [`CommandLineArguments`], locating the start of `revm_command_line`.
    pub fn new(command_line: &'a str) -> Self {
        let mut command_line_arguments = Self {
            command_line,
            revm_arguments_start: None,
        };

        let mut arguments_iter = command_line_arguments
            .arguments_internal()
            .enumerate()
            .peekable();
        while let Some((index, argument)) = arguments_iter.next() {
            if argument == "--" {
                if arguments_iter.peek().is_some() {
                    command_line_arguments.revm_arguments_start = Some(index + 1);
                }

                break;
            }
        }

        command_line_arguments
    }

    /// Returns an [`Iterator`] over `revm-stub`'s arguments, excluding the command line to be
    /// passed to `revm`.
    pub fn arguments(&self) -> CommandLineArgumentsIter<'a> {
        CommandLineArgumentsIter(
            self.arguments_internal().take(
                self.revm_arguments_start
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(usize::MAX),
            ),
        )
    }

    /// Returns the command line that should be passed to `revm`.
    pub fn revm_command_line(&self) -> &'a str {
        let Some(revm_arguments_start) = self.revm_arguments_start else {
            // Return an empty string tied to `self.command_line`.
            return &self.command_line[self.command_line.len()..];
        };

        let mut args = self.arguments_internal();

        let mut skipped = 0;
        while skipped < revm_arguments_start {
            let _ = args.next();
            skipped += 1;
        }

        args.remaining_command_line.trim_start_matches(' ')
    }

    /// Returns an [`Iterator`] over all arguments in this [`CommandLineArguments`], including the
    /// command line arguments to be passed to `revm`.
    fn arguments_internal(&self) -> CommandLineArgumentsInternalIter<'a> {
        CommandLineArgumentsInternalIter {
            remaining_command_line: self.command_line,
        }
    }
}

/// An [`Iterator`] over the command line arguments in the associated [`CommandLineArguments`] that
/// are to be interpreted by `revm-stub` (as opposed to `revm`).
#[derive(Clone, Debug)]
pub struct CommandLineArgumentsIter<'a>(Take<CommandLineArgumentsInternalIter<'a>>);

impl<'a> Iterator for CommandLineArgumentsIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// An [`Iterator`] over all of the command line arguments in the associated
/// [`CommandLineArguments`].
#[derive(Clone, Debug)]
struct CommandLineArgumentsInternalIter<'a> {
    /// The portion of the command line that has not yet been parsed.
    remaining_command_line: &'a str,
}

impl<'a> Iterator for CommandLineArgumentsInternalIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_command_line.is_empty() {
            return None;
        }

        let command_line = self.remaining_command_line.trim_start_matches(' ');

        let mut chars = command_line.chars();
        for c in chars.by_ref() {
            if c == ' ' {
                break;
            }
        }

        let remaining_command_line = chars.as_str();
        let argument = &command_line
            [..self.remaining_command_line.len() - remaining_command_line.len()]
            .trim_end_matches(' ');

        if argument.is_empty() {
            // If the argument is empty, then the remaining command line must not contain any
            // additional arguments.
            self.remaining_command_line = &command_line[command_line.len()..];
            return None;
        }

        self.remaining_command_line = remaining_command_line;
        Some(argument)
    }
}

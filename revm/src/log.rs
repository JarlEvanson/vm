//! Code implementing the logging solution for `revm`.

use core::fmt::{self, Write};

use stub_api::Status;

use crate::stub_protocol::generic_table;

/// Logs a message with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::log::_log(
        $crate::log::LogLevel::Trace,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::log::_log(
        $crate::log::LogLevel::Debug,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::log::_log(
        $crate::log::LogLevel::Info,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::log::_log(
        $crate::log::LogLevel::Warn,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Error`].
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::log::_log(
        $crate::log::LogLevel::Error,
        format_args!($($arg)*))
    );
}

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    // Ignore any logging errors because there is no method to report or deal with them.
    let _ = match level {
        LogLevel::Trace => LogImpl.write_fmt(format_args!("TRACE: {args}\n")),
        LogLevel::Debug => LogImpl.write_fmt(format_args!("DEBUG: {args}\n")),
        LogLevel::Info => LogImpl.write_fmt(format_args!("INFO : {args}\n")),
        LogLevel::Warn => LogImpl.write_fmt(format_args!("WARN : {args}\n")),
        LogLevel::Error => LogImpl.write_fmt(format_args!("ERROR: {args}\n")),
    };
}

/// Various levels to determine the priority of information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Designates very low priority information.
    Trace,
    /// Designates lower priority information.
    Debug,
    /// Designates informatory logs.
    Info,
    /// Designates hazardous logs.
    Warn,
    /// Designates very serious logs.
    Error,
}

/// Zero-sized structure to implement `revm`'s logging mechanism.
struct LogImpl;

impl fmt::Write for LogImpl {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        /*
        if let Some(generic_table) = generic_table() {
            // SAFETY:
            //
            // The REVM protocol ensures that the function pointer is valid and the provided
            // arguments point to a buffer of valid UTF-8 that is at least as long as `s.len()`.
            let result = unsafe { (generic_table.write)(s.as_ptr(), s.len()) };
            if result != Status::SUCCESS {
                return Err(fmt::Error);
            }
        }
        */

        // SAFETY:
        //
        // TODO:
        unsafe { x86_common::io_port::write_u8_slice(0xe9, s.as_bytes()) }

        Ok(())
    }
}

//! Early logging solution.

use core::fmt::{self, Write};

use crate::{log::LogLevel, stub_protocol::generic_table};

/// Logs a message with [`LogLevel::Trace`].
#[allow(unused_macros)]
macro_rules! early_trace {
    ($($arg:tt)*) => ($crate::stub_protocol::log::_log(
        $crate::log::LogLevel::Trace,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Debug`].
#[allow(unused_macros)]
macro_rules! early_debug {
    ($($arg:tt)*) => ($crate::stub_protocol::log::_log(
        $crate::log::LogLevel::Debug,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Info`].
#[allow(unused_macros)]
macro_rules! early_info {
    ($($arg:tt)*) => ($crate::stub_protocol::log::_log(
        $crate::log::LogLevel::Info,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Warn`].
#[allow(unused_macros)]
macro_rules! early_warn {
    ($($arg:tt)*) => ($crate::stub_protocol::log::_log(
        $crate::log::LogLevel::Warn,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Error`].
#[allow(unused_macros)]
macro_rules! early_error {
    ($($arg:tt)*) => ($crate::stub_protocol::log::_log(
        $crate::log::LogLevel::Error,
        format_args!($($arg)*))
    );
}

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    if level < LogLevel::Debug {
        return;
    }

    let mut buffer = LogBuffer {
        buffer: [0; 4096],
        length: 0,
        truncated: false,
    };

    // Ignore any logging errors because there is no method to report or deal with them.
    let _ = match level {
        LogLevel::Trace => buffer.write_fmt(format_args!("TRACE: {args}")),
        LogLevel::Debug => buffer.write_fmt(format_args!("DEBUG: {args}")),
        LogLevel::Info => buffer.write_fmt(format_args!("INFO : {args}")),
        LogLevel::Warn => buffer.write_fmt(format_args!("WARN : {args}")),
        LogLevel::Error => buffer.write_fmt(format_args!("ERROR: {args}")),
    };

    if buffer.truncated {
        let marker = b"<truncated>";
        let marker_length = marker.len().min(buffer.buffer.len());

        let start_pos = buffer.buffer.len().saturating_sub(marker_length);
        buffer.buffer[start_pos..][..marker_length].copy_from_slice(&marker[..marker_length]);
    }

    if let Some(generic_table) = generic_table() {
        // Ignore the result: At this stage, handling it isn't really possible.
        // SAFETY:
        //
        // The REVM protocol ensures that the function pointer is valid and the provided
        // arguments point to a buffer of valid UTF-8 that is at least as long as `s.len()`.
        let _ = unsafe { (generic_table.write)(buffer.buffer.as_ptr(), buffer.length) };
    }
}

/// Buffered logging system for early `revm` logging.
struct LogBuffer<const SIZE: usize> {
    /// The buffer in which the log message is stored.
    buffer: [u8; SIZE],
    /// The length, in bytes, of the formatted message.
    length: usize,
    /// If `true`, the message has been truncated.
    truncated: bool,
}

impl<const SIZE: usize> fmt::Write for LogBuffer<SIZE> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let write_amount = s.len().min(self.buffer.len() - self.length);
        if write_amount < s.len() {
            self.truncated = true;
        }

        self.buffer[self.length..][..write_amount].copy_from_slice(&s.as_bytes()[..write_amount]);
        self.length += write_amount;

        if !self.truncated {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

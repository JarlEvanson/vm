//! Definitions and interfaces that platforms use to provide logging services in a platform
//! agnostic manner.

use core::{
    fmt::{self, Write},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

use sync::Spinlock;

/// The head of the [`Console`] list.
static CONSOLE_HEAD: AtomicPtr<Console> = AtomicPtr::new(ptr::null_mut());
/// The print buffer.
static BUFFER: Spinlock<WriteBuffer> = Spinlock::new(WriteBuffer {
    buffer: [0; 4096],
    written: 0,
});

/// Implementation of a fixed-size printing buffer.
struct WriteBuffer {
    /// The bytes that compose the message.
    buffer: [u8; 4096],
    /// The number of bytes written to this [`WriteBuffer`].
    written: usize,
}

impl fmt::Write for WriteBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let buffer = &mut self.buffer[self.written..];
        if buffer.len() < s.len() {
            return Err(fmt::Error);
        }

        buffer[..s.len()].copy_from_slice(s.as_bytes());
        self.written += s.len();
        Ok(())
    }
}

/// Registers a console with the logging system.
///
/// # Safety
///
/// - There must be zero overlapping calls to any other logging subsystem function.
/// - The provided `console` must not already be in the logging subsystem.
/// - The provided `console` must be placed under the exclusive control of the logging subsystem.
pub unsafe fn register_console(mut console: NonNull<Console>) {
    let head = CONSOLE_HEAD.load(Ordering::Relaxed);

    // SAFETY:
    //
    // The provided [`Console`] has been placed under the exclusive control of the logging
    // subsystem.
    let console_mut = unsafe { console.as_mut() };
    console_mut.next = NonNull::new(head);
    CONSOLE_HEAD.store(console.as_ptr(), Ordering::Release);
}

/// Deregisters a console with the logging system.
///
/// # Safety
///
/// There must be zero overlapping calls to any other logging subsystem function.
#[expect(
    clippy::missing_panics_doc,
    reason = "panic only occurs due to programmer error"
)]
pub unsafe fn deregister_console(console: NonNull<Console>) {
    let mut prev: Option<&mut Console> = None;
    let mut current_ptr = CONSOLE_HEAD.load(Ordering::Acquire);

    while let Some(mut current) = NonNull::new(current_ptr) {
        // SAFETY:
        //
        // There are zero overlapping calls to the logging subsystem functions and thus it is safe
        // to mutably access the [`Console`] list.
        let current_mut = unsafe { current.as_mut() };
        if current == console {
            if let Some(prev) = prev {
                prev.next = current_mut.next;
            } else {
                CONSOLE_HEAD.store(
                    current_mut.next.map(NonNull::as_ptr).unwrap_or_default(),
                    Ordering::Release,
                );
            }
            return;
        }

        current_ptr = current_mut.next.map(NonNull::as_ptr).unwrap_or_default();
        prev = Some(current_mut);
    }

    panic!("attempted to deregister an unregistered console")
}

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    if level < LogLevel::Trace {
        return;
    }

    let mut buffer = BUFFER.lock();
    buffer.written = 0;

    let metadata = Metadata { level };
    let _ = writeln!(&mut buffer, "{args}");
    // SAFETY:
    //
    // WriteBuffer ensures that the bytes in the range `0..buffer.written` have been initialized to
    // UTF-8.
    let message = unsafe { core::str::from_utf8_unchecked(&buffer.buffer[..buffer.written]) };

    let mut console_ptr = NonNull::new(CONSOLE_HEAD.load(Ordering::Acquire));
    while let Some(console) = console_ptr {
        // SAFETY:
        //
        // [`Console`]s are only access immutably.
        let console_ref = unsafe { console.as_ref() };
        let next = console_ref.next;

        (console_ref.write)(console, metadata, message);

        console_ptr = next;
    }

    drop(buffer)
}

/// Logs a message with [`LogLevel::Trace`].
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Trace,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Debug`].
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Debug,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Info`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Info,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Warn`].
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Warn,
        format_args!($($arg)*))
    );
}

/// Logs a message with [`LogLevel::Error`].
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::platform::_log(
        $crate::platform::LogLevel::Error,
        format_args!($($arg)*))
    );
}

/// Information relevant to the associated log message.
#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    /// The [`LogLevel`] of the associated log message.
    pub level: LogLevel,
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

/// The interface for a text-based output device.
pub struct Console {
    /// The function to output the provided string onto the device.
    write: fn(this: NonNull<Self>, metadata: Metadata, str: &str),

    /// Link in the console list.
    next: Option<NonNull<Console>>,
}

// SAFETY:
//
// This is safe to read from multiple threads.
unsafe impl Sync for Console {}
// SAFETY:
//
// The data contained in this thread can safely be sent across threads.
unsafe impl Send for Console {}

impl Console {
    /// Initializes a new [`Console`] structure with the provided function.
    pub const fn new(write: fn(this: NonNull<Self>, metadata: Metadata, str: &str)) -> Self {
        Self { write, next: None }
    }
}

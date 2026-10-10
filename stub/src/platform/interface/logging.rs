//! Definitions and interfaces that platforms use to provide logging services in a platform
//! agnostic manner.

use core::{
    fmt::{self, Write},
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use sync::{ControlledModificationCell, RwSpinlock};

/// Atomic representation of the global [`LogLevel`] used for filtering.
static LOG_LEVEL: AtomicU8 = AtomicU8::new(0);
/// The [`ConsoleList`] used by the global logging subsystem.
static CONSOLE_LIST: RwSpinlock<ConsoleList> = RwSpinlock::new(ConsoleList::new());

/// Registers the provided [`Console`] instance with the global logging subsystem.
pub fn register_console<T: Console + Sync>(console: Pin<&'static T>) {
    CONSOLE_LIST.write().register_console(console);
}

/// Deregisters the provided [`Console`] instance with the global logging subsystem.
pub fn deregister_console<T: Console + Sync>(console: Pin<&'static T>) {
    CONSOLE_LIST.write().deregister_console(console);
}

/// Sets the [`LogLevel`] used for filtering log messages globally.
pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed)
}

/// Retrieves the current global [`LogLevel`] used for filtering.
pub fn get_log_level() -> LogLevel {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        0 => LogLevel::Trace,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        _ => unreachable!(),
    }
}

/// The interface required by any console implementation.
///
/// # Safety
///
/// `CONSOLE_INTERNAL_OFFSET` must correspond the the offset from the base of `T` to the instance of
/// [`ConsoleInternal`] that this [`Console`] implementation utilizes.
pub unsafe trait Console {
    /// The offset to the [`ConsoleInternal`] instance.
    const CONSOLE_INTERNAL_OFFSET: usize;

    /// Writes the provided message and metadata to the [`Console`] output.
    fn write(self: Pin<&Self>, metadata: Metadata, s: &str);
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

#[doc(hidden)]
pub fn _log(level: LogLevel, args: fmt::Arguments) {
    if level < get_log_level() {
        return;
    }

    let mut buffer = WriteBuffer {
        buffer: [0; 4096],
        written: 0,
    };

    let metadata = Metadata { level };
    let _ = writeln!(&mut buffer, "{args}");

    // SAFETY:
    //
    // WriteBuffer ensures that the bytes in the range `0..buffer.written` have been initialized to
    // UTF-8.
    let message = unsafe { core::str::from_utf8_unchecked(&buffer.buffer[..buffer.written]) };

    let console_list = CONSOLE_LIST.read();

    for console_internal_ptr in console_list.iter() {
        // SAFETY:
        //
        // The `console_internal_ptr` was derived from a pinned [`Console`] and since it is in the
        // list, it has not been dropped. [`CONSOLE_LIST`] is locked and thus only immutable access
        // to the [`Console`] can be obtained.
        let console_internal = unsafe { console_internal_ptr.as_ref() };
        let console_internal_offset = *console_internal.console_internal_offset.get();
        // SAFETY:
        //
        // The [`ConsoleInternal`] structure is in a pinned [`Console`] part of a [`ConsoleList`]
        // and thus has been initialized.
        let write_fn_ref = unsafe { console_internal.write.get().assume_init_ref() };

        // SAFETY:
        //
        // The [`ConsoleInternal`] structure is in a pinned [`Console`] part of a [`ConsoleList`]
        // and thus has been properly initialized.
        let console_ptr = unsafe {
            console_internal_ptr
                .byte_add(console_internal_offset)
                .cast::<()>()
        };

        write_fn_ref(console_ptr, metadata, message);
    }
}

/// The internal write function.
type WriteFunc = fn(ptr: NonNull<()>, metadata: Metadata, s: &str);

/// The internal state required for registering and utilizing the [`Console`] API for logging.
pub struct ConsoleInternal {
    /// [`AtomicBool`] acting as a flag controlled whether this [`ConsoleInternal`] is part of a
    /// [`ConsoleList`].
    active: AtomicBool,

    /// The offset from the [`Console`] to the [`ConsoleInternal`] instance.
    console_internal_offset: ControlledModificationCell<usize>,
    /// The wrapper around the [`Console::write()`] function to enable calling a generic function
    /// without knowing the [`Console`] type.
    write: ControlledModificationCell<MaybeUninit<WriteFunc>>,

    /// The link in the [`ConsoleList`] used to navigate to the next [`ConsoleInternal`] instance.
    link: ControlledModificationCell<Option<NonNull<ConsoleInternal>>>,

    /// Phantom marker that [`ConsoleInternal`] is not [`Unpin`][up].
    ///
    /// [up]: core::marker::Unpin
    _pinned: PhantomPinned,
}

impl ConsoleInternal {
    /// Creates a new [`ConsoleInternal`].
    pub const fn new() -> Self {
        Self {
            active: AtomicBool::new(false),

            console_internal_offset: ControlledModificationCell::new(0),
            write: ControlledModificationCell::new(MaybeUninit::uninit()),

            link: ControlledModificationCell::new(None),

            _pinned: PhantomPinned,
        }
    }
}

impl Drop for ConsoleInternal {
    fn drop(&mut self) {
        assert!(
            !self.active.load(Ordering::Acquire),
            "consoles must not be dropped while registered"
        );
    }
}

// SAFETY:
//
// [`ConsoleInternal`] contents are only used in a thread-safe manner.
unsafe impl Send for ConsoleInternal {}
// SAFETY:
//
// [`ConsoleInternal`] contents are only used in a thread-safe manner.
unsafe impl Sync for ConsoleInternal {}

/// The implementation of a linked list of [`Console`]s.
struct ConsoleList {
    /// The start of the linked list.
    head: Option<NonNull<ConsoleInternal>>,
}

impl ConsoleList {
    /// Creates an empty [`ConsoleList`].
    const fn new() -> Self {
        Self { head: None }
    }

    /// Generic method to handle registering a [`Console`] with this [`ConsoleList`].
    fn register_console<T: Console + Sync>(&mut self, console: Pin<&T>) {
        let console_ptr = NonNull::from_ref(console.get_ref());
        self.register_console_internal(
            console_ptr.cast::<()>(),
            T::CONSOLE_INTERNAL_OFFSET,
            Self::write::<T>,
        );
    }

    /// Generic method to handle deregistering a [`Console`] from this [`ConsoleList`].
    fn deregister_console<T: Console + Sync>(&mut self, console: Pin<&T>) {
        let console_ptr = NonNull::from_ref(console.get_ref());
        self.deregister_console_internal(console_ptr.cast::<()>(), T::CONSOLE_INTERNAL_OFFSET);
    }

    /// Non-generic method to handle registering a [`Console`] with this [`ConsoleList`].
    fn register_console_internal(
        &mut self,
        console_ptr: NonNull<()>,
        console_internal_offset: usize,
        write_func: WriteFunc,
    ) {
        // SAFETY:
        //
        // The `console_ptr` was derived from a pinned [`Console`] reference that implements an
        // unsafe trait that specifies that this operation is safe.
        let console_internal_ptr = unsafe {
            console_ptr
                .byte_add(console_internal_offset)
                .cast::<ConsoleInternal>()
        };

        // SAFETY:
        //
        // The `console_internal_ptr` was derived from a pinned [`Console`] reference and thus is
        // safe to utilize as a reference.
        let console_internal = unsafe { console_internal_ptr.as_ref() };
        if console_internal.active.swap(true, Ordering::Acquire) {
            todo!()
        }

        // SAFETY:
        //
        // This context has mutable access to this [`ConsoleList`] and `console_internal` is not in
        // the [`ConsoleList`] yet, so this is the only context that can perform this operation
        // right now.
        unsafe { *console_internal.console_internal_offset.get_mut() = console_internal_offset };
        // SAFETY:
        //
        // This context has mutable access to this [`ConsoleList`] and `console_internal` is not in
        // the [`ConsoleList`] yet, so this is the only context that can perform this operation
        // right now.
        unsafe { console_internal.write.get_mut().write(write_func) };
        // SAFETY:
        //
        // This context has mutable access to this [`ConsoleList`] and `console_internal` is not in
        // the [`ConsoleList`] yet, so this is the only context that can perform this operation
        // right now.
        unsafe { *console_internal.link.get_mut() = self.head };
        self.head = Some(console_internal_ptr);
    }

    /// Non-generic method to handle deregistering a [`Console`] from this [`ConsoleList`].
    fn deregister_console_internal(
        &mut self,
        console_ptr: NonNull<()>,
        console_internal_offset: usize,
    ) {
        // SAFETY:
        //
        // The `console_ptr` was derived from a pinned [`Console`] reference that implements an
        // unsafe trait that specifies that this operation is safe.
        let console_internal_ptr = unsafe {
            console_ptr
                .byte_add(console_internal_offset)
                .cast::<ConsoleInternal>()
        };

        for test_console_internal_ptr in self.iter() {
            if test_console_internal_ptr != console_internal_ptr {
                continue;
            }

            // SAFETY:
            //
            // The `console_internal_ptr` was derived from a pinned [`Console`] and since it is in
            // the list, it has not been dropped.
            let console_internal = unsafe { console_internal_ptr.as_ref() };
            console_internal.active.store(false, Ordering::Release);
            return;
        }

        panic!("attempted to deregister unregistered console")
    }

    /// Returns an [`Iterator`] over the pointers to the [`ConsoleInternal`] instances in this
    /// [`ConsoleList`].
    fn iter(&self) -> Iter<'_> {
        Iter {
            next: self.head,
            _phantom: PhantomData,
        }
    }

    /// The wrapper around the [`Console::write()`] function to enable calling a generic function
    /// without knowing the [`Console`] type.
    fn write<T: Console>(ptr: NonNull<()>, metadata: Metadata, s: &str) {
        // SAFETY:
        //
        // This function is only called when the underlying [`Console`] is in the [`ConsoleList`]
        // and thus has not been dropped.
        let tmp = unsafe { ptr.cast::<T>().as_ref() };
        // SAFETY:
        //
        // `tmp` is in the [`ConsoleList`] list and thus must be pinned.
        let tmp = unsafe { Pin::new_unchecked(tmp) };
        T::write(tmp, metadata, s)
    }
}

impl Default for ConsoleInternal {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY:
//
// [`ConsoleList`] does not send things over boundaries and its own implementation is safe, thus it
// is safe to implement [`Send`].
unsafe impl Send for ConsoleList {}
// SAFETY:
//
// [`ConsoleList`] exposes an API that would require `unsafe` code to break, while not enabling
// mutable access across thread boundaries.
unsafe impl Sync for ConsoleList {}

/// An [`Iterator`] over the pointers to [`ConsoleInternal`] instances in a [`ConsoleList`].
#[derive(Clone)]
struct Iter<'a> {
    /// The pointer to the next [`ConsoleInternal`] instance.
    next: Option<NonNull<ConsoleInternal>>,
    /// Phantom type used for lifetime restriction.
    _phantom: PhantomData<&'a ConsoleList>,
}

impl Iterator for Iter<'_> {
    type Item = NonNull<ConsoleInternal>;

    fn next(&mut self) -> Option<Self::Item> {
        let console_internal_ptr = self.next?;
        // SAFETY:
        //
        // We have immutable access to the [`ConsoleList`] this [`ConsoleInternal`] instance is part
        // of, so this operation is safe.
        let console_internal = unsafe { console_internal_ptr.as_ref() };
        self.next = *console_internal.link.get();

        Some(console_internal_ptr)
    }
}

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

#[cfg(test)]
#[allow(clippy::undocumented_unsafe_blocks)]
mod test {
    use core::{
        mem,
        pin::{Pin, pin},
    };

    use crate::platform::interface::{LogLevel, Metadata};

    use super::{Console, ConsoleInternal, ConsoleList};

    struct ConsoleA {
        data: usize,
        internal: ConsoleInternal,
    }

    unsafe impl Console for ConsoleA {
        const CONSOLE_INTERNAL_OFFSET: usize = mem::offset_of!(ConsoleA, internal);

        fn write(self: Pin<&Self>, metadata: Metadata, s: &str) {
            println!("{} {metadata:?} {s}", self.as_ref().data);
        }
    }

    #[test]
    fn registration() {
        let mut list = ConsoleList::new();

        let console_a = ConsoleA {
            data: 0,
            internal: ConsoleInternal::new(),
        };
        let console_a = pin!(console_a);
        let console_a = console_a.into_ref();

        let console_b = ConsoleA {
            data: 1,
            internal: ConsoleInternal::new(),
        };
        let console_b = pin!(console_b);
        let console_b = console_b.into_ref();

        list.register_console(console_a);
        list.register_console(console_b);

        for console_internal_ptr in list.iter() {
            let console_internal = unsafe { console_internal_ptr.as_ref() };
            let console_internal_offset = *console_internal.console_internal_offset.get();
            let write_fn_ref = unsafe { console_internal.write.get().assume_init_ref() };

            let console_ptr = unsafe {
                console_internal_ptr
                    .byte_add(console_internal_offset)
                    .cast::<()>()
            };
            write_fn_ref(
                console_ptr,
                Metadata {
                    level: LogLevel::Error,
                },
                "a",
            );
        }

        list.deregister_console(console_b);
        list.deregister_console(console_a);
    }
}

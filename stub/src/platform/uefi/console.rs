//! Implementation of [`Console`] using `con_out` provided by boot services.

use core::{mem, pin::Pin, sync::atomic::Ordering};

use crate::platform::{
    Console, ConsoleInternal, Metadata, register_console, uefi::UEFI_SYSTEM_TABLE,
};

static UEFI_CONSOLE: UefiConsole = UefiConsole {
    internal: ConsoleInternal::new(),
};

/// Registers the UEFI `con_out` console in the console subsystem.
pub fn register() {
    register_console(Pin::static_ref(&UEFI_CONSOLE));
}

/// The wrapper structure for the UEFI `con_out` console.
struct UefiConsole {
    /// The internal [`Console`] management data associated with this instance of [`Console`].
    internal: ConsoleInternal,
}

unsafe impl Console for UefiConsole {
    const CONSOLE_INTERNAL_OFFSET: usize = mem::offset_of!(UefiConsole, internal);

    fn write(self: Pin<&Self>, _: Metadata, s: &str) {
        const BUFFER_SIZE: usize = 128;

        let system_table_ptr = UEFI_SYSTEM_TABLE.load(Ordering::Relaxed);
        if system_table_ptr.is_null() {
            return;
        }

        // SAFETY:
        //
        // `system_table_ptr` is not null and was provided in accordance with the UEFI specification
        // and thus it is safe to read `con_out`.
        let con_out_ptr = unsafe { (*system_table_ptr).con_out };
        if con_out_ptr.is_null() {
            return;
        }

        // SAFETY:
        //
        // According to the UEFI specification, `con_out` points to a `SimpleTextOutputProcotol`
        // instance, which means that reading `output_string_func` is safe.
        let output_string_func = unsafe { (*con_out_ptr).output_string };

        let mut buffer = [0; BUFFER_SIZE + 1];
        let mut index = 0;

        let mut chars = s.chars();
        let mut next_char = chars.next();

        let mut new_line_processed = false;
        while let Some(mut c) = next_char.take() {
            if c == '\n' && !new_line_processed {
                new_line_processed = true;

                next_char = Some(c);
                c = '\r';
            } else {
                new_line_processed = false;
            }

            if c.len_utf16() != 1 {
                // Character is unrepresentable in UCS-2 and thus must be replaced with the
                // replacement character.
                c = '\u{FFFD}';
            }

            buffer[index] = c as u16;
            index += 1;

            if index == BUFFER_SIZE {
                let string = &mut buffer[..=index];
                string[index] = 0;

                // Ignore any warnings/errors (we can't fix them and logging them could cause a
                // stack overflow).
                //
                // SAFETY:
                //
                // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
                // pointer, which means it is safe to be called.
                let _ = unsafe { output_string_func(con_out_ptr, string.as_mut_ptr()) };
                index = 0;
            }

            if next_char.is_none() {
                next_char = chars.next();
            }
        }

        if index != 0 {
            let string = &mut buffer[..=index];
            string[index] = 0;

            // Ignore any warnings/errors (we can't fix them and logging them could cause a
            // stack overflow).
            //
            // SAFETY:
            //
            // `output_string_func` was obtained from a valid UEFI SimpleTextOutputProcotol
            // pointer, which means it is safe to be called.
            let _ = unsafe { output_string_func(con_out_ptr, string.as_mut_ptr()) };
        }
    }
}

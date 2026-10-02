use core::{arch::asm, mem, pin::Pin};

use crate::platform::{Console, ConsoleInternal, Metadata};

pub struct DebugCon {
    pub port: u16,
    pub console_internal: ConsoleInternal,
}

unsafe impl Console for DebugCon {
    const CONSOLE_INTERNAL_OFFSET: usize = mem::offset_of!(DebugCon, console_internal);

    fn write(self: Pin<&Self>, _: Metadata, s: &str) {
        let slice = s.as_bytes();
        let port = self.get_ref().port;

        // SAFETY:
        //
        // According to the invariants of the function, this is safe to run.
        unsafe {
            #[cfg(target_arch = "x86")]
            asm!(
                "xchg esi, {ptr}",
                "rep outsb",
                "xchg {ptr}, esi",
                in("dx") port,
                ptr = inout(reg) slice.as_ptr() => _,
                inout("ecx") slice.len() => _,
                options(readonly, nostack, preserves_flags)
            );

            #[cfg(target_arch = "x86_64")]
            asm!(
                "rep outsb",
                in("dx") port,
                inout("rsi") slice.as_ptr() => _,
                inout("rcx") slice.len() => _,
                options(readonly, nostack, preserves_flags)
            );
        }
    }
}

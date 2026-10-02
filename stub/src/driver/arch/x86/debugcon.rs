use core::{arch::asm, mem::{self, MaybeUninit}, pin::Pin, sync::atomic::{AtomicBool, AtomicU8, Ordering}};

use sync::ControlledModificationCell;

use crate::{declare_driver, driver::Driver, platform::{Console, ConsoleInternal, Metadata, register_console}};

static DEBUG_CON_DRIVER: DebugConDriver = DebugConDriver;
declare_driver!(DEBUG_CON_DRIVER);

static DEBUG_CON_INSTANCE_USED: AtomicU8 = AtomicU8::new(0);
static DEBUG_CON_INSTANCE: ControlledModificationCell<MaybeUninit<DebugCon>> = ControlledModificationCell::new(MaybeUninit::uninit());

struct DebugConDriver;

impl Driver for DebugConDriver {
    fn driver_name(&self) -> &'static str {
        "debugcon"
    }

    fn validate_preparedness(&self, arg: &'static str) -> bool {
        let Some(port) = arg.strip_prefix("port=0x") else {
            return false;
        };

        u16::from_str_radix(port, 16).is_ok()
    }

    fn connect(&self, arg: &'static str) -> Result<(), ()> {
        let Some(port) = arg.strip_prefix("port=0x") else {
            crate::warn!("debugcon requires argument of form 'port=<ADDRESS>'");
            return Err(());
        };

        let port = u16::from_str_radix(port, 16).map_err(|_| ())?;
        let debugcon = DebugCon {
            port,
            console_internal: ConsoleInternal::new(),
        };

        if DEBUG_CON_INSTANCE_USED.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            // Successfully claimed the `DEBUG_CON_INSTANCE`.
            
            // SAFETY:
            //
            // `DEBUG_CON_INSTANCE` is claimed and controlled by `DEBUG_CON_INSTANCE_USED`.
            let debugcon_instance = unsafe { DEBUG_CON_INSTANCE.get_mut() };
            let debugcon_instance = debugcon_instance.write(debugcon);
            let debugcon_instance = Pin::static_ref(debugcon_instance);
            register_console(debugcon_instance);

            DEBUG_CON_INSTANCE_USED.store(2, Ordering::Release);
        } else {
            todo!()
        }

        Ok(())
    }
}

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


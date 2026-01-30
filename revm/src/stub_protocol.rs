//! Code interacting with the REVM protocol.

use core::{ptr, sync::atomic::AtomicPtr};

use stub_api::{HeaderV0, Status};

#[unsafe(no_mangle)]
extern "C" fn revm_entry(header_ptr: *mut HeaderV0) -> Status {
    Status::SUCCESS
}

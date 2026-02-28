//! Code interacting with the REVM protocol.

use stub_api::{HeaderV0, Status};

/// Entry point to `revm` utilizing the REVM protocol.
#[unsafe(no_mangle)]
extern "C" fn revm_entry(header_ptr: *mut HeaderV0) -> Status {
    Status::SUCCESS
}

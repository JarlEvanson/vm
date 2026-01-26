//! Definitions and utilities useful for interacting with architectural state on either `x86_32` or
//! `x86_64`.

#![no_std]

pub mod cpuid;
pub mod io_port;
pub mod msr;
pub mod paging;

/// The privilege level associated with an item.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    /// Ring 0 is the most privileged ring, used by critical system-software components that
    /// require direct access to, and control over, all processor and system resources.
    Ring0 = 0,
    /// Ring 1 is typically not used anymore, and its privilege is controlled by the operating
    /// system.
    Ring1 = 1,
    /// Ring 2 is typically not used anymore, and its privilege is controlled by the operating
    /// system.
    Ring2 = 2,
    /// Ring 3 is the least privileged ring, used by application software.
    Ring3 = 3,
}

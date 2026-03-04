//! Cross address space switching related functionality.

use core::fmt;

use crate::{
    arch::{
        generic::switch::setup::{
            ComponentError, CpuDataError, allocate_protocol_table, clear, switch_data,
        },
        paging::ArchScheme,
        switch::{arch_policy, enter},
    },
    platform::{frame_size, main_processor_id},
};

use conversion::u64_to_usize_strict;
use memory::address::PhysicalAddress;

pub mod func;
pub mod setup;

/// Initializes and runs the cross address space switching program.
#[expect(clippy::missing_errors_doc)]
#[expect(clippy::missing_panics_doc)]
pub fn switch(
    scheme: ArchScheme,
    entry_point: u64,
    image_physical_address: PhysicalAddress,
    image_virtual_address: u64,
) -> Result<(), SwitchError> {
    let policy = arch_policy();
    setup::setup(scheme, policy)?;

    let main_processor_id_usize = u64_to_usize_strict(main_processor_id());
    let (protocol_table_frame_allocation, protocol_table) = {
        let mut switch_data = switch_data();
        let switch_data = switch_data
            .as_mut()
            .expect("switch data should be initialized");
        let (scheme, cpu_data_slice) = switch_data.both_mut();

        let main_cpu_data = cpu_data_slice[main_processor_id_usize].lock();
        let layout = &main_cpu_data.executable;

        let protocol_table = allocate_protocol_table(
            scheme,
            layout,
            image_physical_address,
            image_virtual_address,
        )
        .map_err(SwitchError::ProtocolTable)?;
        let protocol_table_address = protocol_table
            .range()
            .start()
            .start_address(frame_size())
            .value();
        (protocol_table, protocol_table_address)
    };

    crate::debug!(
        "Calling executable at {entry_point:#x} with protocol table at {protocol_table:#x}"
    );
    let result = enter(entry_point, protocol_table);

    crate::info!("Executable Result: {result:?}");

    drop(protocol_table_frame_allocation);
    clear();

    Ok(())
}

/// Various errors that can occur while preparing for switching to the executable and actually
/// switching to the executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchError {
    /// An error occurred while preparing data for the CPUs.
    CpuData(CpuDataError),
    /// An error occurred while preparing the protocol table.
    ProtocolTable(ComponentError),
}

impl From<CpuDataError> for SwitchError {
    fn from(error: CpuDataError) -> Self {
        Self::CpuData(error)
    }
}

impl fmt::Display for SwitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuData(error) => write!(f, "error preparing CPU data: {error}"),
            Self::ProtocolTable(error) => write!(f, "error preparing protocol table: {error}"),
        }
    }
}

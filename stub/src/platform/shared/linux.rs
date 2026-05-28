//! Code shared between protocols for purposes of interacting with the `linux` boot protocol.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::{mem, ptr};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use conversion::{u8_to_usize, u64_to_usize_strict, usize_to_u64};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use linux::x86::{BootParams, E820Entry, SetupData, SetupDataIndirect, SetupType};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::platform::{
    PhysicalAddress, PhysicalAddressRange, map_temporary, page_size, read_u32_at, read_u64_at,
};

/// [`Iterator`] over the [`E820Entry`]s that the [`BootParams`]s table provides.
///
/// This [`Iterator`] uses [`map_temporary()`].
#[derive(Clone, Debug)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct E820Iter {
    /// The location of the [`BootParams`] structure.
    boot_params: PhysicalAddress,
    /// The index of the next [`E820Entry`] to be outputted.
    index: usize,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl E820Iter {
    /// Creates an [`Iterator`] over the [`E820Entry`]s obtainable from the [`BootParams`]s table.
    pub fn new(boot_params: PhysicalAddress) -> Self {
        assert!(boot_params.is_aligned(usize_to_u64(mem::size_of::<BootParams>())));
        assert!(page_size() >= mem::size_of::<BootParams>());

        Self {
            boot_params,
            index: 0,
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Iterator for E820Iter {
    type Item = E820Entry;

    fn next(&mut self) -> Option<Self::Item> {
        let mapping = map_temporary(self.boot_params)?;
        // SAFETY:
        //
        // The `linux` boot protocol ensures that this operation is safe.
        let boot_params = unsafe { &*ptr::with_exposed_provenance::<BootParams>(mapping.value()) };
        let total_core_entries = u8_to_usize(boot_params.e820_entries);

        if self.index < total_core_entries && self.index < 128 {
            let entry = boot_params.e820_table[self.index];
            self.index += 1;
            return Some(entry);
        };

        let core_entries_read = total_core_entries.min(128);
        let mut extension_index = self.index - core_entries_read;

        let description_iter =
            SetupDataDescriptionIter::new(PhysicalAddress::new(boot_params.hdr.setup_data));
        for description in description_iter {
            if !description.setup_type.is_base_type(SetupType::E820_EXT) {
                continue;
            }

            let entry_size = mem::size_of::<E820Entry>();
            let total_entries_in_node = u64_to_usize_strict(description.range.count()) / entry_size;

            if extension_index < total_core_entries {
                // Calculate the [`PhysicalAddress`] of the specific [`E820Entry`].
                let offset = usize_to_u64(extension_index * entry_size);
                let entry_address = description.range.start().checked_add(offset)?;

                let address_address =
                    entry_address.checked_add(usize_to_u64(mem::offset_of!(E820Entry, addr)))?;
                let size_address =
                    entry_address.checked_add(usize_to_u64(mem::offset_of!(E820Entry, size)))?;
                let entry_type_address = entry_address
                    .checked_add(usize_to_u64(mem::offset_of!(E820Entry, entry_type)))?;

                let address = read_u64_at(address_address)?;
                let size = read_u64_at(size_address)?;
                let entry_type = read_u32_at(entry_type_address)?;

                let entry = E820Entry {
                    addr: address,
                    size,
                    entry_type,
                };

                return Some(entry);
            }

            // If not in this node, subtract its capacity and check subsequent nodes
            extension_index -= total_entries_in_node;
        }

        None
    }
}

/// [`Iterator`] over the [`SetupDataDescription`]s that correspond to each [`SetupData`] node.
///
/// This [`Iterator`] uses [`map_temporary()`].
#[derive(Clone, Debug)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct SetupDataDescriptionIter {
    /// Underlying [`Iterator`].
    iter: SetupDataNodeIter,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl SetupDataDescriptionIter {
    /// Creates an [`Iterator`] over the [`SetupDataDescription`]s that correspond to each
    /// [`SetupData`] node (with the first [`SetupData`] node located at `address`).
    pub fn new(address: PhysicalAddress) -> Self {
        Self {
            iter: SetupDataNodeIter::new(address),
        }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Iterator for SetupDataDescriptionIter {
    type Item = SetupDataDescription;

    fn next(&mut self) -> Option<Self::Item> {
        let (node_addr, node_data) = self.iter.next()?;
        let base_data_addr = node_addr.strict_add(usize_to_u64(mem::offset_of!(SetupData, _data)));

        if node_data.setup_type.is_indirect() {
            let setup_type_address = base_data_addr
                .strict_add(usize_to_u64(mem::offset_of!(SetupDataIndirect, setup_type)));
            let length_address =
                base_data_addr.strict_add(usize_to_u64(mem::offset_of!(SetupDataIndirect, length)));
            let address_address = base_data_addr
                .strict_add(usize_to_u64(mem::offset_of!(SetupDataIndirect, address)));

            let setup_type = SetupType(read_u32_at(setup_type_address)?);
            let length = read_u64_at(length_address)?;
            let address = PhysicalAddress::new(read_u64_at(address_address)?);

            let description = SetupDataDescription {
                setup_type,
                range: PhysicalAddressRange::new(address, length),
            };

            Some(description)
        } else {
            let description = SetupDataDescription {
                setup_type: node_data.setup_type,
                range: PhysicalAddressRange::new(base_data_addr, u64::from(node_data.len)),
            };

            Some(description)
        }
    }
}

/// The type and physical region in which the data associated with a [`SetupData`] node is located.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct SetupDataDescription {
    /// The type of the data.
    pub setup_type: SetupType,
    /// The [`PhysicalAddressRange`] in which the data resides.
    pub range: PhysicalAddressRange,
}

/// [`Iterator`] over the [`SetupData`] nodes.
///
/// This [`Iterator`] uses [`map_temporary()`].
#[derive(Clone, Debug)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct SetupDataNodeIter {
    /// The [`PhysicalAddress`] of the next [`SetupData`] node.
    next: PhysicalAddress,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl SetupDataNodeIter {
    /// Creates an [`Iterator`] over the [`SetupData`] nodes (with the first [`SetupData`] node
    /// located at `address`).
    pub fn new(address: PhysicalAddress) -> Self {
        Self { next: address }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Iterator for SetupDataNodeIter {
    type Item = (PhysicalAddress, SetupData);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == PhysicalAddress::zero() {
            return None;
        }

        let address = self.next;

        let next_address = address.checked_add(usize_to_u64(mem::offset_of!(SetupData, next)))?;
        let setup_type_address =
            address.checked_add(usize_to_u64(mem::offset_of!(SetupData, setup_type)))?;
        let length_address = address.checked_add(usize_to_u64(mem::offset_of!(SetupData, len)))?;

        let next = PhysicalAddress::new(read_u64_at(next_address)?);
        let setup_type = SetupType(read_u32_at(setup_type_address)?);
        let length = read_u32_at(length_address)?;

        let setup_data = SetupData {
            next: next.value(),
            setup_type,
            len: length,
            _data: [],
        };
        self.next = next;

        Some((address, setup_data))
    }
}

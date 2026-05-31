//! The `device_tree` crate provides an interface for parsing a device tree.
#![no_std]

use core::{ffi::CStr, fmt, mem, ptr::NonNull, slice::from_raw_parts};

use conversion::{u32_to_usize_checked, u32_to_usize_strict, usize_to_u32_checked};

use crate::raw::{
    FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_MAGIC, FDT_NOP, FDT_PROP, FDT_VERSION, FdtHeader,
    FdtProperty, FdtReserveEntry,
};

pub mod raw;

/// Base struct holding information about the Flattened Device Tree.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Fdt<'a> {
    /// The Flattened Device Tree header.
    header: &'a FdtHeader,

    /// The [`FdtReserveEntry`] entries associated with the Flattened Device Tree.
    reserve_entries: &'a [FdtReserveEntry],
    /// The string table associated with the Flattened Device Tree.
    strings: &'a [u8],
    /// The byte array used by the Flattened Device Tree structures array.
    structs: &'a [u8],
}

impl<'a> Fdt<'a> {
    ///
    ///
    /// # Safety
    ///
    /// `dtb_ptr` must point a region of memory that is initialized and readable for at least
    /// this size of [`FdtHeader`] and if [`FdtHeader`] has a valid magic, it must be readable for
    /// [`FdtHeader::totalsize`] bytes.
    pub unsafe fn from_ptr(dtb_ptr: *mut FdtHeader) -> Option<Self> {
        let dtb_ptr = NonNull::new(dtb_ptr)?;

        // SAFETY:
        //
        // The provided `dtb_ptr` complies with the Devicetree Specification.
        let fdt_header = unsafe { dtb_ptr.as_ref() };
        if u32::from_be(fdt_header.magic) != FDT_MAGIC {
            return None;
        }

        if u32::from_be(fdt_header.last_comp_version) > FDT_VERSION {
            return None;
        }

        let total_size = u32::from_be(fdt_header.totalsize);

        let struct_offset = u32::from_be(fdt_header.off_dt_struct);
        let struct_size = u32::from_be(fdt_header.size_dt_struct);
        let struct_end = struct_offset.checked_add(struct_size)?;

        if struct_end > total_size || !struct_offset.is_multiple_of(4) {
            return None;
        }

        // SAFETY:
        //
        // The `dtb_ptr` points to a valid Flattened Device Tree` and thus `structs` is within the
        // valid area.
        let structs = unsafe {
            from_raw_parts(
                dtb_ptr
                    .as_ptr()
                    .wrapping_byte_add(u32_to_usize_checked(struct_offset)?)
                    .cast::<u8>(),
                u32_to_usize_checked(struct_size)?,
            )
        };

        let strings_offset = u32::from_be(fdt_header.off_dt_strings);
        let strings_size = u32::from_be(fdt_header.size_dt_strings);
        let strings_end = strings_offset.checked_add(strings_size)?;

        if strings_end > total_size {
            return None;
        };

        // SAFETY:
        //
        // The `dtb_ptr` points to a valid Flattened Device Tree` and thus `strings` is within the
        // valid area.
        let strings = unsafe {
            from_raw_parts(
                dtb_ptr
                    .as_ptr()
                    .wrapping_byte_add(u32_to_usize_checked(strings_offset)?)
                    .cast::<u8>(),
                u32_to_usize_checked(strings_size)?,
            )
        };

        let mem_reserved_offset = u32::from_be(fdt_header.off_mem_rsvmap);
        if !mem_reserved_offset.is_multiple_of(8) {
            return None;
        }

        let mut current_offset = mem_reserved_offset;
        let reserve_entry_mem_size = usize_to_u32_checked(mem::size_of::<FdtReserveEntry>())?;
        loop {
            let current_max_offset = current_offset.checked_add(reserve_entry_mem_size)?;
            if current_max_offset > total_size {
                return None;
            }

            let reserve_entry_ptr = dtb_ptr
                .as_ptr()
                .wrapping_byte_add(u32_to_usize_checked(current_offset)?);
            // SAFETY:
            //
            // The specification states that the reserve entry structure is 8-byte aligned and
            // initialized.
            let reserve_entry = unsafe { reserve_entry_ptr.cast::<FdtReserveEntry>().read() };
            let reserve_entry_address = u64::from_be(reserve_entry.address);
            let reserve_entry_size = u64::from_be(reserve_entry.size);
            if reserve_entry_address == 0 && reserve_entry_size == 0 {
                break;
            }

            current_offset += reserve_entry_mem_size;
        }

        let reserve_entry_count = (current_offset - mem_reserved_offset) / reserve_entry_mem_size;

        // SAFETY:
        //
        // The `dtb_ptr` points to a valid Flattened Device Tree` and thus `reserve_entries` is
        // within the valid area.
        let reserve_entries = unsafe {
            from_raw_parts(
                dtb_ptr
                    .as_ptr()
                    .wrapping_byte_add(u32_to_usize_checked(current_offset)?)
                    .cast::<FdtReserveEntry>(),
                u32_to_usize_checked(reserve_entry_count)?,
            )
        };

        let fdt = Self {
            header: fdt_header,
            reserve_entries,
            strings,
            structs,
        };

        fdt.validate()?;
        Some(fdt)
    }

    /// Returns a byte slice over the entire region occupied by the Flattened Device Tree.
    pub const fn fdt_region(&self) -> &[u8] {
        let ptr = core::ptr::from_ref(self.header).cast::<u8>();
        let total_size = u32_to_usize_strict(u32::from_be(self.header.totalsize));

        // SAFETY:
        //
        // The `dtb_ptr` pointed to a valid [`FdtHeader`] and thus the region of memory extends for
        // `totalsize` bytes.
        unsafe { from_raw_parts(ptr, total_size) }
    }

    /// Returns the array of [`FdtReserveEntry`]s.
    ///
    /// This does not include the final [`FdtReserveEntry`] entry (i.e., the double zero entry).
    pub fn reserve_entries(&self) -> impl Iterator<Item = FdtReserveEntry> + Clone {
        self.reserve_entries.iter().map(|entry| FdtReserveEntry {
            address: u64::from_be(entry.address),
            size: u64::from_be(entry.size),
        })
    }

    /// Returns the array of bytes used to contain the [`CStr`]s used by the device tree.
    pub const fn strings(&self) -> &'a [u8] {
        self.strings
    }

    /// Returns the byte array used by the Flattened Device Tree structures array.
    pub const fn structs(&self) -> &'a [u8] {
        self.structs
    }

    /// Returns the root node of the Flattened Device Tree.
    pub fn root(&self) -> Node<'a> {
        let mut offset = 0;
        let token = next_token(self.structs(), self.strings(), &mut offset);
        let Token::BeginNode { name } = token else {
            unreachable!()
        };

        Node {
            name,
            strings: self.strings(),
            structures: &self.structs()[offset..],
        }
    }

    /// Validates the Flattened Device Tree.
    fn validate(&self) -> Option<()> {
        fn read_u32_at(buffer: &[u8], base_offset: usize, sub_offset: usize) -> Option<u32> {
            let slice = slice_with_base(buffer, base_offset, sub_offset, mem::size_of::<u32>())?;

            let mut bytes = [0; mem::size_of::<u32>()];
            bytes.copy_from_slice(slice);
            Some(u32::from_be_bytes(bytes))
        }

        let buffer = self.structs();
        let mut offset = 0;
        let mut depth = 0usize;
        let mut first_node = true;
        loop {
            let token_type = read_u32_at(buffer, offset, 0)?;
            offset += 4;

            match token_type {
                FDT_BEGIN_NODE => {
                    depth = depth.checked_add(1)?;

                    let mut sub_index = 0;
                    loop {
                        let byte = slice_with_base(buffer, offset, sub_index, 1)?[0];
                        sub_index += 1;

                        if byte == b'\0' {
                            break;
                        }
                    }

                    // `sub_index == 1` indicates an empty name.
                    if first_node && sub_index != 1 {
                        return None;
                    }
                    first_node = false;

                    let next_offset = sub_index.checked_next_multiple_of(mem::size_of::<u32>())?;
                    offset = offset.checked_add(next_offset)?;
                }
                FDT_END_NODE => {
                    depth = depth.checked_sub(1)?;
                }
                FDT_PROP => {
                    let len = read_u32_at(buffer, offset, mem::offset_of!(FdtProperty, len))?;
                    let len = u32_to_usize_checked(len)?;
                    let nameoff =
                        read_u32_at(buffer, offset, mem::offset_of!(FdtProperty, nameoff))?;
                    let nameoff = u32_to_usize_checked(nameoff)?;
                    offset += mem::size_of::<FdtProperty>();

                    let mut sub_index = 0;
                    loop {
                        let byte = slice_with_base(self.strings(), nameoff, sub_index, 1)?[0];
                        sub_index += 1;

                        if byte == b'\0' {
                            break;
                        }
                    }

                    slice_with_base(buffer, offset, 0, len)?;
                    offset = offset.checked_add(len)?;
                    offset = offset.checked_next_multiple_of(mem::size_of::<u32>())?;
                }
                FDT_NOP => {}
                FDT_END => {
                    if depth != 0 {
                        return None;
                    }

                    return Some(());
                }
                _ => return None,
            }
        }
    }
}

impl fmt::Debug for Fdt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Fdt");

        debug_struct.field("root", &self.root());

        debug_struct.finish()
    }
}

/// Returns the sub-slice of `size` bytes located `base + offset` bytes into the provided `buffer`.
pub fn slice_with_base(buffer: &[u8], base: usize, offset: usize, size: usize) -> Option<&[u8]> {
    let actual_offset = base.checked_add(offset)?;
    let max_offset = actual_offset.checked_add(size)?;
    if max_offset > buffer.len() {
        return None;
    }

    Some(&buffer[actual_offset..][..size])
}

/// Returns the next [`Token`].
fn next_token<'a>(structs: &'a [u8], strings: &'a [u8], offset: &mut usize) -> Token<'a> {
    fn read_u32_at(buffer: &[u8], base_offset: usize, sub_offset: usize) -> u32 {
        let slice = slice_with_base(buffer, base_offset, sub_offset, mem::size_of::<u32>())
            .expect("invariant failed");

        let mut bytes = [0; mem::size_of::<u32>()];
        bytes.copy_from_slice(slice);
        u32::from_be_bytes(bytes)
    }

    let token_type = read_u32_at(structs, *offset, 0);
    *offset += 4;

    match token_type {
        FDT_BEGIN_NODE => {
            let name = CStr::from_bytes_until_nul(&structs[*offset..]).expect("invariant failed");
            let token = Token::BeginNode { name };

            *offset =
                (*offset + name.to_bytes_with_nul().len()).next_multiple_of(mem::size_of::<u32>());
            token
        }
        FDT_END_NODE => Token::EndNode,
        FDT_PROP => {
            let len = read_u32_at(structs, *offset, mem::offset_of!(FdtProperty, len));
            let len = u32_to_usize_strict(len);
            let nameoff = read_u32_at(structs, *offset, mem::offset_of!(FdtProperty, nameoff));
            let nameoff = u32_to_usize_strict(nameoff);
            *offset += mem::size_of::<FdtProperty>();

            let token = Token::Property {
                name: CStr::from_bytes_until_nul(&strings[nameoff..]).expect("invariant failed"),
                data: &structs[*offset..][..len],
            };

            *offset = (*offset + len).next_multiple_of(mem::size_of::<u32>());
            token
        }
        FDT_NOP => next_token(structs, strings, offset),
        FDT_END => Token::End,
        _ => unreachable!(),
    }
}

/// A token and its associated data after parsing from the Flattened Device Tree.
enum Token<'a> {
    /// A new [`Node`] begins.
    BeginNode {
        /// The name of the new [`Node`].
        name: &'a CStr,
    },
    /// The active [`Node`] ends here.
    EndNode,
    /// A [`Property`] was located.
    Property {
        /// The name of the [`Property`].
        name: &'a CStr,
        /// The data associated with the [`Property`].
        data: &'a [u8],
    },
    /// The FDT structs section has ended.
    End,
}

/// A Flattened Device Tree node.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Node<'a> {
    /// The name of the node.
    name: &'a CStr,
    /// The string table associated with the Flattened Device Tree.
    strings: &'a [u8],
    /// The byte array that describes the node.
    structures: &'a [u8],
}

impl<'a> Node<'a> {
    /// Returns the name associated with this [`Node`].
    pub const fn name(&self) -> &'a CStr {
        self.name
    }

    /// Returns an iterator over the subnodes in the node.
    pub fn nodes(&self) -> impl Iterator<Item = Node<'a>> + Clone {
        NodeIter {
            node: *self,
            offset: 0,
            finished: false,
        }
    }

    /// Returns an iterator over the properties in the node.
    pub fn properties(&self) -> impl Iterator<Item = Property<'a>> + Clone {
        PropertyIter {
            node: *self,
            offset: 0,
        }
    }

    /// Returns a [`Node`] with the given `name`, if it exists.
    pub fn find_node(&self, name: &CStr) -> Option<Node<'a>> {
        self.nodes().find(|node| node.name == name)
    }

    /// Returns the [`Property`] with the given `name`, if it exists.
    pub fn find_property(&self, name: &CStr) -> Option<Property<'a>> {
        self.properties().find(|property| property.name == name)
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Node");

        debug_struct.field("name", &self.name);
        debug_struct.field(
            "properties",
            &PropertyIter {
                node: *self,
                offset: 0,
            },
        );
        debug_struct.field(
            "nodes",
            &NodeIter {
                node: *self,
                offset: 0,
                finished: false,
            },
        );

        debug_struct.finish()
    }
}

/// An [`Iterator`] over the subnodes in a node.
#[derive(Clone)]
pub struct NodeIter<'a> {
    /// The [`Node`] from which child [`Node`]s are being extracted.
    node: Node<'a>,
    /// The byte offset into the [`Node`].
    offset: usize,
    /// If the [`NodeIter`] has finished iteration.
    finished: bool,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let mut new_offset = self.offset;
        let mut depth = 0;
        let mut node_data = None;
        loop {
            let token = next_token(self.node.structures, self.node.strings, &mut new_offset);
            match token {
                Token::BeginNode { name } => {
                    if depth == 0 {
                        node_data = Some(Node {
                            name,
                            strings: self.node.strings,
                            structures: &self.node.structures[new_offset..],
                        });
                        depth += 1;
                    } else {
                        depth += 1;
                    }
                }
                Token::EndNode => {
                    if depth == 0 {
                        self.finished = true;
                        break;
                    } else if depth == 1 {
                        break;
                    } else {
                        depth -= 1;
                    }
                }
                Token::Property { name: _, data: _ } => {}
                Token::End => {}
            }
        }

        self.offset = new_offset;
        node_data
    }
}

impl fmt::Debug for NodeIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_list = f.debug_list();

        debug_list.entries(self.clone());

        debug_list.finish()
    }
}

/// An [`Iterator`] over the properties in a node.
#[derive(Clone)]
pub struct PropertyIter<'a> {
    /// The [`Node`] from which [`Property`]s are being extracted.
    node: Node<'a>,
    /// The byte offset into the [`Node`].
    offset: usize,
}

impl<'a> Iterator for PropertyIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut new_offset = self.offset;
        let token = next_token(self.node.structures, self.node.strings, &mut new_offset);
        let Token::Property { name, data } = token else {
            return None;
        };

        self.offset = new_offset;
        Some(Property { name, data })
    }
}

impl fmt::Debug for PropertyIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_list = f.debug_list();

        debug_list.entries(self.clone());

        debug_list.finish()
    }
}

/// A Flattened Debug Tree node property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property<'a> {
    /// The name of the proerty.
    name: &'a CStr,
    /// The data associated with the property.
    data: &'a [u8],
}

impl<'a> Property<'a> {
    /// Returns the name associated with this [`Property`].
    pub const fn name(&self) -> &'a CStr {
        self.name
    }

    /// Returns the data associated with this [`Property`].
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Returns the [`u32`] at `offset` bytes into this [`Property`]'s data.
    pub fn read_u32_at(&self, offset: usize) -> Option<u32> {
        let slice = slice_with_base(self.data, offset, 0, mem::size_of::<u32>())?;

        let mut bytes = [0; mem::size_of::<u32>()];
        bytes.copy_from_slice(slice);
        Some(u32::from_be_bytes(bytes))
    }

    /// Returns the [`u64`] at `offset` bytes into this [`Property`]'s data.
    pub fn read_u64_at(&self, offset: usize) -> Option<u64> {
        let lower_slice = slice_with_base(self.data, offset, 0, mem::size_of::<u32>())?;
        let upper_slice = slice_with_base(
            self.data,
            offset,
            mem::size_of::<u32>(),
            mem::size_of::<u32>(),
        )?;

        let mut bytes = [0; mem::size_of::<u32>()];

        bytes.copy_from_slice(lower_slice);
        let lower = u32::from_ne_bytes(bytes);
        bytes.copy_from_slice(upper_slice);
        let upper = u32::from_ne_bytes(bytes);

        Some(u64::from_be(u64::from(lower) | (u64::from(upper) << 32)))
    }

    /// Returns the [`CStr`] at `offset` bytes into this [`Property`]'s data.
    pub fn read_cstr(&self, offset: usize) -> Option<&'a CStr> {
        let slice = slice_with_base(self.data, offset, 0, self.data.len().saturating_sub(offset))?;
        CStr::from_bytes_until_nul(slice).ok()
    }
}

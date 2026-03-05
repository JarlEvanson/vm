//! Raw definitions related to paging structures for `aarch64` utilizing the `VMSAv8-64`
//! translation system.

#![expect(
    missing_docs,
    reason = "bit shift/masking is documented in `TranslationDescriptor`"
)]

use crate::{common::Granule, paging::AddressSize};

pub const PRESENT_SHIFT: u32 = 0;
pub const PRESENT_BIT: u64 = 1 << PRESENT_SHIFT;

// Table-related constants.
pub const TABLE_SHIFT: u32 = 1;
pub const TABLE_BIT: u64 = 1 << TABLE_SHIFT;

pub const TABLE_48_MASK_SIZE: u32 = 36;
pub const TABLE_48_MASK_SHIFT: u32 = 12;
pub const TABLE_48_MASK: u64 = ((1 << TABLE_48_MASK_SIZE) - 1) << TABLE_48_MASK_SHIFT;

pub const TABLE_4K_16K_52_MASK_LOWER_SIZE: u32 = 38;
pub const TABLE_4K_16K_52_MASK_LOWER_SHIFT: u32 = 12;
pub const TABLE_4K_16K_52_MASK_LOWER: u64 =
    ((1 << TABLE_4K_16K_52_MASK_LOWER_SIZE) - 1) << TABLE_4K_16K_52_MASK_LOWER_SHIFT;

pub const TABLE_4K_16K_52_MASK_UPPER_ADDR_SHIFT: u32 = 50;
pub const TABLE_4K_16K_52_MASK_UPPER_SIZE: u32 = 2;
pub const TABLE_4K_16K_52_MASK_UPPER_SHIFT: u32 = 8;
pub const TABLE_4K_16K_52_MASK_UPPER: u64 =
    ((1 << TABLE_4K_16K_52_MASK_UPPER_SIZE) - 1) << TABLE_4K_16K_52_MASK_UPPER_SHIFT;

pub const TABLE_64K_52_MASK_LOWER_SIZE: u32 = 32;
pub const TABLE_64K_52_MASK_LOWER_SHIFT: u32 = 16;
pub const TABLE_64K_52_MASK_LOWER: u64 =
    ((1 << TABLE_64K_52_MASK_LOWER_SIZE) - 1) << TABLE_64K_52_MASK_LOWER_SHIFT;

pub const TABLE_64K_52_MASK_UPPER_ADDR_SHIFT: u32 = 48;
pub const TABLE_64K_52_MASK_UPPER_SIZE: u32 = 4;
pub const TABLE_64K_52_MASK_UPPER_SHIFT: u32 = 12;
pub const TABLE_64K_52_MASK_UPPER: u64 =
    ((1 << TABLE_64K_52_MASK_UPPER_SIZE) - 1) << TABLE_64K_52_MASK_UPPER_SHIFT;

// Block-related constants.
pub const BLOCK_48_MASK_SIZE: u32 = 27;
pub const BLOCK_48_MASK_SHIFT: u32 = 21;
pub const BLOCK_48_MASK: u64 = ((1 << BLOCK_48_MASK_SIZE) - 1) << BLOCK_48_MASK_SHIFT;

pub const BLOCK_4K_16K_52_MASK_LOWER_SIZE: u32 = 29;
pub const BLOCK_4K_16K_52_MASK_LOWER_SHIFT: u32 = 21;
pub const BLOCK_4K_16K_52_MASK_LOWER: u64 =
    ((1 << BLOCK_4K_16K_52_MASK_LOWER_SIZE) - 1) << BLOCK_4K_16K_52_MASK_LOWER_SHIFT;

pub const BLOCK_4K_16K_52_MASK_UPPER_ADDR_SHIFT: u32 = 50;
pub const BLOCK_4K_16K_52_MASK_UPPER_SIZE: u32 = 2;
pub const BLOCK_4K_16K_52_MASK_UPPER_SHIFT: u32 = 8;
pub const BLOCK_4K_16K_52_MASK_UPPER: u64 =
    ((1 << BLOCK_4K_16K_52_MASK_UPPER_SIZE) - 1) << BLOCK_4K_16K_52_MASK_UPPER_SHIFT;

pub const BLOCK_64K_52_MASK_LOWER_SIZE: u32 = 19;
pub const BLOCK_64K_52_MASK_LOWER_SHIFT: u32 = 29;
pub const BLOCK_64K_52_MASK_LOWER: u64 =
    ((1 << BLOCK_64K_52_MASK_LOWER_SIZE) - 1) << BLOCK_64K_52_MASK_LOWER_SHIFT;

pub const BLOCK_64K_52_MASK_UPPER_ADDR_SHIFT: u32 = 48;
pub const BLOCK_64K_52_MASK_UPPER_SIZE: u32 = 4;
pub const BLOCK_64K_52_MASK_UPPER_SHIFT: u32 = 12;
pub const BLOCK_64K_52_MASK_UPPER: u64 =
    ((1 << BLOCK_64K_52_MASK_UPPER_SIZE) - 1) << BLOCK_64K_52_MASK_UPPER_SHIFT;

// Page-related constants.
pub const PAGE_SHIFT: u32 = 1;
pub const PAGE_BIT: u64 = 1 << PAGE_SHIFT;

pub const PAGE_4K_48_MASK_SIZE: u32 = 36;
pub const PAGE_4K_48_MASK_SHIFT: u32 = 12;
pub const PAGE_4K_48_MASK: u64 = ((1 << PAGE_4K_48_MASK_SIZE) - 1) << PAGE_4K_48_MASK_SHIFT;

pub const PAGE_16K_48_MASK_SIZE: u32 = 34;
pub const PAGE_16K_48_MASK_SHIFT: u32 = 14;
pub const PAGE_16K_48_MASK: u64 = ((1 << PAGE_16K_48_MASK_SIZE) - 1) << PAGE_16K_48_MASK_SHIFT;

pub const PAGE_64K_48_MASK_SIZE: u32 = 32;
pub const PAGE_64K_48_MASK_SHIFT: u32 = 16;
pub const PAGE_64K_48_MASK: u64 = ((1 << PAGE_64K_48_MASK_SIZE) - 1) << PAGE_64K_48_MASK_SHIFT;

pub const PAGE_4K_52_MASK_LOWER_SIZE: u32 = 38;
pub const PAGE_4K_52_MASK_LOWER_SHIFT: u32 = 12;
pub const PAGE_4K_52_MASK_LOWER: u64 =
    ((1 << PAGE_4K_52_MASK_LOWER_SIZE) - 1) << PAGE_4K_52_MASK_LOWER_SHIFT;

pub const PAGE_4K_52_MASK_UPPER_ADDR_SHIFT: u32 = 50;
pub const PAGE_4K_52_MASK_UPPER_SIZE: u32 = 2;
pub const PAGE_4K_52_MASK_UPPER_SHIFT: u32 = 8;
pub const PAGE_4K_52_MASK_UPPER: u64 =
    ((1 << PAGE_4K_52_MASK_UPPER_SIZE) - 1) << PAGE_4K_52_MASK_UPPER_SHIFT;

pub const PAGE_16K_52_MASK_LOWER_SIZE: u32 = 36;
pub const PAGE_16K_52_MASK_LOWER_SHIFT: u32 = 14;
pub const PAGE_16K_52_MASK_LOWER: u64 =
    ((1 << PAGE_16K_52_MASK_LOWER_SIZE) - 1) << PAGE_16K_52_MASK_LOWER_SHIFT;

pub const PAGE_16K_52_MASK_UPPER_ADDR_SHIFT: u32 = 50;
pub const PAGE_16K_52_MASK_UPPER_SIZE: u32 = 2;
pub const PAGE_16K_52_MASK_UPPER_SHIFT: u32 = 8;
pub const PAGE_16K_52_MASK_UPPER: u64 =
    ((1 << PAGE_16K_52_MASK_UPPER_SIZE) - 1) << PAGE_16K_52_MASK_UPPER_SHIFT;

pub const PAGE_64K_52_MASK_LOWER_SIZE: u32 = 32;
pub const PAGE_64K_52_MASK_LOWER_SHIFT: u32 = 16;
pub const PAGE_64K_52_MASK_LOWER: u64 =
    ((1 << PAGE_64K_52_MASK_LOWER_SIZE) - 1) << PAGE_64K_52_MASK_LOWER_SHIFT;

pub const PAGE_64K_52_MASK_UPPER_ADDR_SHIFT: u32 = 48;
pub const PAGE_64K_52_MASK_UPPER_SIZE: u32 = 4;
pub const PAGE_64K_52_MASK_UPPER_SHIFT: u32 = 12;
pub const PAGE_64K_52_MASK_UPPER: u64 =
    ((1 << PAGE_64K_52_MASK_UPPER_SIZE) - 1) << PAGE_64K_52_MASK_UPPER_SHIFT;

/// A generic translation descriptor, which can either be a table descriptor, block descriptor, or
/// page descriptor.
///
/// This representation does not do any validity checking.
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationDescriptor(u64);

impl TranslationDescriptor {
    /// Creates a new [`TranslationDescriptor`] that is not present.
    pub const fn non_present() -> Self {
        Self(0)
    }

    /// Constructs a new [`TranslationDescriptor`] from the bit representation.
    pub const fn from_bits(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the bit representation of the [`TranslationDescriptor`].
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// Returns `true` if the [`TranslationDescriptor`] describes a present descriptor.
    pub const fn present(self) -> bool {
        self.0 & PRESENT_BIT == PRESENT_BIT
    }

    /// Sets whether the [`TranslationDescriptor`] is present, thereby affecting whether
    /// translation succeeds.
    pub const fn set_present(self, present: bool) -> Self {
        Self((self.0 & !PRESENT_BIT) | (bool_as_u64(present) << PRESENT_SHIFT))
    }

    // Page descriptor utility functions.

    /// Returns `true` if the [`TranslationDescriptor`] is a page descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    pub const fn page(self) -> bool {
        self.0 & PAGE_BIT == PAGE_BIT
    }

    /// Sets whether the [`TranslationDescriptor`] should be treated as a page descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    pub const fn set_page(self, page: bool) -> Self {
        Self((self.0 & !PAGE_BIT) | (bool_as_u64(page) << PAGE_SHIFT))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page4KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn page_4k_48bit_address(self) -> u64 {
        self.0 & PAGE_4K_48_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page4KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn set_page_4k_48bit_address(self, address: u64) -> Self {
        Self((self.0 & !PAGE_4K_48_MASK) | (address & PAGE_4K_48_MASK))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn page_4k_52bit_address(self) -> u64 {
        (self.0 & PAGE_4K_52_MASK_LOWER)
            | ((self.0 & PAGE_4K_52_MASK_UPPER) << PAGE_4K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn set_page_4k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(PAGE_4K_52_MASK_LOWER | PAGE_4K_52_MASK_UPPER))
                | ((address & PAGE_4K_52_MASK_LOWER)
                    | ((address >> PAGE_4K_52_MASK_UPPER_ADDR_SHIFT) & PAGE_4K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page16KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn page_16k_48bit_address(self) -> u64 {
        self.0 & PAGE_16K_48_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page16KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn set_page_16k_48bit_address(self, address: u64) -> Self {
        Self((self.0 & !PAGE_16K_48_MASK) | (address & PAGE_16K_48_MASK))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page16KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn page_16k_52bit_address(self) -> u64 {
        (self.0 & PAGE_16K_52_MASK_LOWER)
            | ((self.0 & PAGE_16K_52_MASK_UPPER) << PAGE_16K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page16KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn set_page_16k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(PAGE_16K_52_MASK_LOWER | PAGE_16K_52_MASK_UPPER))
                | ((address & PAGE_16K_52_MASK_LOWER)
                    | ((address >> PAGE_16K_52_MASK_UPPER_ADDR_SHIFT) & PAGE_16K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page64KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn page_64k_48bit_address(self) -> u64 {
        self.0 & PAGE_64K_48_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with [`Granule::Page64KiB`] and
    /// output [`AddressSize::Bits48`].
    pub const fn set_page_64k_48bit_address(self, address: u64) -> Self {
        Self((self.0 & !PAGE_64K_48_MASK) | (address & PAGE_64K_48_MASK))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn page_64k_52bit_address(self) -> u64 {
        (self.0 & PAGE_64K_52_MASK_LOWER)
            | ((self.0 & PAGE_64K_52_MASK_UPPER) << PAGE_64K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] and
    /// output [`AddressSize::Bits52`].
    pub const fn set_page_64k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(PAGE_64K_52_MASK_LOWER | PAGE_64K_52_MASK_UPPER))
                | ((address & PAGE_64K_52_MASK_LOWER)
                    | ((address >> PAGE_64K_52_MASK_UPPER_ADDR_SHIFT) & PAGE_64K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    pub const fn page_address(self, granule: Granule, output_size: AddressSize) -> u64 {
        match output_size {
            AddressSize::Bits48 => match granule {
                Granule::Page4KiB => self.page_4k_48bit_address(),
                Granule::Page16KiB => self.page_16k_48bit_address(),
                Granule::Page64KiB => self.page_64k_48bit_address(),
            },
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB => self.page_4k_52bit_address(),
                Granule::Page16KiB => self.page_16k_52bit_address(),
                Granule::Page64KiB => self.page_64k_52bit_address(),
            },
        }
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a page descriptor.
    pub const fn set_page_address(
        self,
        granule: Granule,
        output_size: AddressSize,
        address: u64,
    ) -> Self {
        match output_size {
            AddressSize::Bits48 => match granule {
                Granule::Page4KiB => self.set_page_4k_48bit_address(address),
                Granule::Page16KiB => self.set_page_16k_48bit_address(address),
                Granule::Page64KiB => self.set_page_64k_48bit_address(address),
            },
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB => self.set_page_4k_52bit_address(address),
                Granule::Page16KiB => self.set_page_16k_52bit_address(address),
                Granule::Page64KiB => self.set_page_64k_52bit_address(address),
            },
        }
    }

    // Table descriptor utility functions.

    /// Returns `true` if the [`TranslationDescriptor`] is a table descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    pub const fn table(self) -> bool {
        self.0 & TABLE_BIT == TABLE_BIT
    }

    /// Sets whether the [`TranslationDescriptor`] should be treated as a table descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    pub const fn set_table(self, table: bool) -> Self {
        Self((self.0 & !TABLE_BIT) | (bool_as_u64(table) << TABLE_SHIFT))
    }

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with output
    /// [`AddressSize::Bits48`].
    pub const fn table_48bit_address(self) -> u64 {
        self.0 & TABLE_48_MASK
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with output
    /// [`AddressSize::Bits48`].
    pub const fn set_table_48bit_address(self, address: u64) -> Self {
        Self((self.0 & !TABLE_48_MASK) | (address & TABLE_48_MASK))
    }

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] or
    /// [`Granule::Page16KiB`] with output [`AddressSize::Bits52`].
    pub const fn table_4k_16k_52bit_address(self) -> u64 {
        (self.0 & TABLE_4K_16K_52_MASK_LOWER)
            | ((self.0 & TABLE_4K_16K_52_MASK_UPPER) << TABLE_4K_16K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] or
    /// [`Granule::Page16KiB`] with output [`AddressSize::Bits52`].
    pub const fn set_table_4k_16k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(TABLE_4K_16K_52_MASK_LOWER | TABLE_4K_16K_52_MASK_UPPER))
                | ((address & TABLE_4K_16K_52_MASK_LOWER)
                    | ((address >> TABLE_4K_16K_52_MASK_UPPER_ADDR_SHIFT)
                        & TABLE_4K_16K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] with
    /// output [`AddressSize::Bits52`].
    pub const fn table_64k_52bit_address(self) -> u64 {
        (self.0 & TABLE_64K_52_MASK_LOWER)
            | ((self.0 & TABLE_64K_52_MASK_UPPER) << TABLE_64K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] with
    /// output [`AddressSize::Bits52`].
    pub const fn set_table_64k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(TABLE_64K_52_MASK_LOWER | TABLE_64K_52_MASK_UPPER))
                | ((address & TABLE_64K_52_MASK_LOWER)
                    | ((address >> TABLE_64K_52_MASK_UPPER_ADDR_SHIFT) & TABLE_64K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    pub const fn table_address(self, granule: Granule, output_size: AddressSize) -> u64 {
        match output_size {
            AddressSize::Bits48 => self.table_48bit_address(),
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB | Granule::Page16KiB => self.table_4k_16k_52bit_address(),
                Granule::Page64KiB => self.table_64k_52bit_address(),
            },
        }
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a table descriptor.
    pub const fn set_table_address(
        self,
        granule: Granule,
        output_size: AddressSize,
        address: u64,
    ) -> Self {
        match output_size {
            AddressSize::Bits48 => self.set_table_48bit_address(address),
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB | Granule::Page16KiB => {
                    self.set_table_4k_16k_52bit_address(address)
                }
                Granule::Page64KiB => self.set_table_64k_52bit_address(address),
            },
        }
    }

    // Block descriptor utility functions.

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with output
    /// [`AddressSize::Bits48`].
    pub const fn block_48bit_address(self) -> u64 {
        self.0 & BLOCK_48_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with output
    /// [`AddressSize::Bits48`].
    pub const fn set_block_48bit_address(self, address: u64) -> Self {
        Self((self.0 & !BLOCK_48_MASK) | (address & BLOCK_48_MASK))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] or
    /// [`Granule::Page16KiB`] with output [`AddressSize::Bits52`].
    pub const fn block_4k_16k_52bit_address(self) -> u64 {
        (self.0 & BLOCK_4K_16K_52_MASK_LOWER)
            | ((self.0 & BLOCK_4K_16K_52_MASK_UPPER) << BLOCK_4K_16K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page4KiB`] or
    /// [`Granule::Page16KiB`] with output [`AddressSize::Bits52`].
    pub const fn set_block_4k_16k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(BLOCK_4K_16K_52_MASK_LOWER | BLOCK_4K_16K_52_MASK_UPPER))
                | ((address & BLOCK_4K_16K_52_MASK_LOWER)
                    | ((address >> BLOCK_4K_16K_52_MASK_UPPER_ADDR_SHIFT)
                        & BLOCK_4K_16K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] with
    /// output [`AddressSize::Bits52`].
    pub const fn block_64k_52bit_address(self) -> u64 {
        (self.0 & BLOCK_64K_52_MASK_LOWER)
            | ((self.0 & BLOCK_64K_52_MASK_UPPER) << BLOCK_64K_52_MASK_UPPER_ADDR_SHIFT)
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    /// Moreover, this must only be used on translation schemes with a [`Granule::Page64KiB`] with
    /// output [`AddressSize::Bits52`].
    pub const fn set_block_64k_52bit_address(self, address: u64) -> Self {
        Self(
            (self.0 & !(BLOCK_64K_52_MASK_LOWER | BLOCK_64K_52_MASK_UPPER))
                | ((address & BLOCK_64K_52_MASK_LOWER)
                    | ((address >> BLOCK_64K_52_MASK_UPPER_ADDR_SHIFT) & BLOCK_64K_52_MASK_UPPER)),
        )
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    pub const fn block_address(self, granule: Granule, output_size: AddressSize) -> u64 {
        match output_size {
            AddressSize::Bits48 => self.block_48bit_address(),
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB | Granule::Page16KiB => self.block_4k_16k_52bit_address(),
                Granule::Page64KiB => self.block_64k_52bit_address(),
            },
        }
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could be a block descriptor.
    pub const fn set_block_address(
        self,
        granule: Granule,
        output_size: AddressSize,
        address: u64,
    ) -> Self {
        match output_size {
            AddressSize::Bits48 => self.set_block_48bit_address(address),
            AddressSize::Bits52 => match granule {
                Granule::Page4KiB | Granule::Page16KiB => {
                    self.set_block_4k_16k_52bit_address(address)
                }
                Granule::Page64KiB => self.set_block_64k_52bit_address(address),
            },
        }
    }

    /// Returns `true` if the `accessed` bit is set.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that are page or block descriptors.
    pub const fn page_block_accessed(self) -> bool {
        ((self.0 >> 10) & 0b1) == 0b1
    }

    /// Sets whether the `accessed` bit should be `true`.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that are page or block descriptors.
    pub const fn set_page_block_accessed(self, accessed: bool) -> Self {
        Self((self.0 & !(1 << 10)) | (bool_as_u64(accessed) << 10))
    }
}

#[expect(clippy::as_conversions)]
const fn bool_as_u64(value: bool) -> u64 {
    value as u64
}

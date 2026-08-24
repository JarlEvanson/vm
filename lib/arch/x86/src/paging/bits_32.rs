//! 32-bit paging-related utilities.

#![expect(
    missing_docs,
    reason = "bit shift/masking is documented in `TranslationDescriptor`"
)]

/// The number of [`TranslationDescriptor`]s in a [`TranslationDescriptor`] table.
pub const TRANSLATION_DESCRIPTOR_TABLE_LEN_32: usize = 1024;

pub const MAX_PHYSICAL_ADDRESS_SHIFT: u32 = 40;

pub const PRESENT_SHIFT: u32 = 0;
pub const PRESENT_BIT: u32 = 1 << PRESENT_SHIFT;

pub const WRITABLE_SHIFT: u32 = 1;
pub const WRITABLE_BIT: u32 = 1 << WRITABLE_SHIFT;

pub const USER_SHIFT: u32 = 2;
pub const USER_BIT: u32 = 1 << USER_SHIFT;

pub const PWT_SHIFT: u32 = 3;
pub const PWT_BIT: u32 = 1 << PWT_SHIFT;

pub const PCD_SHIFT: u32 = 4;
pub const PCD_BIT: u32 = 1 << PCD_SHIFT;

pub const ACCESSED_SHIFT: u32 = 5;
pub const ACCESSED_BIT: u32 = 1 << ACCESSED_SHIFT;

// Table-related pub constants.
pub const TABLE_ADDRESS_MASK_SHIFT: u32 = 12;
pub const TABLE_ADDRESS_MASK_SIZE: u32 = MAX_PHYSICAL_ADDRESS_SHIFT - TABLE_ADDRESS_MASK_SHIFT;
pub const TABLE_ADDRESS_MASK: u32 =
    ((1 << TABLE_ADDRESS_MASK_SIZE) - 1) << TABLE_ADDRESS_MASK_SHIFT;

pub const TABLE_IGNORED_MASK: u32 = ((1 << 4) - 1) << 8;

// Block-related pub constants.
pub const BLOCK_SHIFT: u32 = 7;
pub const BLOCK_BIT: u32 = 1 << BLOCK_SHIFT;

pub const BLOCK_PAT_SHIFT: u32 = 12;
pub const BLOCK_PAT_BIT: u32 = 1 << BLOCK_PAT_SHIFT;

pub const BLOCK_LOWER_ADDRESS_MASK_SHIFT: u32 = 22;
pub const BLOCK_LOWER_ADDRESS_MASK_SIZE: u32 =
    MAX_PHYSICAL_ADDRESS_SHIFT - BLOCK_LOWER_ADDRESS_MASK_SHIFT;
pub const BLOCK_LOWER_ADDRESS_MASK: u32 =
    ((1 << BLOCK_LOWER_ADDRESS_MASK_SIZE) - 1) << BLOCK_LOWER_ADDRESS_MASK_SHIFT;

pub const BLOCK_UPPER_ADDRESS_MASK_SHIFT: u32 = 13;
pub const BLOCK_UPPER_ADDRESS_MASK_SIZE: u32 = 8;
pub const BLOCK_UPPER_ADDRESS_MASK: u32 =
    ((1 << BLOCK_UPPER_ADDRESS_MASK_SIZE) - 1) << BLOCK_UPPER_ADDRESS_MASK_SHIFT;

// Page-related pub constants.
pub const PAGE_PAT_SHIFT: u32 = 7;
pub const PAGE_PAT_BIT: u32 = 1 << PAGE_PAT_SHIFT;

pub const PAGE_ADDRESS_MASK_SHIFT: u32 = 12;
pub const PAGE_ADDRESS_MASK_SIZE: u32 = MAX_PHYSICAL_ADDRESS_SHIFT - PAGE_ADDRESS_MASK_SHIFT;
pub const PAGE_ADDRESS_MASK: u32 = ((1 << PAGE_ADDRESS_MASK_SIZE) - 1) << PAGE_ADDRESS_MASK_SHIFT;

// Page-or-block-related pub constants.
pub const PAGE_OR_BLOCK_DIRTY_SHIFT: u32 = 6;
pub const PAGE_OR_BLOCK_DIRTY_BIT: u32 = 1 << PAGE_OR_BLOCK_DIRTY_SHIFT;

pub const PAGE_OR_BLOCK_GLOBAL_SHIFT: u32 = 8;
pub const PAGE_OR_BLOCK_GLOBAL_BIT: u32 = 1 << PAGE_OR_BLOCK_GLOBAL_SHIFT;

pub const PAGE_OR_BLOCK_IGNORED_MASK: u32 = ((1 << 3) - 1) << 9;

/// A generic 32-bit translation descriptor.
///
/// This representation performs no validity checking.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationDescriptor(u32);

impl TranslationDescriptor {
    /// Creates a new [`TranslationDescriptor`] that is not present.
    pub const fn non_present() -> Self {
        Self(0)
    }

    /// Creates a new [`TranslationDescriptor`] that is present with the provided
    /// `physical_address`.
    pub const fn new_table(physical_address: u32) -> Self {
        Self::non_present()
            .set_present(true)
            .set_table_address(physical_address)
    }

    /// Constructs a new [`TranslationDescriptor`] from the bit representation.
    pub const fn from_bits(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the bit representation of the [`TranslationDescriptor`].
    pub const fn to_bits(self) -> u32 {
        self.0
    }

    /// Returns `true` if the [`TranslationDescriptor`] describes a present descriptor.
    pub const fn present(self) -> bool {
        self.0 & PRESENT_BIT == PRESENT_BIT
    }

    /// Sets whether the [`TranslationDescriptor`] is present, thereby affecting whether
    /// translation succeeds.
    pub const fn set_present(self, present: bool) -> Self {
        Self((self.0 & !PRESENT_BIT) | (bool_as_u32(present) << PRESENT_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`]
    /// is writable.
    pub const fn writable(self) -> bool {
        self.0 & WRITABLE_BIT == WRITABLE_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] is writable.
    pub const fn set_writable(self, writable: bool) -> Self {
        Self((self.0 & !WRITABLE_BIT) | (bool_as_u32(writable) << WRITABLE_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] is
    /// accessible by the user.
    pub const fn user(self) -> bool {
        self.0 & USER_BIT == USER_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] is user
    /// accessible.
    pub const fn set_user(self, user: bool) -> Self {
        Self((self.0 & !USER_BIT) | (bool_as_u32(user) << USER_SHIFT))
    }

    /// Returns `true` if the [`TranslationDescriptor`] has the `PWT` bit set.
    pub const fn pwt(self) -> bool {
        self.0 & PWT_BIT == PWT_BIT
    }

    /// Sets the `PWT` bit in the [`TranslationDescriptor`].
    pub const fn set_pwt(self, pwt: bool) -> Self {
        Self((self.0 & !PWT_BIT) | (bool_as_u32(pwt) << PWT_SHIFT))
    }

    /// Returns `true` if the [`TranslationDescriptor`] has the `PCD` bit set.
    pub const fn pcd(self) -> bool {
        self.0 & PCD_BIT == PCD_BIT
    }

    /// Sets the `PCD` bit in the [`TranslationDescriptor`].
    pub const fn set_pcd(self, pcd: bool) -> Self {
        Self((self.0 & !PCD_BIT) | (bool_as_u32(pcd) << PCD_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] has been
    /// accessed.
    pub const fn accessed(self) -> bool {
        self.0 & ACCESSED_BIT == ACCESSED_BIT
    }

    /// Sets the accessed bit, which indicates whether the [`TranslationDescriptor`] has been used.
    pub const fn set_accessed(self, accessed: bool) -> Self {
        Self((self.0 & !ACCESSED_BIT) | (bool_as_u32(accessed) << ACCESSED_SHIFT))
    }

    // Table descriptor utilities.

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on table descriptors.
    pub const fn table_address(self) -> u32 {
        self.0 & TABLE_ADDRESS_MASK
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on table descriptors.
    pub const fn set_table_address(self, address: u32) -> Self {
        Self((self.0 & !TABLE_ADDRESS_MASK) | (address & TABLE_ADDRESS_MASK))
    }

    /// Returns the ignored bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on table descriptors.
    pub const fn table_ignored(self) -> u32 {
        self.0 & TABLE_IGNORED_MASK
    }

    // Block descriptor utilities.

    /// Returns `true` if the [`TranslationDescriptor`] is a block descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could possibly be block
    /// descriptors.
    pub const fn block(self) -> bool {
        self.0 & BLOCK_BIT == BLOCK_BIT
    }

    /// Sets whether the [`TranslationDescriptor`] should be treated as a block descriptor.
    ///
    /// This should only be used on [`TranslationDescriptor`]s that could possibly be block
    /// descriptors.
    pub const fn set_block(self, block: bool) -> Self {
        Self((self.0 & !BLOCK_BIT) | (bool_as_u32(block) << BLOCK_SHIFT))
    }

    /// Returns `true` if the [`TranslationDescriptor`] has the `PAT` bit set.
    ///
    /// This should only be used on block descriptors.
    pub const fn block_pat(self) -> bool {
        self.0 & BLOCK_PAT_BIT == BLOCK_PAT_BIT
    }

    /// Sets the `PAT` bit in the [`TranslationDescriptor`].
    ///
    /// This should only be used on block descriptors.
    pub const fn set_block_pat(self, pat: bool) -> Self {
        Self((self.0 & !BLOCK_PAT_BIT) | (bool_as_u32(pat) << BLOCK_PAT_SHIFT))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on block descriptors.
    #[expect(clippy::as_conversions)]
    pub const fn block_address(self) -> u64 {
        (self.0 & BLOCK_LOWER_ADDRESS_MASK) as u64
            | (((self.0 & BLOCK_UPPER_ADDRESS_MASK) as u64)
                << (32 - BLOCK_UPPER_ADDRESS_MASK_SHIFT))
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on block descriptors.
    #[expect(clippy::as_conversions)]
    pub const fn set_block_address(self, address: u64) -> Self {
        let non_address_bits = self.0 & !(BLOCK_LOWER_ADDRESS_MASK | BLOCK_UPPER_ADDRESS_MASK);

        Self(
            non_address_bits
                | ((address & 0xFFFF_FFFF) as u32 & BLOCK_LOWER_ADDRESS_MASK)
                | (((address >> (32 - BLOCK_UPPER_ADDRESS_MASK_SHIFT)) & 0xFFFF_FFFF) as u32),
        )
    }

    // Page descriptor utilities.

    /// Returns `true` if the [`TranslationDescriptor`] has the `PAT` bit set.
    ///
    /// This should only be used on page descriptors.
    pub const fn page_pat(self) -> bool {
        self.0 & PAGE_PAT_BIT == PAGE_PAT_BIT
    }

    /// Sets the `PAT` bit in the [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn set_page_pat(self, pat: bool) -> Self {
        Self((self.0 & !PAGE_PAT_BIT) | (bool_as_u32(pat) << PAGE_PAT_SHIFT))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn page_address(self) -> u32 {
        self.0 & PAGE_ADDRESS_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn set_page_address(self, address: u32) -> Self {
        Self((self.0 & !PAGE_ADDRESS_MASK) | (address & PAGE_ADDRESS_MASK))
    }

    // Page & block descriptor utilities.
    //
    // These utilities should only be used if the [`TranslationDescriptor`] is a page or block
    // descriptor.

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] has been
    /// written to since the last time the dirty bit was cleared.
    ///
    /// This should only be used on page or block descriptors.
    pub const fn page_or_block_dirty(self) -> bool {
        self.0 & PAGE_OR_BLOCK_DIRTY_BIT == PAGE_OR_BLOCK_DIRTY_BIT
    }

    /// Sets the dirty bit, which indicates whether the region of memory controlled by the
    /// [`TranslationDescriptor`] has been written to since the last time the dirty bit was
    /// cleared.
    ///
    /// This should only be used to page or block descriptors.
    pub const fn set_page_or_block_dirty(self, dirty: bool) -> Self {
        Self(
            (self.0 & !PAGE_OR_BLOCK_DIRTY_BIT) | (bool_as_u32(dirty) << PAGE_OR_BLOCK_DIRTY_SHIFT),
        )
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] has
    /// global translation.
    ///
    /// This should only be used on page or block descriptors.
    pub const fn page_or_block_global(self) -> bool {
        self.0 & PAGE_OR_BLOCK_GLOBAL_BIT == PAGE_OR_BLOCK_GLOBAL_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] has global
    /// translation.
    ///
    /// This should only be used on page or block descriptors.
    pub const fn set_page_or_block_global(self, global: bool) -> Self {
        Self(
            (self.0 & !PAGE_OR_BLOCK_GLOBAL_BIT)
                | (bool_as_u32(global) << PAGE_OR_BLOCK_GLOBAL_SHIFT),
        )
    }

    /// Returns the ignored bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on page or block descriptors.
    pub const fn page_or_block_ignored(self) -> u32 {
        self.0 & PAGE_OR_BLOCK_IGNORED_MASK
    }
}

#[expect(clippy::missing_docs_in_private_items)]
const fn bool_as_u32(value: bool) -> u32 {
    value as u32
}

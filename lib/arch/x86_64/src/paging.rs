//! Paging-related utilities.

#![expect(
    clippy::missing_docs_in_private_items,
    reason = "bit shift/masking is documented in `TranslationDescriptor`"
)]

/// The number of [`TranslationDescriptor`]'s in a single [`TranslationDescriptor`] table.
pub const TRANSLATION_DESCRIPTOR_TABLE_LEN: usize = 512;

const MAX_PHYSICAL_ADDRESS_SHIFT: u32 = 52;

const PRESENT_SHIFT: u32 = 0;
const PRESENT_BIT: u64 = 1 << PRESENT_SHIFT;

const WRITABLE_SHIFT: u32 = 1;
const WRITABLE_BIT: u64 = 1 << WRITABLE_SHIFT;

const USER_SHIFT: u32 = 2;
const USER_BIT: u64 = 1 << USER_SHIFT;

const PWT_SHIFT: u32 = 3;
const PWT_BIT: u64 = 1 << PWT_SHIFT;

const PCD_SHIFT: u32 = 4;
const PCD_BIT: u64 = 1 << PCD_SHIFT;

const ACCESSED_SHIFT: u32 = 5;
const ACCESSED_BIT: u64 = 1 << ACCESSED_SHIFT;

const HLAT_RESTART_SHIFT: u32 = 11;
const HLAT_RESTART_BIT: u64 = 1 << HLAT_RESTART_SHIFT;

const XD_SHIFT: u32 = 63;
const XD_BIT: u64 = 1 << XD_SHIFT;

// Table-related constants.
const TABLE_ADDRESS_MASK_SHIFT: u32 = 12;
const TABLE_ADDRESS_MASK_SIZE: u32 = MAX_PHYSICAL_ADDRESS_SHIFT - TABLE_ADDRESS_MASK_SHIFT;
const TABLE_ADDRESS_MASK: u64 = ((1 << TABLE_ADDRESS_MASK_SIZE) - 1) << TABLE_ADDRESS_MASK_SHIFT;

const TABLE_IGNORED_MASK: u64 = !(XD_BIT
    | TABLE_ADDRESS_MASK
    | HLAT_RESTART_BIT
    | (1 << 7) // Explicitly reserved bit.
    | ACCESSED_BIT
    | PCD_BIT
    | PWT_BIT
    | USER_BIT
    | WRITABLE_BIT
    | PRESENT_BIT);

const TABLE_RESERVED_MASK: u64 = 1 << 7; // Only explicitly reserved bit.

// Block-related constants.
const BLOCK_SHIFT: u32 = 7;
const BLOCK_BIT: u64 = 1 << BLOCK_SHIFT;

const BLOCK_PAT_SHIFT: u32 = 12;
const BLOCK_PAT_BIT: u64 = 1 << BLOCK_PAT_SHIFT;

const BLOCK_PML3_ADDRESS_MASK_SHIFT: u32 = 30;
const BLOCK_PML3_ADDRESS_MASK_SIZE: u32 =
    MAX_PHYSICAL_ADDRESS_SHIFT - BLOCK_PML3_ADDRESS_MASK_SHIFT;
const BLOCK_PML3_ADDRESS_MASK: u64 =
    ((1 << BLOCK_PML3_ADDRESS_MASK_SIZE) - 1) << BLOCK_PML3_ADDRESS_MASK_SHIFT;

const BLOCK_PML2_ADDRESS_MASK_SHIFT: u32 = 21;
const BLOCK_PML2_ADDRESS_MASK_SIZE: u32 =
    MAX_PHYSICAL_ADDRESS_SHIFT - BLOCK_PML2_ADDRESS_MASK_SHIFT;
const BLOCK_PML2_ADDRESS_MASK: u64 =
    ((1 << BLOCK_PML2_ADDRESS_MASK_SIZE) - 1) << BLOCK_PML2_ADDRESS_MASK_SHIFT;

const BLOCK_IGNORED_MASK: u64 = !(XD_BIT
    | PAGE_OR_BLOCK_PROTECTION_KEY_MASK
    | BLOCK_PML3_ADDRESS_MASK
    | BLOCK_PML2_ADDRESS_MASK
    | BLOCK_PAT_BIT
    | HLAT_RESTART_BIT
    | PAGE_OR_BLOCK_GLOBAL_BIT
    | BLOCK_BIT
    | PAGE_OR_BLOCK_DIRTY_BIT
    | ACCESSED_BIT
    | PCD_BIT
    | PWT_BIT
    | USER_BIT
    | WRITABLE_BIT
    | PRESENT_BIT);

const BLOCK_PML3_RESERVED_MASK: u64 =
    !(BLOCK_PML3_ADDRESS_MASK & PAGE_ADDRESS_MASK) & PAGE_ADDRESS_MASK;
const BLOCK_PML2_RESERVED_MASK: u64 =
    !(BLOCK_PML2_ADDRESS_MASK & PAGE_ADDRESS_MASK) & PAGE_ADDRESS_MASK;

// Page-related constants.
const PAGE_PAT_SHIFT: u32 = 7;
const PAGE_PAT_BIT: u64 = 1 << PAGE_PAT_SHIFT;

const PAGE_ADDRESS_MASK_SHIFT: u32 = 12;
const PAGE_ADDRESS_MASK_SIZE: u32 = MAX_PHYSICAL_ADDRESS_SHIFT - PAGE_ADDRESS_MASK_SHIFT;
const PAGE_ADDRESS_MASK: u64 = ((1 << PAGE_ADDRESS_MASK_SIZE) - 1) << PAGE_ADDRESS_MASK_SHIFT;

const PAGE_IGNORED_MASK: u64 = !(XD_BIT
    | PAGE_OR_BLOCK_PROTECTION_KEY_MASK
    | PAGE_ADDRESS_MASK
    | HLAT_RESTART_BIT
    | PAGE_OR_BLOCK_GLOBAL_BIT
    | PAGE_PAT_BIT
    | PAGE_OR_BLOCK_DIRTY_BIT
    | ACCESSED_BIT
    | PCD_BIT
    | PWT_BIT
    | USER_BIT
    | WRITABLE_BIT
    | PRESENT_BIT);

// Page-or-block-related constants.
const PAGE_OR_BLOCK_DIRTY_SHIFT: u32 = 6;
const PAGE_OR_BLOCK_DIRTY_BIT: u64 = 1 << PAGE_OR_BLOCK_DIRTY_SHIFT;

const PAGE_OR_BLOCK_GLOBAL_SHIFT: u32 = 8;
const PAGE_OR_BLOCK_GLOBAL_BIT: u64 = 1 << PAGE_OR_BLOCK_GLOBAL_SHIFT;

const PAGE_OR_BLOCK_PROTECTION_KEY_SHIFT: u32 = 59;
const PAGE_OR_BLOCK_PROTECTION_KEY_SIZE: u32 = 4;
const PAGE_OR_BLOCK_PROTECTION_KEY_MASK: u64 =
    ((1 << PAGE_OR_BLOCK_PROTECTION_KEY_SIZE) - 1) << PAGE_OR_BLOCK_PROTECTION_KEY_SHIFT;

/// A generic translation descriptor, which can be either a table descriptor, block descriptor, or
/// page descriptor.
///
/// This representation does not do any validity checking.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationDescriptor(u64);

impl TranslationDescriptor {
    /// Creates a new [`TranslationDescriptor`] that is not present.
    pub const fn non_present() -> Self {
        Self(0)
    }

    /// Creates a new [`TranslationDescriptor`] that is present with the provided
    /// `physical_address`.
    pub const fn new_table(physical_address: u64) -> Self {
        Self::non_present()
            .set_present(true)
            .set_table_address(physical_address)
    }

    /// Creates a new [`TranslationDescriptor`] that is present with the provided
    /// `physical_address` and is a PML3 block descriptor.
    pub const fn new_block_pml3(physical_address: u64) -> Self {
        Self::non_present()
            .set_present(true)
            .set_block(true)
            .set_block_pml3_address(physical_address)
    }

    /// Creates a new [`TranslationDescriptor`] that is present with the provided
    /// `physical_address` and is a PML2 block descriptor.
    pub const fn new_block_pml2(physical_address: u64) -> Self {
        Self::non_present()
            .set_present(true)
            .set_block(true)
            .set_block_pml2_address(physical_address)
    }

    /// Creates a new [`TranslationDescriptor`] that is present with the provided
    /// `physical_address`.
    pub const fn new_page(physical_address: u64) -> Self {
        Self::non_present()
            .set_present(true)
            .set_page_address(physical_address)
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

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`]
    /// is writable.
    pub const fn writable(self) -> bool {
        self.0 & WRITABLE_BIT == WRITABLE_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] is writable.
    pub const fn set_writable(self, writable: bool) -> Self {
        Self((self.0 & !WRITABLE_BIT) | (bool_as_u64(writable) << WRITABLE_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] is
    /// accessible by the user.
    pub const fn user(self) -> bool {
        self.0 & USER_BIT == USER_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] is user
    /// accessible.
    pub const fn set_user(self, user: bool) -> Self {
        Self((self.0 & !USER_BIT) | (bool_as_u64(user) << USER_SHIFT))
    }

    /// Returns `true` if the [`TranslationDescriptor`] has the `PWT` bit set.
    pub const fn pwt(self) -> bool {
        self.0 & PWT_BIT == PWT_BIT
    }

    /// Sets the `PWT` bit in the [`TranslationDescriptor`].
    pub const fn set_pwt(self, pwt: bool) -> Self {
        Self((self.0 & !PWT_BIT) | (bool_as_u64(pwt) << PWT_SHIFT))
    }

    /// Returns `true` if the [`TranslationDescriptor`] has the `PCD` bit set.
    pub const fn pcd(self) -> bool {
        self.0 & PCD_BIT == PCD_BIT
    }

    /// Sets the `PCD` bit in the [`TranslationDescriptor`].
    pub const fn set_pcd(self, pcd: bool) -> Self {
        Self((self.0 & !PCD_BIT) | (bool_as_u64(pcd) << PCD_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] has been
    /// accessed.
    pub const fn accessed(self) -> bool {
        self.0 & ACCESSED_BIT == ACCESSED_BIT
    }

    /// Sets the accessed bit, which indicates whether the [`TranslationDescriptor`] has been used.
    pub const fn set_accessed(self, accessed: bool) -> Self {
        Self((self.0 & !ACCESSED_BIT) | (bool_as_u64(accessed) << ACCESSED_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] has
    /// `HLAT` restart bit set.
    pub const fn hlat_restart(self) -> bool {
        self.0 & HLAT_RESTART_BIT == HLAT_RESTART_BIT
    }

    /// Sets the `HLAT` restart bit for the region of memory controlled by the
    /// [`TranslationDescriptor`].
    pub const fn set_hlat_restart(self, hlat_restart: bool) -> Self {
        Self((self.0 & !HLAT_RESTART_BIT) | (bool_as_u64(hlat_restart) << HLAT_RESTART_SHIFT))
    }

    /// Returns `true` if the region of memory controlled by the [`TranslationDescriptor`] is not
    /// executable.
    ///
    /// This bit is only valid if the `NXE` feature is enabled.
    pub const fn xd(self) -> bool {
        self.0 & XD_BIT == XD_BIT
    }

    /// Sets whether the region of memory controlled by the [`TranslationDescriptor`] is not
    /// executable.
    pub const fn set_xd(self, xd: bool) -> Self {
        Self((self.0 & !XD_BIT) | (bool_as_u64(xd) << XD_SHIFT))
    }

    // Table descriptor utilities.

    /// Returns the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on table descriptors.
    pub const fn table_address(self) -> u64 {
        self.0 & TABLE_ADDRESS_MASK
    }

    /// Sets the physical address of the next table in the address translation hierarchy.
    ///
    /// This should only be used on table descriptors.
    pub const fn set_table_address(self, address: u64) -> Self {
        Self((self.0 & !TABLE_ADDRESS_MASK) | (address & TABLE_ADDRESS_MASK))
    }

    /// Returns the ignored bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on table descriptors.
    pub const fn table_ignored(self) -> u64 {
        self.0 & TABLE_IGNORED_MASK
    }

    /// Returns the reserved bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on table descriptors.
    pub const fn table_reserved(self) -> u64 {
        self.0 & TABLE_RESERVED_MASK
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
        Self((self.0 & !BLOCK_BIT) | (bool_as_u64(block) << BLOCK_SHIFT))
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
        Self((self.0 & !BLOCK_PAT_BIT) | (bool_as_u64(pat) << BLOCK_PAT_SHIFT))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on PML3 block descriptors.
    pub const fn block_pml3_address(self) -> u64 {
        self.0 & BLOCK_PML3_ADDRESS_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on PML3 block descriptors.
    pub const fn set_block_pml3_address(self, address: u64) -> Self {
        Self((self.0 & !BLOCK_PML3_ADDRESS_MASK) | (address & BLOCK_PML3_ADDRESS_MASK))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on PML2 block descriptors.
    pub const fn block_pml2_address(self) -> u64 {
        self.0 & BLOCK_PML2_ADDRESS_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on PML2 block descriptors.
    pub const fn set_block_pml2_address(self, address: u64) -> Self {
        Self((self.0 & !BLOCK_PML2_ADDRESS_MASK) | (address & BLOCK_PML2_ADDRESS_MASK))
    }

    /// Returns the ignored bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on block descriptors.
    pub const fn block_ignored(self) -> u64 {
        self.0 & BLOCK_IGNORED_MASK
    }

    /// Returns the reserved bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on PML3 block descriptors.
    pub const fn block_pml3_reserved(self) -> u64 {
        self.0 & BLOCK_PML3_RESERVED_MASK
    }

    /// Returns the reserved bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on PML2 block descriptors.
    pub const fn block_pml2_reserved(self) -> u64 {
        self.0 & BLOCK_PML2_RESERVED_MASK
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
        Self((self.0 & !PAGE_PAT_BIT) | (bool_as_u64(pat) << PAGE_PAT_SHIFT))
    }

    /// Returns the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn page_address(self) -> u64 {
        self.0 & PAGE_ADDRESS_MASK
    }

    /// Sets the physical address of the region of memory controlled by the
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn set_page_address(self, address: u64) -> Self {
        Self((self.0 & !PAGE_ADDRESS_MASK) | (address & PAGE_ADDRESS_MASK))
    }

    /// Returns the ignored bits of the [`TranslationDescriptor`].
    ///
    /// This should only be used on page descriptors.
    pub const fn page_ignored(self) -> u64 {
        self.0 & PAGE_IGNORED_MASK
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
            (self.0 & !PAGE_OR_BLOCK_DIRTY_BIT) | (bool_as_u64(dirty) << PAGE_OR_BLOCK_DIRTY_SHIFT),
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
                | (bool_as_u64(global) << PAGE_OR_BLOCK_GLOBAL_SHIFT),
        )
    }

    /// Returns the protection key associated with the region of memory controlled by
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page or block descriptors.
    pub const fn page_or_block_protection_key(self) -> u64 {
        (self.0 & PAGE_OR_BLOCK_PROTECTION_KEY_MASK) >> PAGE_OR_BLOCK_PROTECTION_KEY_SHIFT
    }

    /// Sets the protection key associated with the region of memory controlled by
    /// [`TranslationDescriptor`].
    ///
    /// This should only be used on page or block descriptors.
    pub const fn set_page_or_block_protection_key(self, key: u64) -> Self {
        Self(
            (self.0 & !PAGE_OR_BLOCK_PROTECTION_KEY_MASK)
                | (key << PAGE_OR_BLOCK_PROTECTION_KEY_SHIFT) & PAGE_OR_BLOCK_PROTECTION_KEY_MASK,
        )
    }
}

#[expect(clippy::as_conversions)]
const fn bool_as_u64(value: bool) -> u64 {
    value as u64
}

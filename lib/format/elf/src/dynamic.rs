//! Ergonomic wrapper over ELF dynamic structures.

use core::fmt;

use crate::{
    class::ClassBase,
    encoding::Encoding,
    extract_format,
    medium::{Medium, MediumError},
    table::{Table, TableItem},
};

/// A [`Table`] of [`Dynamic`] structures.
pub type DynamicTable<'slice, M, C, E> = Table<'slice, M, C, E, Dynamic<'slice, M, C, E>>;

/// An ELF dynamic structure.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Dynamic<'slice, M: ?Sized, C, E> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// The offset of the [`Dynamic`].
    offset: u64,
    /// The [`Class`][crate::class::Class] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
}

#[expect(clippy::missing_errors_doc)]
impl<'slice, M: Medium + ?Sized, C: ClassDynamic, E: Encoding> Dynamic<'slice, M, C, E> {
    /// Returns the [`DynamicTag`] associated with this [`Dynamic`] structure.
    pub fn tag(&self) -> Result<DynamicTag, MediumError<M::Error>> {
        self.class
            .read_class_isize(
                self.encoding,
                self.offset + ClassDynamic::dynamic_tag_offset(self.class),
                self.medium,
            )
            .map(Into::<i64>::into)
            .map(DynamicTag)
    }

    /// Returns the value or pointer associated with this [`Dynamic`] structure.
    pub fn val_ptr(&self) -> Result<C::ClassUsize, MediumError<M::Error>> {
        self.class.read_class_usize(
            self.encoding,
            self.offset + ClassDynamic::dynamic_val_ptr_offset(self.class),
            self.medium,
        )
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassDynamic, E: Encoding> fmt::Debug
    for Dynamic<'slice, M, C, E>
where
    <M as Medium>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = self.tag();
        let val_ptr = self.val_ptr();

        f.debug_struct("Dynamic")
            .field("tag", extract_format(&tag))
            .field("val_ptr", extract_format(&val_ptr))
            .finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: ClassDynamic, E: Encoding> TableItem<'slice, M, C, E>
    for Dynamic<'slice, M, C, E>
{
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self {
        let max_offset = offset
            .checked_add(class.expected_dynamic_size())
            .expect("overflow when calculating max offset");
        assert!(max_offset <= medium.size(), "out of bounds structure");

        Self {
            medium,
            offset,
            class,
            encoding,
        }
    }

    fn expected_size(c: C) -> u64 {
        c.expected_dynamic_size()
    }
}

/// Indicator of the interpretation of [`Dynamic::val_ptr()`].
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct DynamicTag(pub i64);

impl DynamicTag {
    /// Marks the end of the ELF dynamic array.
    pub const NULL: Self = Self(0);
    /// Holds the string table offset of a null-terminated string which is the name of a needed
    /// library. This offset is an index into the table recording in the
    /// [`DynamicTag::STRING_TABLE`] entry.
    ///
    /// The dynamic array may contain multiple entries with this type, and the order of these
    /// entries is significant, but only relative to entries of the same type.
    pub const NEEDED: Self = Self(1);
    /// Holds the total size, in bytes, of the relocation entries associated with the procedure
    /// linkage table. If an [`DynamicTag::JMP_REL`] entry is present, this tag must accompany
    /// it.
    pub const PLT_REL_SIZE: Self = Self(2);
    /// Holds an address associated with the procedure linkage table and/or the global offset
    /// table.
    pub const PLT_GOT: Self = Self(3);
    /// Holds the address of the symbol hash table, which refers to the symbol table referenced in
    /// an [`DynamicTag::SYMBOL_TABLE`] entry.
    pub const HASH: Self = Self(4);
    /// Holds the address of the string table.
    pub const STRING_TABLE: Self = Self(5);
    /// Holds the address of the symbol table.
    pub const SYMBOL_TABLE: Self = Self(6);
    /// Holds the address of a relocation table, with explicit addends.
    ///
    /// If this entry is present, the dynamic array must also have [`DynamicTag::RELA_SIZE`] and
    /// [`DynamicTag::RELA_ENTRY_SIZE`] entries.
    pub const RELA_TABLE: Self = Self(7);
    /// Holds the total size, in bytes, of the relocation table pointed to by the [`DynamicTag::RELA_TABLE`] .
    pub const RELA_SIZE: Self = Self(8);
    /// Holds the size, in bytes, of the relocation table pointed to by the
    /// [`DynamicTag::RELA_TABLE`].
    pub const RELA_ENTRY_SIZE: Self = Self(9);
    /// Holds the total size, in bytes, of the string table pointed to by the
    /// [`DynamicTag::STRING_TABLE`] entry.
    pub const STRING_TABLE_SIZE: Self = Self(10);
    /// Holds the size, in bytes, of an entry in the symbol table pointed to by the
    /// [`DynamicTag::SYMBOL_TABLE`] entry.
    pub const SYMBOL_ENTRY_SIZE: Self = Self(11);
    /// Holds the address of the initialization function.
    pub const INIT: Self = Self(12);
    /// Holds the address of the termination function.
    pub const FINI: Self = Self(13);
    /// Holds the string table offset of a null-terminated string giving the name of the shared
    /// object.
    pub const SO_NAME: Self = Self(14);
    /// Holds the string table offset of a null-terminated string giving the library search path
    /// string.
    ///
    /// The use of this has been superseded by [`DynamicTag::RUNPATH`].
    pub const RPATH: Self = Self(15);
    /// Indicates that the dynamic linker's symbol resolution algorithm should start from the
    /// shared object and then if the shared object fails to provide the referenced symbol, then
    /// the linker searches the executable file and other shared objects as usual.
    pub const SYMBOLIC: Self = Self(16);
    /// Holds the address of a relocation table, with implicit addends.
    ///
    /// If this entry is present, the dynamic array must also have [`DynamicTag::REL_SIZE`] and
    /// [`DynamicTag::REL_ENTRY_SIZE`] entries.
    pub const REL_TABLE: Self = Self(17);
    /// The total size, in bytes, of the relocation table pointed to by the
    /// [`DynamicTag::REL_TABLE`] entry.
    pub const REL_SIZE: Self = Self(18);
    /// The size, in bytes, of an entry in the relocation table pointed to by the
    /// [`DynamicTag::REL_TABLE`] entry.
    pub const REL_ENTRY_SIZE: Self = Self(19);
    /// The type of relocation entry to which the procedure linkage table refers.
    pub const PLT_REL: Self = Self(20);
    /// This member is used for debugging, but its contents are not specified by the ABI.
    pub const DEBUG: Self = Self(21);
    /// Indicates that one or more relocation entries might cause a modification to a non-writable segment.
    ///
    /// The use of this has been superseded by [`DynamicTag::FLAGS`] `TEXTREL`.
    pub const TEXT_REL: Self = Self(22);
    /// Holds the address of relocation entries associated solely with the procedure linkage table.
    ///
    /// If this entry is present, the dynamic array must also have [`DynamicTag::PLT_REL`] and
    /// [`DynamicTag::PLT_REL_SIZE`] entries.
    pub const JMP_REL: Self = Self(23);
    /// Indicates that the dynamic linker should process all relocations for the object containing
    /// this entry before transferring control to the program.
    pub const BIND_NOW: Self = Self(24);
    /// Holds the address of the array of pointers to initialization functions.
    pub const INIT_ARRAY: Self = Self(25);
    /// Holds the address of the array of pointers to termination functions.
    pub const FINI_ARRAY: Self = Self(26);
    /// Holds the size, in bytes, of the array of pointers to initialization functions.
    pub const INIT_ARRAY_SIZE: Self = Self(27);
    /// Holds the size, in bytes, of the array of pointers to termination functions.
    pub const FINI_ARRAY_SIZE: Self = Self(28);
    /// Holds the string table offset of the null-terminated library search path string.
    pub const RUNPATH: Self = Self(29);
    /// Holds flag values specific to the object being loaded.
    pub const FLAGS: Self = Self(30);

    /// Holds the address of the array of pointers to pre-initialization functions.
    ///
    /// This is processed only in an executable file.
    pub const PREINIT_ARRAY: Self = Self(32);
    /// Holds the size, in bytes, of the array of pointers to pre-initialization functions.
    pub const PREINIT_ARRAY_SIZE: Self = Self(33);
    /// Holds the address of the SHT_SYMTAB_SHNDX section associated with the dynamic symbol
    /// table referenced by the [`DynamicTag::SYMBOL_TABLE`] element.
    pub const SYMBOL_TABLE_SECTION_INDEX: Self = Self(34);
}

impl fmt::Debug for DynamicTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DynamicTag::NULL => f.pad("Null"),
            DynamicTag::NEEDED => f.pad("Needed"),
            DynamicTag::PLT_REL_SIZE => f.pad("PltRelSize"),
            DynamicTag::PLT_GOT => f.pad("PltGot"),
            DynamicTag::HASH => f.pad("Hash"),
            DynamicTag::STRING_TABLE => f.pad("StringTable"),
            DynamicTag::SYMBOL_TABLE => f.pad("SymbolTable"),
            DynamicTag::RELA_TABLE => f.pad("RelaTable"),
            DynamicTag::RELA_SIZE => f.pad("RelaSize"),
            DynamicTag::RELA_ENTRY_SIZE => f.pad("RelaEntrySize"),
            DynamicTag::STRING_TABLE_SIZE => f.pad("StringTableSize"),
            DynamicTag::SYMBOL_ENTRY_SIZE => f.pad("SymbolEntrySize"),
            DynamicTag::INIT => f.pad("Init"),
            DynamicTag::FINI => f.pad("Fini"),
            DynamicTag::SO_NAME => f.pad("SoName"),
            DynamicTag::RPATH => f.pad("RPath"),
            DynamicTag::SYMBOLIC => f.pad("Symbolic"),
            DynamicTag::REL_TABLE => f.pad("RelTable"),
            DynamicTag::REL_SIZE => f.pad("RelSize"),
            DynamicTag::REL_ENTRY_SIZE => f.pad("RelEntrySize"),
            DynamicTag::PLT_REL => f.pad("PltRel"),
            DynamicTag::DEBUG => f.pad("Debug"),
            DynamicTag::TEXT_REL => f.pad("TextRel"),
            DynamicTag::JMP_REL => f.pad("JmpRel"),
            DynamicTag::BIND_NOW => f.pad("BindNow"),
            DynamicTag::INIT_ARRAY => f.pad("InitArray"),
            DynamicTag::FINI_ARRAY => f.pad("FiniArray"),
            DynamicTag::INIT_ARRAY_SIZE => f.pad("InitArraySize"),
            DynamicTag::FINI_ARRAY_SIZE => f.pad("FiniArraySize"),
            DynamicTag::RUNPATH => f.pad("RunPath"),
            DynamicTag::FLAGS => f.pad("Flags"),
            DynamicTag::PREINIT_ARRAY => f.pad("PreinitArray"),
            DynamicTag::PREINIT_ARRAY_SIZE => f.pad("PreinitArraySize"),
            DynamicTag::SYMBOL_TABLE_SECTION_INDEX => f.pad("SymbolTableSectionIndex"),
            dynamic_tag => f.debug_tuple("DynamicTag").field(&dynamic_tag.0).finish(),
        }
    }
}

/// The requirements to implement class aware parsing of ELF dynamic structures.
pub trait ClassDynamic: ClassBase {
    /// The offset of the tag of the ELF dynamic structure.
    fn dynamic_tag_offset(self) -> u64;
    /// The offset of the value or pointer of the ELF dynamic structure.
    fn dynamic_val_ptr_offset(self) -> u64;

    /// The expected size of an ELF dynamic structure.
    fn expected_dynamic_size(self) -> u64;
}

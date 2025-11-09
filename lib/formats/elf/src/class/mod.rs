//! Class aware reading.

use core::{error, fmt};

use crate::{
    Encoding,
    Medium,
    file_header::ClassFileHeader,
    ident,
    program_header::ClassProgramHeader,
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol, //symbol::ClassSymbol,
};

mod class_32;
mod class_64;
mod merge;

pub use class_32::Class32;
pub use class_64::Class64;
pub use merge::Merge;

/// A zero-sized object offering methods for safe parsing of 32-bit and 64-bit ELF files.
pub type AnyClass = Merge<Class32, Class64>;

/// A combination of all other class parsing traits.
pub trait Class:
    ClassBase
    + ClassFileHeader
    + ClassSectionHeader
    + ClassSymbol
    + ClassRelocation
    + ClassProgramHeader
{
}

impl<
    C: ClassBase
        + ClassFileHeader
        + ClassSectionHeader
        + ClassSymbol
        + ClassRelocation
        + ClassProgramHeader,
> Class for C
{
}

/// The base definitions of a class aware parser.
pub trait ClassBase: Clone + Copy {
    /// An unsigned class sized integer.
    type ClassUsize: Clone
        + Copy
        + TryInto<usize>
        + fmt::Debug
        + fmt::Display
        + Eq
        + Ord
        + Into<u64>;
    /// A signed class sized integer.
    type ClassIsize: Clone + Copy + fmt::Debug + fmt::Display + Eq + Ord + Into<i64>;

    /// Returns the [`ClassBase`] instance that corresponds with the given [`ident::Class`].
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedClassError`] if the given [`ident::Class`] is not supported by this
    /// [`ClassBase`] implementation.
    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError>;

    /// Returns the unsigned class sized integer at `offset` bytes from the start of the slice.
    ///
    /// # Panics
    ///
    /// Panics if an arithmetic or bounds overflow error occurs.
    fn parse_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassUsize;

    /// Returns the signed class sized integer at `offset` bytes from the start of the slice.
    ///
    /// # Panics
    ///
    /// Panics if an arithmetic or bounds overflow error occurs.
    fn parse_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Self::ClassIsize;
}

/// An error that occurs when the code does not support a particular [`ident::Class`]
/// object.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnsupportedClassError(ident::Class);

impl fmt::Display for UnsupportedClassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ident::Class::NONE => write!(f, "no class ELF parsing not supported"),
            ident::Class::CLASS32 => write!(f, "32-bit ELF file parsing not supported"),
            ident::Class::CLASS64 => write!(f, "64-bit ELF file parsing not supported"),
            ident::Class(class) => write!(f, "unknown class({class}) not supported"),
        }
    }
}

impl error::Error for UnsupportedClassError {}

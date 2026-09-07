//! Class aware reading.

use core::{error, fmt};

use crate::{
    dynamic::ClassDynamic,
    encoding::Encoding,
    header::ClassElfHeader,
    ident,
    medium::{Medium, MediumError},
    program_header::ClassProgramHeader,
    relocation::ClassRelocation,
    section_header::ClassSectionHeader,
    symbol::ClassSymbol,
};

pub mod class_32;
pub mod class_64;
pub mod class_any;

/// A combination of all other class parsing traits.
pub trait Class:
    ClassBase
    + ClassElfHeader
    + ClassSectionHeader
    + ClassProgramHeader
    + ClassRelocation
    + ClassDynamic
    + ClassSymbol
{
}

impl<
    C: ClassBase
        + ClassElfHeader
        + ClassSectionHeader
        + ClassProgramHeader
        + ClassRelocation
        + ClassDynamic
        + ClassSymbol,
> Class for C
{
}

/// The base definitions of a class aware parser.
#[expect(
    clippy::missing_errors_doc,
    reason = "errors are documented in trait implementation"
)]
pub trait ClassBase: Clone + Copy {
    /// An unsigned class sized integer.
    type ClassUsize: Copy + fmt::Debug + fmt::Display + Eq + Ord + Into<u64>;
    /// A signed class sized integer.
    type ClassIsize: Copy + fmt::Debug + fmt::Display + Eq + Ord + Into<i64>;

    /// Returns the [`ClassBase`] instance that corresponds with the given [`ident::Class`].
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedClassError`] if the given [`ident::Class`] is not supported by this
    /// [`ClassBase`] implementation.
    fn from_elf_class(class: ident::Class) -> Result<Self, UnsupportedClassError>;

    /// Returns the unsigned class sized integer at `offset` bytes from the start of the slice.
    fn read_class_usize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassUsize, MediumError<M::Error>>;

    /// Returns the signed class sized integer at `offset` bytes from the start of the slice.
    fn read_class_isize<E: Encoding, M: Medium + ?Sized>(
        self,
        encoding: E,
        offset: u64,
        medium: &M,
    ) -> Result<Self::ClassIsize, MediumError<M::Error>>;
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

//! Elf parsing functionality.

use core::{error, fmt};

use elf::{
    Elf,
    class::class_any::AnyClass,
    encoding::AnyEndian,
    header::{ElfHeaderError, ElfType, Machine},
    medium::MediumError,
    program_header::ProgramHeaderTable,
};

/// Useful information extracted from an [`Elf`] file.
pub struct ParsedElf<'a> {
    /// The [`Elf`] file from which the useful data has been extracted.
    pub elf: Elf<'a, [u8], AnyClass, AnyEndian>,
    /// The [`Machine`] associated with the [`Elf`] file.
    pub machine: Machine,
    /// The [`ElfType`] of the [`Elf`] file.
    pub elf_type: ElfType,
    /// The entry point of the [`Elf`] file.
    pub entry_point: u64,
    /// The [`ProgramHeaderTable`] of the [`Elf`] table.
    pub program_headers: ProgramHeaderTable<'a, [u8], AnyClass, AnyEndian>,
}

/// Parses the provided `blob` as an [`Elf`] and extract relevant information.
///
/// # Errors
///
/// - [`ParseElfError::ElfHeaderError`]: Returned if an error occurs while parsing the [`Elf`]
///   header.
/// - [`ParseElfError::MediumError`]: Returned if an error occurs while accessing the underlying
///   medium.
/// - [`ParseElfError::MissingProgramHeadersError`]: Returned if the provided [`Elf`] file has no
///   [`ProgramHeaderTable`]
pub fn parse(blob: &[u8]) -> Result<ParsedElf<'_>, ParseElfError<core::convert::Infallible>> {
    let file = Elf::new(blob)?;

    let machine = file.header().machine()?;
    let elf_type = file.header().elf_type()?;
    let entry_point = file.header().entry()?;
    let Some(program_headers) = file.program_header_table()? else {
        return Err(ParseElfError::MissingProgramHeadersError);
    };

    Ok(ParsedElf {
        elf: file,
        machine,
        elf_type,
        entry_point,
        program_headers,
    })
}

/// Various errors that can occur when parsing an [`Elf`] file and extracting useful information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseElfError<E> {
    /// An error occured when parsing the [`Elf`] header.
    ElfHeaderError(ElfHeaderError<E>),
    /// An error occur when accessing the underlying medium.
    MediumError(MediumError<E>),
    /// The provided executable file does not contain a [`ProgramHeaderTable`] and as such is not
    /// loadable.
    MissingProgramHeadersError,
}

impl<E> From<ElfHeaderError<E>> for ParseElfError<E> {
    fn from(error: ElfHeaderError<E>) -> Self {
        Self::ElfHeaderError(error)
    }
}

impl<E> From<MediumError<E>> for ParseElfError<E> {
    fn from(error: MediumError<E>) -> Self {
        Self::MediumError(error)
    }
}

impl<E: fmt::Display> fmt::Display for ParseElfError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElfHeaderError(error) => write!(f, "error parsing ELF header: {error}"),
            Self::MediumError(error) => write!(f, "error accessing ELF data: {error}"),
            Self::MissingProgramHeadersError => {
                f.pad("provided ELF file is missing program headers")
            }
        }
    }
}

impl<E: fmt::Debug + fmt::Display> error::Error for ParseElfError<E> {}

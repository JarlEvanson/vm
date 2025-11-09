//! Generic wrapper over typed ELF tables.

use core::{fmt, marker::PhantomData};

use crate::{Class, Encoding, Medium};

/// A generic ELF table implementation.
#[derive(Hash, PartialEq, Eq)]
pub struct Table<'slice, M: ?Sized, C, E, T> {
    /// The underlying [`Medium`] of the ELF file.
    medium: &'slice M,
    /// THe offset of the start of the [`Table`].
    offset: u64,
    /// The number of `T`s in the [`Table`].
    count: u64,
    /// The stride between each `T`.
    size: u64,
    /// The [`Class`] used to decode the ELF file.
    class: C,
    /// The [`Encoding`] used to decode the ELF file.
    encoding: E,
    /// Phantom type.
    phantom: PhantomData<T>,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E>>
    Table<'slice, M, C, E, T>
{
    /// Creates a new [`Table`] from the given [`Medium`].
    ///
    /// The [`Table`] has `count` entries of `size` bytes at `offset`.
    pub fn new(
        class: C,
        encoding: E,
        medium: &'slice M,
        offset: u64,
        count: u64,
        size: u64,
    ) -> Option<Self> {
        if size < T::expected_size(class) {
            return None;
        }

        let total_size = count.checked_mul(size)?;
        let max_offset = offset.checked_add(total_size)?;
        if max_offset > medium.size() {
            return None;
        }

        let table = Self {
            medium,
            offset,
            count,
            size,
            class,
            encoding,
            phantom: PhantomData,
        };

        Some(table)
    }

    /// Returns the `T` located at `index`.
    pub fn get(&self, index: u64) -> Option<T> {
        if index >= self.count {
            return None;
        }

        let offset = self.offset + index * self.size;
        let item = T::new_panicking(self.class, self.encoding, offset, self.medium);
        Some(item)
    }

    /// Returns the number of `T`s in the [`Table`].
    pub fn count(&self) -> u64 {
        self.count
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E>> IntoIterator
    for Table<'slice, M, C, E, T>
{
    type Item = T;
    type IntoIter = IntoIter<'slice, M, C, E, T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            table: self,
            next: 0,
        }
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E> + fmt::Debug>
    fmt::Debug for Table<'slice, M, C, E, T>
where
    &'slice M: Copy,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_list = f.debug_list();

        debug_list.entries(IntoIter {
            table: *self,
            next: 0,
        });

        debug_list.finish()
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E>> Clone
    for Table<'slice, M, C, E, T>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E>> Copy
    for Table<'slice, M, C, E, T>
{
}

/// An [`Iterator`] over the contents of a [`Table`].
#[derive(Hash, PartialEq, Eq)]
pub struct IntoIter<'slice, M: ?Sized, C, E, T> {
    /// The [`Table`] to iterate over.
    table: Table<'slice, M, C, E, T>,
    /// The next index in the [`Table`].
    next: u64,
}

impl<'slice, M: Medium + ?Sized, C: Class, E: Encoding, T: TableItem<'slice, M, C, E>> Iterator
    for IntoIter<'slice, M, C, E, T>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.table.get(self.next)?;
        self.next += 1;
        Some(item)
    }
}

/// Trait representing an item that can appear in ELF tables.
pub trait TableItem<'slice, M: Medium + ?Sized, C: Class, E: Encoding> {
    /// Creates a new [`Self`], panicking when necessary to maintain safety.
    fn new_panicking(class: C, encoding: E, offset: u64, medium: &'slice M) -> Self;

    /// Returns the expected size of the [`TableItem`].
    fn expected_size(c: C) -> u64;
}

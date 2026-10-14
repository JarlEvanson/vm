//! Linked list implementation to be used for a graph-based initialization subsystem.

use sync::ControlledModificationCell;

/// A singly linked list of values.
#[derive(Debug)]
pub(in crate::graph) struct LinkedList<'a, T: ?Sized> {
    /// The start of the [`LinkedList`].
    head: Option<&'a ControlledModificationCell<Link<'a, T>>>,
    /// The last node of the [`LinkedList`].
    tail: Option<&'a ControlledModificationCell<Link<'a, T>>>,
}

impl<'a, T: ?Sized> LinkedList<'a, T> {
    /// Constructs an empty [`LinkedList`].
    pub(in crate::graph) const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    /// Removes the first [`Link`] from the start of this [`LinkedList`] and returns it.
    pub(in crate::graph) fn pop_front(
        &mut self,
    ) -> Option<&'a ControlledModificationCell<Link<'a, T>>> {
        let head = self.head?;
        self.head = head.get().next;
        if self.head.is_none() {
            self.tail = None;
        }

        Some(head)
    }

    /// Places the provided [`Link`] at the end of this [`LinkedList`].
    pub(in crate::graph) fn push_back(
        &mut self,
        link: &'a ControlledModificationCell<Link<'a, T>>,
    ) {
        // SAFETY:
        //
        // The user does not have any other links active.
        let link_mut = unsafe { link.get_mut() };
        link_mut.next = None;

        if self.head.is_none() {
            self.head = Some(link);
        }

        if let Some(tail) = self.tail {
            // SAFETY:
            //
            // The user does not have any other links active.
            let tail_mut = unsafe { tail.get_mut() };
            tail_mut.next = Some(link);
        }

        self.tail = Some(link);
    }

    /// Returns an [`Iterator`] over the [`Link`]s in the [`LinkedList`].
    pub(in crate::graph) fn link_iter(&self) -> LinkIter<'a, T> {
        LinkIter {
            current_link: self.head,
        }
    }

    /// Returns an [`Iterator`] over the values in the [`LinkedList`].
    pub(in crate::graph) fn iter(&self) -> LinkValueIter<'a, T> {
        LinkValueIter(self.link_iter())
    }
}

/// An [`Iterator`] over the [`Link`]s in a [`LinkedList`].
pub(in crate::graph) struct LinkIter<'a, T: ?Sized> {
    /// The current [`Link`] in the list.
    current_link: Option<&'a ControlledModificationCell<Link<'a, T>>>,
}

impl<'a, T: ?Sized> Iterator for LinkIter<'a, T> {
    type Item = &'a ControlledModificationCell<Link<'a, T>>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_link = self.current_link?;
        self.current_link = current_link.get().next;
        Some(current_link)
    }
}

/// An [`Iterator`] over the values in a [`LinkedList`].
pub(in crate::graph) struct LinkValueIter<'a, T: ?Sized>(LinkIter<'a, T>);

impl<'a, T: ?Sized> Iterator for LinkValueIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let link = self.0.next()?;
        Some(link.get().value())
    }
}

/// A single chain in the [`LinkedList`].
#[doc(hidden)]
#[derive(Debug)]
pub struct Link<'a, T: ?Sized> {
    /// The associated value.
    value: Option<&'a T>,
    /// The next [`Link`] in the [`LinkedList`].
    next: Option<&'a ControlledModificationCell<Link<'a, T>>>,
}

impl<'a, T: ?Sized> Link<'a, T> {
    /// Constructs an empty [`Link`].
    #[doc(hidden)]
    pub const fn empty() -> Self {
        Self {
            value: None,
            next: None,
        }
    }

    /// Returns the value associated with this [`Link`].
    pub(in crate::graph) const fn value(&self) -> &'a T {
        self.value
            .expect("Link::value() must only be called after the Link has been initialized")
    }

    /// Sets the value associated with this [`Link`].
    pub(in crate::graph) const fn set_value(&mut self, value: Option<&'a T>) {
        self.value = value;
    }
}

#[cfg(test)]
mod test {
    use core::ptr;

    use sync::ControlledModificationCell;

    use crate::graph::linked_list::{Link, LinkedList};

    #[derive(Debug)]
    struct Value<'a> {
        _value: usize,

        link: ControlledModificationCell<Link<'a, Value<'a>>>,
    }

    impl<'a> Value<'a> {
        const fn new(value: usize) -> Self {
            Self {
                _value: value,
                link: ControlledModificationCell::new(Link::empty()),
            }
        }
    }

    #[test]
    fn empty_list() {
        let mut list = LinkedList::<u8>::new();
        assert!(list.pop_front().is_none());
        assert_eq!(list.iter().next(), None);
    }

    #[test]
    fn single_value() {
        let value = Value::new(0);

        let mut list = LinkedList::new();
        list.push_back(&value.link);

        for link in list.link_iter() {
            assert_eq!(ptr::from_ref(link), ptr::from_ref(&value.link));
        }

        let popped = list.pop_front().unwrap();
        assert_eq!(ptr::from_ref(popped), ptr::from_ref(&value.link));

        assert!(list.pop_front().is_none());
    }

    #[test]
    fn multiple_values() {
        let value_0 = Value::new(0);
        let value_1 = Value::new(0);

        let values = [&value_0, &value_1];

        let mut list = LinkedList::new();
        list.push_back(&value_0.link);
        list.push_back(&value_1.link);

        for (link, value) in list.link_iter().zip(values) {
            assert_eq!(ptr::from_ref(link), ptr::from_ref(&value.link));
        }

        let popped_0 = list.pop_front().unwrap();
        assert_eq!(ptr::from_ref(popped_0), ptr::from_ref(&value_0.link));

        let popped_1 = list.pop_front().unwrap();
        assert_eq!(ptr::from_ref(popped_1), ptr::from_ref(&value_1.link));

        assert!(list.pop_front().is_none());
    }
}

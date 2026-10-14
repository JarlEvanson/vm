//! Implementation of the [`GraphNode`][gn] dependency [`Edge`]s.
//!
//! [gn]: super::GraphNode

use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

use sync::ControlledModificationCell;

use crate::graph::{linked_list::Link, node::Internal};

/// A dependency relationship.
#[derive(Debug)]
pub struct Edge<'nodes> {
    /// The [`Internal`] instance which has a dependency on the [`Internal`] instance in
    /// [`Edge::dependency`].
    dependent: &'nodes Internal<'nodes>,
    /// The [`Link`] that enables this [`Edge`] to be added to the [`LinkedList`][ll] in the
    /// [`Internal`] instance in [`Edge::dependent`].
    ///
    /// [ll]: super::super::linked_list::LinkedList
    dependent_link: ControlledModificationCell<Link<'nodes, Edge<'nodes>>>,

    /// The [`Internal`] instance on which the [`Internal`] instance in [`Edge::dependent`] has a
    /// dependency.
    dependency: &'nodes Internal<'nodes>,
    /// The [`Link`] that enables this [`Edge`] to be added to the [`LinkedList`][ll] in the
    /// [`Internal`] instance in [`Edge::dependency`].
    ///
    /// [ll]: super::super::linked_list::LinkedList
    dependency_link: ControlledModificationCell<Link<'nodes, Edge<'nodes>>>,

    /// If `true`, this [`Edge`] is enabled and thus the dependency relationship is considered in
    /// graph computations.
    enabled: AtomicBool,
}

impl<'nodes> Edge<'nodes> {
    /// Creates a new [`Edge`] with the provided `dependent`, `dependency`, and `enabled` status.
    pub const fn new(
        slot: &'nodes mut MaybeUninit<Edge<'nodes>>,
        dependent: &'nodes Internal<'nodes>,
        dependency: &'nodes Internal<'nodes>,
        enabled: bool,
    ) -> (
        &'nodes ControlledModificationCell<Link<'nodes, Edge<'nodes>>>,
        &'nodes ControlledModificationCell<Link<'nodes, Edge<'nodes>>>,
    ) {
        let slot = slot.write(Self {
            dependent,
            dependent_link: ControlledModificationCell::new(Link::empty()),
            dependency,
            dependency_link: ControlledModificationCell::new(Link::empty()),
            enabled: AtomicBool::new(enabled),
        });

        let slot = &*slot;

        // SAFETY:
        //
        // Since this function was provided with mutable access to this [`Edge`] slot, there cannot
        // be any mechanism to obtain another access concurrently. Therefore, there are no
        // overlapping references to [`Edge::dependent_link`].
        unsafe { slot.dependent_link.get_mut().set_value(Some(slot)) };
        // SAFETY:
        //
        // Since this function was provided with mutable access to this [`Edge`] slot, there cannot
        // be any mechanism to obtain another access concurrently. Therefore, there are no
        // overlapping references to [`Edge::dependent_link`].
        unsafe { slot.dependency_link.get_mut().set_value(Some(slot)) };
        (&slot.dependent_link, &slot.dependency_link)
    }

    /// Returns the [`Internal`] instance that has the dependency specified by this [`Edge`].
    pub const fn dependent(&self) -> &'nodes Internal<'nodes> {
        self.dependent
    }

    /// Returns the [`Internal`] instance that is the dependency specified by this [`Edge`].
    pub const fn dependency(&self) -> &'nodes Internal<'nodes> {
        self.dependency
    }

    /// Returns `true` if this [`Edge`] should be considered in graph computations.
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enables this [`Edge`] such that the dependency relationship is considered in graph
    /// computations.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disables this [`Edge`] such that the dependency relationship is not considered in graph
    /// computations.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}

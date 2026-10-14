//! Internal implementation of [`InternalRuntimeInit`].

use core::{mem::MaybeUninit, ptr};

use sync::ControlledModificationCell;

use crate::graph::{linked_list::LinkedList, node::{Internal, edge::Edge}};

/// Structure containing all of the runtime initialized portions of the [`Internal`] structure.
#[derive(Debug)]
pub struct InternalRuntimeInit<'nodes> {
    /// A [`LinkedList`] of all [`Edge`]s that define the [`GraphNode`]s that the associated
    /// [`GraphNode`] requires to be active before it is activated.
    required_list: LinkedList<'nodes, Edge<'nodes>>,
    /// The [`Edge`] slots that are used to store the finalized [`Edge`]s that are added to
    /// [`InternalRuntimeInit::required_list`] during initialization.
    required_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],

    /// A [`LinkedList`] of all [`Edge`]s that define the [`GraphNode`]s for which the associated
    /// [`GraphNode`] is a dependency.
    required_by_list: LinkedList<'nodes, Edge<'nodes>>,
    /// The [`Edge`] slots that are used to store the finalized [`Edge`]s that are added to
    /// [`InternalRuntimeInit::required_by_list`] during initialization.
    required_by_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],
}

impl<'nodes> InternalRuntimeInit<'nodes> {
    /// Creates a new instance of [`InternalRuntimeInit`] with the provided [`Edge`] slots.
    ///
    /// # Safety
    ///
    /// - After this call returns, the provided [`Edge`] slices must not be accessed in any manner
    ///   except through the [`InternalRuntimeInit`] instance that this call returns.
    pub const unsafe fn new(
        required_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],
        required_by_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],
    ) -> Self {
        Self {
            required_list: LinkedList::new(),
            required_by_list: LinkedList::new(),

            required_edges,
            required_by_edges,
        }
    }

    /// Initializes the [`InternalRuntimeInit`] instance contained within [`Internal`].
    ///
    /// # Safety
    ///
    /// - Accessing the [`InternalRuntimeInit`] instance of `this` in a mutable manner must be
    ///   valid for the duration of this call.
    /// - Accessing the [`InternalRuntimeInit`] instances of all [`Internal`] instances that are
    ///   directly related to `this` [`Internal`] instance in a mutable manner must be valid for
    ///   the duration of this call.
    unsafe fn initialize(this: &'nodes Internal<'nodes>) {
        // SAFETY:
        //
        // According to the invariants of this function, mutable access to `this`'s
        // [`InternalRuntimeInit`] instance is valid for the duration of this call.
        let runtime_init = unsafe { this.runtime_init.get_mut() };
        
        for (&required, edge) in this
            .required_enabled
            .iter()
            .zip(runtime_init.required_edges.iter())
        {
            assert_ne!(
                ptr::from_ref(this),
                ptr::from_ref(required),
                "a node must not be required by itthis"
            );

            // SAFETY:
            //
            // Since `this` and `required` are not equal, their [`InternalRuntimeInit`] instances
            // cannot overlap. According to the invariants of this function, accessing the
            // [`InternalRuntimeInit`] instances of directly related [`Internal`] instances (which
            // `required` is, given that it has a `required` relationship) in a mutable manner is
            // valid.
            unsafe {
                Self::initialize_edge(this, required, runtime_init, edge, true, true);
            }
        }

        for (&required, edge) in this
            .required_by_enabled
            .iter()
            .zip(runtime_init.required_by_edges.iter())
        {
            assert_ne!(
                ptr::from_ref(this),
                ptr::from_ref(required),
                "a node must not be required by itthis"
            );

            // SAFETY:
            //
            // Since `this` and `required_by` are not equal, their [`InternalRuntimeInit`]
            // instances cannot overlap. According to the invariants of this function, accessing
            // the [`InternalRuntimeInit`] instances of directly related [`Internal`] instances
            // (which `required_by` is, given that it has a `required_by` relationship) in a
            // mutable manner is valid.
            unsafe {
                Self::initialize_edge(this, required, runtime_init, edge, true, false);
            }
        }

        for (&required, edge) in this
            .required_disabled
            .iter()
            .zip(runtime_init.required_edges.iter())
        {
            assert_ne!(
                ptr::from_ref(this),
                ptr::from_ref(required),
                "a node must not be required by itthis"
            );

            // SAFETY:
            //
            // Since `this` and `required` are not equal, their [`InternalRuntimeInit`] instances
            // cannot overlap. According to the invariants of this function, accessing the
            // [`InternalRuntimeInit`] instances of directly related [`Internal`] instances (which
            // `required` is, given that it has a `required` relationship) in a mutable manner is
            // valid.
            unsafe {
                Self::initialize_edge(this, required, runtime_init, edge, false, true);
            }
        }

        for (&required, edge) in this
            .required_by_disabled
            .iter()
            .zip(runtime_init.required_by_edges.iter())
        {
            assert_ne!(
                ptr::from_ref(this),
                ptr::from_ref(required),
                "a node must not be required by itthis"
            );

            // SAFETY:
            //
            // Since `this` and `required_by` are not equal, their [`InternalRuntimeInit`]
            // instances cannot overlap. According to the invariants of this function, accessing
            // the [`InternalRuntimeInit`] instances of directly related [`Internal`] instances
            // (which `required_by` is, given that it has a `required_by` relationship) in a
            // mutable manner is valid.
            unsafe {
                Self::initialize_edge(this, required, runtime_init, edge, false, false);
            }
        }
    }

    /// Initializes the provided [`Edge`] with the two [`Internal`] instances, setting the `enabled`
    /// state according to the corresponding argument.
    ///
    /// # Safety
    ///
    /// - Accessing the [`InternalRuntimeInit`] field of `other` in a mutable manner must be valid
    ///   for the duration of this call.
    /// - `runtime_init` must correspond to the [`InternalRuntimeInit`] field of `this`.
    /// - `edge` must be an instance of [`Edge`] from the provided [`InternalRuntimeInit`] instance.
    unsafe fn initialize_edge(
        this: &'nodes Internal<'nodes>,
        other: &'nodes Internal<'nodes>,
        runtime_init: &mut InternalRuntimeInit<'nodes>,
        edge: &'nodes ControlledModificationCell<MaybeUninit<Edge<'nodes>>>,
        enabled: bool,
        required: bool,
    ) {
        let (dependent, dependency) = if required {
            (this, other)
        } else {
            (other, this)
        };

        // SAFETY:
        //
        // [`Internal`] instances cannot be created without the provided [`InternalRuntimeInit`]
        // [`Edge`] slices being inaccessible to the remainder of the program except through the
        // newly created [`Internal`] instance. Since this function can safely access
        // [`InternalRuntimeInit`] mutably and the only way to access the [`Edge`] slices is
        // through this [`InternalRuntimeInit`], it must be safe to access this edge mutably.
        let edge_mut = unsafe { edge.get_mut() };
        let (dependent_link, dependency_link) = Edge::new(edge_mut, dependent, dependency, enabled);

        // SAFETY:
        //
        // According to the invariants of this function, this operation must be safe.
        let other_runtime_init = unsafe { other.runtime_init.get_mut() };

        if required {
            runtime_init.required_list.push_back(dependent_link);
            other_runtime_init
                .required_by_list
                .push_back(dependency_link);
        } else {
            runtime_init.required_by_list.push_back(dependency_link);
            other_runtime_init.required_list.push_back(dependent_link);
        }
    }
}

//! Implementation of the nodes in a graph-based initialization subsystem.

use core::{
    mem::MaybeUninit,
    ops, ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

use sync::ControlledModificationCell;

use crate::graph::{
    linked_list::LinkedList,
    node::{data::Data, edge::Edge, runtime_init::InternalRuntimeInit},
};

mod data;
mod edge;
mod runtime_init;

/// A node in the initialization graph. [`GraphNode`]s are primarily used either to aggregate a
/// series of [`GraphNode`]s together in order to simplify dependency specifications or to perform
/// actions when activation or deactivating a given [`GraphNode`]. An activation function can return
/// a generic object that is stored in the [`GraphNode`] and is reference counted to provide access
/// to said object until the [`GraphNode`] is deactivated.
pub struct GraphNode<'nodes, T> {
    /// The activation function.
    activation_func: Option<ActivationFunc<T>>,
    /// The deactivation function.
    deactivation_func: Option<DeactivationFunc<T>>,

    /// The statically available instance of `T`.
    data: Data<T>,

    internal: Internal<'nodes>,
}

/// The protype required for activation functions.
type ActivationFunc<T> = fn() -> Result<T, ()>;
/// The protype required for deactivation functions.
type DeactivationFunc<T> = fn(T) -> Result<(), T>;

#[derive(Debug)]
pub struct Internal<'nodes> {
    /// The [`Internal`] nodes that the associated [`GraphNode`] requires and whose relationship is
    /// enabled at initialization time.
    required_enabled: &'nodes [&'nodes Internal<'nodes>],
    /// The [`Internal`] nodes that the associated [`GraphNode`] is a dependency of and whose
    /// relationship is enabled at initialization time.
    required_by_enabled: &'nodes [&'nodes Internal<'nodes>],

    /// The [`Internal`] nodes that the associated [`GraphNode`] requires and whose relationship is
    /// disabled at initialization time.
    required_disabled: &'nodes [&'nodes Internal<'nodes>],
    /// The [`Internal`] nodes that the associated [`GraphNode`] is a dependency of and whose
    /// relationship is disabled at initialization time.
    required_by_disabled: &'nodes [&'nodes Internal<'nodes>],

    /// Storage for data initialized at runtime.
    runtime_init: ControlledModificationCell<InternalRuntimeInit<'nodes>>,
}

impl<'nodes> Internal<'nodes> {
    unsafe fn new(
        required_enabled: &'nodes [&'nodes Internal<'nodes>],
        required_by_enabled: &'nodes [&'nodes Internal<'nodes>],
        required_disabled: &'nodes [&'nodes Internal<'nodes>],
        required_by_disabled: &'nodes [&'nodes Internal<'nodes>],
        required_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],
        required_by_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>],
    ) -> Self {
        // SAFETY:
        //
        // According to the invariants of this function, the provided [`Edge`] slices will not be
        // access in any manner except through the [`InternalRuntimeInit`] instance that the
        // [`InternalRuntimeInit::new()`] call returns after this function returns. Since this
        // function doesn't access `required_edges` and `required_by_edges` after this call, the
        // invariants of [`InternalRuntimeInit::new()`] are fulfilled.
        let runtime_init = unsafe { InternalRuntimeInit::new(required_edges, required_by_edges) };

        Self {
            required_enabled,
            required_by_enabled,

            required_disabled,
            required_by_disabled,

            runtime_init: ControlledModificationCell::new(runtime_init),
        }
    }
}

/// The execution state of an [`GraphNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::graph) enum State {
    /// The [`GraphNode`] is disabled and thus is not considered in graph calculations.
    Disabled,
    /// The [`GraphNode`] is enabled and thus is considered in graph calculations.
    Enabled,

    /// The [`GraphNode`] is waiting to be activated in the current operation.
    WaitingForActivation,

    /// The [`GraphNode`] is currently active.
    Active,

    /// The [`GraphNode`]'s startup failed.
    Failed,
}

impl State {
    /// Converts the provided [`u8`] into its corresponding [`State`].
    const fn from_u8(state: u8) -> Self {
        match state {
            0 => Self::Disabled,
            1 => Self::Enabled,
            2 => Self::WaitingForActivation,
            3 => Self::Active,
            4 => Self::Failed,
            _ => unreachable!(),
        }
    }

    /// Converts the provided [`State`] into its corresponding [`u8`].
    const fn to_u8(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Enabled => 1,
            Self::WaitingForActivation => 2,
            Self::Active => 3,
            Self::Failed => 4,
        }
    }
}

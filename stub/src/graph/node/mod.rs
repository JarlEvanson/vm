//! Implementation of the nodes in a graph-based initialization subsystem.

use crate::graph::node::data::Data;

mod data;

/// The protype required for activation functions.
type ActivationFunc<T> = fn() -> Result<T, ()>;

/// The protype required for deactivation functions.
type DeactivationFunc<T> = fn(T) -> Result<(), T>;

/// A node in the initialization graph. [`GraphNode`]s are primarily used either to aggregate a
/// series of [`GraphNode`]s together in order to simplify dependency specifications or to perform
/// actions when activating or deactivating a given [`GraphNode`]. An activation can return a `T`
/// instance that is stored in the [`GraphNode`] and is reference counted to provide a safe
/// interface to access the `T` instance until the [`GraphNode`] is deactivated.
#[derive(Debug)]
pub struct GraphNode<T> {
    /// The activation function.
    activation_func: Option<ActivationFunc<T>>,

    /// The deactivation function.
    deactivation_func: Option<DeactivationFunc<T>>,

    /// The statically available instance of `T`.
    data: Data<T>,
}

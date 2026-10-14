//! Implementation of the nodes in a graph-based initialization subsystem.

use core::{
    mem::{self, MaybeUninit},
    ptr::NonNull,
    sync::atomic::{AtomicU8, Ordering},
};

use sync::ControlledModificationCell;

use crate::graph::{
    linked_list::Link,
    node::{data::Data, runtime_init::InternalRuntimeInit},
};

mod data;
mod edge;
mod runtime_init;

pub use edge::Edge;
pub use runtime_init::{DependencyIter, DependentIter};

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

    /// The untyped data required for execution.
    internal: Internal<'nodes>,
}

impl<'nodes, T> GraphNode<'nodes, T> {
    /// Creates a new [`GraphNode`] instance.
    ///
    /// # Safety
    ///
    /// After this call returns, the provided [`Edge`] arrays must not be accessed in any manner
    /// except through the interfaces provided by the [`GraphNode`] instance that this call
    /// returned.
    #[expect(clippy::too_many_arguments)]
    pub const unsafe fn new<
        const RE: usize,
        const RD: usize,
        const RBE: usize,
        const RBD: usize,
        const TR: usize,
        const TRB: usize,
    >(
        name: &'nodes str,
        life_cycle_funcs: Option<(ActivationFunc<T>, Option<DeactivationFunc<T>>)>,
        enabled: bool,
        required_enabled: &'nodes [&'nodes Internal<'nodes>; RE],
        required_disabled: &'nodes [&'nodes Internal<'nodes>; RD],
        required_by_enabled: &'nodes [&'nodes Internal<'nodes>; RBE],
        required_by_disabled: &'nodes [&'nodes Internal<'nodes>; RBD],
        required_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>; TR],
        required_by_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>; TRB],
    ) -> Self {
        let (activation_func, deactivation_func) =
            if let Some((activation_func, deactivation_func)) = life_cycle_funcs {
                (Some(activation_func), deactivation_func)
            } else {
                (None, None)
            };

        Self {
            activation_func,
            deactivation_func,
            data: Data::new(),
            // SAFETY:
            //
            // According to the invariants of this function, the provided [`Edge`] arrays will not
            // be accessed in any manner except through the interfaces provided by the [`Internal`]
            // instance that this [`GraphNode`] will contain.
            internal: unsafe {
                Internal::new::<T, _, _, _, _, _, _>(
                    name,
                    enabled,
                    required_enabled,
                    required_disabled,
                    required_by_enabled,
                    required_by_disabled,
                    required_edges,
                    required_by_edges,
                )
            },
        }
    }

    /// Returns an immutable reference to the contained [`Internal`].
    #[doc(hidden)]
    pub(crate) const fn internal(&self) -> &Internal<'nodes> {
        &self.internal
    }
}

/// The protype required for activation functions.
type ActivationFunc<T> = fn() -> Result<T, ()>;
/// The protype required for deactivation functions.
type DeactivationFunc<T> = fn(T) -> Result<(), T>;

/// The untyped data for [`GraphNode`].
#[derive(Debug)]
pub struct Internal<'nodes> {
    /// The name of the [`GraphNode`].
    ///
    /// This is not unique.
    name: &'nodes str,

    /// The state of this [`GraphNode`].
    state: AtomicU8,

    /// The offset to the associated [`GraphNode`].
    node_offset: usize,

    /// The untyped startup function.
    activation_func: unsafe fn(node_ptr: NonNull<()>) -> Result<(), ()>,
    /// The untyped shutdown function.
    deactivation_func: unsafe fn(node_ptr: NonNull<()>) -> Result<(), ()>,

    /// An intrusive link used for computing the execution graph.
    link: ControlledModificationCell<Link<'nodes, Internal<'nodes>>>,

    /// The [`Internal`] nodes that the associated [`GraphNode`] requires and whose relationship is
    /// enabled at initialization time.
    required_enabled: &'nodes [&'nodes Internal<'nodes>],
    /// The [`Internal`] nodes that the associated [`GraphNode`] requires and whose relationship is
    /// disabled at initialization time.
    required_disabled: &'nodes [&'nodes Internal<'nodes>],

    /// The [`Internal`] nodes that the associated [`GraphNode`] is a dependency of and whose
    /// relationship is enabled at initialization time.
    required_by_enabled: &'nodes [&'nodes Internal<'nodes>],
    /// The [`Internal`] nodes that the associated [`GraphNode`] is a dependency of and whose
    /// relationship is disabled at initialization time.
    required_by_disabled: &'nodes [&'nodes Internal<'nodes>],

    /// Storage for data initialized at runtime.
    runtime_init: ControlledModificationCell<InternalRuntimeInit<'nodes>>,
}

impl<'nodes> Internal<'nodes> {
    /// Creates a new [`Internal`] instance.
    ///
    /// # Safety
    ///
    /// After this call returns, the provided [`Edge`] arrays must not be accessed in any manner
    /// except through the interfaces provided by the [`Internal`] instance that this call
    /// returned.
    #[expect(clippy::too_many_arguments)]
    const unsafe fn new<
        T,
        const RE: usize,
        const RD: usize,
        const RBE: usize,
        const RBD: usize,
        const TR: usize,
        const TRB: usize,
    >(
        name: &'nodes str,
        enabled: bool,
        required_enabled: &'nodes [&'nodes Internal<'nodes>; RE],
        required_disabled: &'nodes [&'nodes Internal<'nodes>; RD],
        required_by_enabled: &'nodes [&'nodes Internal<'nodes>; RBE],
        required_by_disabled: &'nodes [&'nodes Internal<'nodes>; RBD],
        required_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>; TR],
        required_by_edges: &'nodes [ControlledModificationCell<MaybeUninit<Edge<'nodes>>>; TRB],
    ) -> Self {
        assert!(
            RE + RD == TR,
            "required enabled and required disabled must equal total required edges"
        );
        assert!(
            RE + RD == TR,
            "required by enabled and required by disabled must equal total required by edges"
        );

        // SAFETY:
        //
        // According to the invariants of this function, the provided [`Edge`] slices will not be
        // access in any manner except through the [`InternalRuntimeInit`] instance that the
        // [`InternalRuntimeInit::new()`] call returns after this function returns. Since this
        // function doesn't access `required_edges` and `required_by_edges` after this call, the
        // invariants of [`InternalRuntimeInit::new()`] are fulfilled.
        let runtime_init = unsafe { InternalRuntimeInit::new(required_edges, required_by_edges) };

        let state = if enabled {
            State::Enabled
        } else {
            State::Disabled
        };

        Self {
            name,

            state: AtomicU8::new(state.to_u8()),
            link: ControlledModificationCell::new(Link::empty()),

            node_offset: mem::offset_of!(GraphNode<T>, internal),
            activation_func: Self::activation_func::<T>,
            deactivation_func: Self::deactivation_func::<T>,

            required_enabled,
            required_disabled,

            required_by_enabled,
            required_by_disabled,

            runtime_init: ControlledModificationCell::new(runtime_init),
        }
    }

    /// Initializes the runtime portions of [`Internal`].
    ///
    /// # Safety
    ///
    /// For the duration of this call, there must not be any calls to any functions provided by
    /// [`Internal`] on `self` or any [`Internal`] structures directly references by [`Internal`].
    pub(super) unsafe fn initialize(&'nodes self) {
        // SAFETY:
        //
        // The invariants of [`Internal::initialize()`] fulfill the invariants of
        // [`InternalRuntimeInit::initialize()`].
        unsafe { InternalRuntimeInit::initialize(self) }
    }

    /// Returns the name of the [`Internal`] instance.
    ///
    /// This is not unique.
    pub(super) fn name(&self) -> &'nodes str {
        self.name
    }

    /// Returns the [`State`] of this [`GraphNode`].
    pub(super) fn state(&self) -> State {
        State::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Sets the [`State`] of this [`GraphNode`].
    pub(super) fn set_state(&self, state: State) {
        self.state.store(state.to_u8(), Ordering::Relaxed);
    }

    /// Returns an [`Iterator`] over all [`Internal`] instances that are enabled dependents of
    /// this [`Internal`] instance.
    pub(super) fn dependents(&self) -> DependentIter<'nodes> {
        self.runtime_init.get().dependents()
    }

    /// Returns an [`Iterator`] over all [`Internal`] instances that are enabled dependencies of
    /// this [`Internal`] instance.
    pub(super) fn dependencies(&self) -> DependencyIter<'nodes> {
        self.runtime_init.get().dependencies()
    }

    /// Activates the [`GraphNode`] holding this [`Internal`] instance.
    ///
    /// # Safety
    ///
    /// This [`Internal`] instance must be placed within a [`GraphNode`] of the same type `T` that
    /// was specified when it was created.
    pub(super) unsafe fn activate(&self) -> Result<(), ()> {
        let internal_ptr = NonNull::from_ref(self);
        // SAFETY:
        //
        // This invariants of this function ensure that this operation is safe.
        let node_ptr = unsafe { internal_ptr.byte_sub(self.node_offset).cast::<()>() };

        // SAFETY:
        //
        // The invariants of [`Internal::new()`] mean that `node_ptr` refers to the address of a
        // [`GraphNode`].
        unsafe { (self.activation_func)(node_ptr) }
    }

    /// Deactivates the [`GraphNode`] holding this [`Internal`] instance.
    ///
    /// # Safety
    ///
    /// This [`Internal`] instance must be placed within a [`GraphNode`] of the same type `T` that
    /// was specified when it was created.
    pub(super) unsafe fn deactivate(&self) -> Result<(), ()> {
        let internal_ptr = NonNull::from_ref(self);
        // SAFETY:
        //
        // This invariants of this function ensure that this operation is safe.
        let node_ptr = unsafe { internal_ptr.byte_sub(self.node_offset).cast::<()>() };

        // SAFETY:
        //
        // The invariants of [`Internal::new()`] mean that `node_ptr` refers to the address of a
        // [`GraphNode`].
        unsafe { (self.deactivation_func)(node_ptr) }
    }

    /// The generic untyped activation function calling interface.
    ///
    /// # Safety
    ///
    /// `node_ptr` must be the address of a [`GraphNode<T>`] that is safe to immutably reference.
    unsafe fn activation_func<T>(node_ptr: NonNull<()>) -> Result<(), ()> {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        let node = unsafe { node_ptr.cast::<GraphNode<T>>().as_ref() };
        let Some(activation_func) = node.activation_func else {
            return Ok(());
        };

        activation_func().map(|data| node.data.initialize(data))
    }

    /// The generic untyped deactivation function calling interface.
    ///
    /// # Safety
    ///
    /// `node_ptr` must be the address of a [`GraphNode<T>`] that is safe to immutably reference.
    unsafe fn deactivation_func<T>(node_ptr: NonNull<()>) -> Result<(), ()> {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        let node = unsafe { node_ptr.cast::<GraphNode<T>>().as_ref() };
        if node.activation_func.is_none() {
            return Ok(());
        }

        let data = node.data.take();
        let Some(deactivation_func) = node.deactivation_func else {
            return Ok(());
        };

        deactivation_func(data).map_err(|data| node.data.initialize(data))
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

/// Creates a new [`GraphNode`].
#[doc(hidden)]
#[macro_export]
macro_rules! make_node {
    (
        $name:expr,
        $life_cycle_funcs:expr,
        active = $active:expr,
        required_enabled = [$($required_enabled_item:expr),* $(,)?],
        required_disabled = [$($required_disabled_item:expr),* $(,)?],
        required_by_enabled = [$($required_by_enabled_item:expr),* $(,)?],
        required_by_disabled = [$($required_by_disabled_item:expr),* $(,)?],
    ) => {{
        {
            const REQUIRED_ENABLED_COUNT: usize =
                <[()]>::len(&[ $( { let _ = &$required_enabled_item; () }),*]);
            const REQUIRED_BY_ENABLED_COUNT: usize =
                <[()]>::len(&[ $( { let _ = &$required_by_enabled_item; () }),*]);
            const REQUIRED_DISABLED_COUNT: usize =
                <[()]>::len(&[ $( { let _ = &$required_disabled_item; () }),*]);
            const REQUIRED_BY_DISABLED_COUNT: usize =
                <[()]>::len(&[ $( { let _ = &$required_by_disabled_item; () }),*]);
            const REQUIRED_COUNT: usize = REQUIRED_ENABLED_COUNT + REQUIRED_DISABLED_COUNT;
            const REQUIRED_BY_COUNT: usize =
                REQUIRED_BY_ENABLED_COUNT + REQUIRED_BY_DISABLED_COUNT;

            static REQUIRED_ENABLED: [&$crate::graph::Internal; REQUIRED_ENABLED_COUNT] =
                [$( &$required_enabled_item.internal() ),*];
            static REQUIRED_DISABLED: [&$crate::graph::Internal; REQUIRED_DISABLED_COUNT] =
                [$( &$required_disabled_item.internal() ),*];
            static REQUIRED_BY_ENABLED: [&$crate::graph::Internal; REQUIRED_BY_ENABLED_COUNT] =
                [$( &$required_by_enabled_item.internal() ),*];
            static REQUIRED_BY_DISABLED: [&$crate::graph::Internal; REQUIRED_BY_DISABLED_COUNT] =
                [$( &$required_by_disabled_item.internal() ),*];

            static REQUIRED_EDGES: [::sync::ControlledModificationCell<
                ::core::mem::MaybeUninit::<$crate::graph::Edge>
            >; REQUIRED_COUNT] = [const {
                ::sync::ControlledModificationCell::new(::core::mem::MaybeUninit::uninit())
            }; REQUIRED_COUNT];
            static REQUIRED_BY_EDGES: [::sync::ControlledModificationCell<
                ::core::mem::MaybeUninit::<$crate::graph::Edge>
                >;
                REQUIRED_BY_COUNT] = [const {
                ::sync::ControlledModificationCell::new(::core::mem::MaybeUninit::uninit())
            }; REQUIRED_BY_COUNT];

            // SAFETY:
            //
            // After this call returns, the provided [`Edge`] arrays will not be able to be named
            // since they are inside a static definitions.
            unsafe {
                $crate::graph::GraphNode::new(
                    $name,
                    $life_cycle_funcs,
                    $active,
                    &REQUIRED_ENABLED,
                    &REQUIRED_DISABLED,
                    &REQUIRED_BY_ENABLED,
                    &REQUIRED_BY_DISABLED,
                    &REQUIRED_EDGES,
                    &REQUIRED_BY_EDGES,
                )
            }
        }
    }};
}

#[cfg(test)]
mod test {
    use core::ptr::NonNull;

    use super::State;

    #[test]
    fn state_roundtrips() {
        let states = [
            State::Disabled,
            State::Enabled,
            State::WaitingForActivation,
            State::Active,
            State::Failed,
        ];
        for state in states {
            assert_eq!(state, State::from_u8(state.to_u8()));
        }
    }
}

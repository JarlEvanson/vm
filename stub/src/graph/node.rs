use core::{
    mem::MaybeUninit,
    ops,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

use sync::ControlledModificationCell;

use crate::graph::linked_list::{Link, LinkIter, LinkedList};

pub struct GraphNode<'nodes, T> {
    startup_func: Option<StartupFunc<T>>,
    shutdown_func: Option<ShutdownFunc<T>>,

    data: NodeData<T>,

    #[doc(hidden)]
    pub(crate) internal: Internal<'nodes>,
}

impl<'nodes, T: Send + Sync> GraphNode<'nodes, T> {
    #[doc(hidden)]
    #[expect(clippy::too_many_arguments)]
    pub const fn new<const R: usize, const RB: usize, const W: usize, const WB: usize>(
        name: &'static str,
        life_cycle_funcs: Option<(StartupFunc<T>, Option<ShutdownFunc<T>>)>,
        enabled: bool,
        required: &'nodes [&'nodes Internal<'nodes>; R],
        required_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; R],
        required_by: &'nodes [&'nodes Internal<'nodes>; RB],
        required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; RB],
        wanted: &'nodes [&'nodes Internal<'nodes>; W],
        wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        wanted_by: &'nodes [&'nodes Internal<'nodes>; WB],
        wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; WB],
    ) -> Self {
        let (startup, shutdown) = if let Some((startup, shutdown_opt)) = life_cycle_funcs {
            (Some(startup), shutdown_opt)
        } else {
            (None, None)
        };

        Self {
            startup_func: startup,
            shutdown_func: shutdown,

            data: NodeData {
                references: AtomicUsize::new(0),
                data: ControlledModificationCell::new(MaybeUninit::uninit()),
            },

            internal: Internal::new(
                name,
                enabled,
                required,
                required_links,
                required_by,
                required_by_links,
                wanted,
                wanted_links,
                wanted_by,
                wanted_by_links,
            ),
        }
    }
}

impl<'nodes, T> GraphNode<'nodes, T> {
    /// Takes a [`Reference`] to the data associated with this [`GraphNode`].
    pub fn ref_data<'a>(&'a self) -> Option<Reference<'a, T>> {
        let mut current = self.data.references.load(Ordering::Relaxed);
        loop {
            if current == 0 {
                return None;
            }

            let Some(new) = current.checked_add(1) else {
                panic!("too many references to node data");
            };
            let result = self.data.references.compare_exchange_weak(
                current,
                new,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
            match result {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }

        Some(Reference { ptr: &self.data })
    }
}

pub struct Reference<'node, T> {
    ptr: &'node NodeData<T>,
}

impl<'node, T> Clone for Reference<'node, T> {
    fn clone(&self) -> Self {
        let mut current = self.ptr.references.load(Ordering::Relaxed);
        loop {
            let Some(new) = current.checked_add(1) else {
                panic!("too many references to node data");
            };
            let result = self.ptr.references.compare_exchange_weak(
                current,
                new,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
            match result {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }

        Self { ptr: self.ptr }
    }
}

impl<'node, T> ops::Deref for Reference<'node, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        //
        // The presence of a [`Reference`] indicates that the associated data is initialized and
        // valid to reference.
        unsafe { self.ptr.data.get().assume_init_ref() }
    }
}

impl<T> Drop for Reference<'_, T> {
    fn drop(&mut self) {
        let previous_references = self.ptr.references.fetch_sub(1, Ordering::Release);

        // [`Reference`] instances should only exist while `references` is greater than 1, since a
        // value of 0 indicates that the data is uninitialized, while a value of 1 indicates that
        // it is initialized and unreferenced.
        debug_assert!(previous_references <= 1);
    }
}

struct NodeData<T> {
    pub(in crate::graph) references: AtomicUsize,
    pub(in crate::graph) data: ControlledModificationCell<MaybeUninit<T>>,
}

type StartupFunc<T> = fn() -> Result<T, ()>;
type ShutdownFunc<T> = fn(T) -> Result<(), T>;

pub(crate) struct Internal<'nodes> {
    /// The name of the [`GraphNode`].
    ///
    /// This is not unique.
    pub(in crate::graph) name: &'static str,

    /// The state of this [`GraphNode`].
    pub(in crate::graph) state: AtomicU8,

    /// An intrusive link used for computing the execution graph.
    pub(in crate::graph) link: ControlledModificationCell<Link<'nodes, Internal<'nodes>>>,

    /// [`GraphNode`]s that this [`GraphNode`] requires to successfully execute before it executes
    /// and thus adds to the graph calculations when this [`GraphNode`] is considered for
    /// execution.
    ///
    /// [`GraphNode`]s in this list are always executed before this [`GraphNode`].
    pub(in crate::graph) required: &'nodes [&'nodes Internal<'nodes>],
    /// [`GraphNode`]s for which this [`GraphNode`] is required (used to enable decentralized
    /// dependencies).
    pub(in crate::graph) required_by: &'nodes [&'nodes Internal<'nodes>],

    /// [`GraphNode`]s that this [`GraphNode`] wants to execute before it executes and thus adds to
    /// the graph calculations when this [`GraphNode`] is considered for execution and the wanted
    /// [`GraphNode`] is enabled.
    ///
    /// [`GraphNode`]s in this list are executed before this [`GraphNode`] if added to the execution
    /// graph and no cycle forms that involves the two [`GraphNode`]s. If a cycle forms, this wanted
    /// relationship is relaxed to break the cycle. Furthermore, the execution of this [`GraphNode`]
    /// will proceed even if wanted [`GraphNode`]s fail during execution.
    pub(in crate::graph) wanted: &'nodes [&'nodes Internal<'nodes>],
    /// [`GraphNode`]s for which this [`GraphNode`] is wanted (used to enable decentralized
    /// dependencies).
    pub(in crate::graph) wanted_by: &'nodes [&'nodes Internal<'nodes>],

    /// Storage for runtime initialized data (including the final required/wanted lists computed
    /// by adding the reverse dependencies to the target node's required/wanted lists).
    pub(in crate::graph) runtime_init: ControlledModificationCell<InternalRuntimeInit<'nodes>>,
}

impl<'nodes> Internal<'nodes> {
    #[doc(hidden)]
    #[expect(clippy::too_many_arguments)]
    pub const fn new<const R: usize, const RB: usize, const W: usize, const WB: usize>(
        name: &'static str,
        enabled: bool,
        required: &'nodes [&'nodes Internal<'nodes>; R],
        required_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; R],
        required_by: &'nodes [&'nodes Internal<'nodes>; RB],
        required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; RB],
        wanted: &'nodes [&'nodes Internal<'nodes>; W],
        wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        wanted_by: &'nodes [&'nodes Internal<'nodes>; WB],
        wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; WB],
    ) -> Self {
        let state_value = if enabled {
            State::Enabled
        } else {
            State::Disabled
        };

        Self {
            name,

            state: AtomicU8::new(state_value.to_u8()),

            link: ControlledModificationCell::new(Link::empty()),

            required,
            required_by,

            wanted,
            wanted_by,

            runtime_init: ControlledModificationCell::new(InternalRuntimeInit {
                required_list: LinkedList::new(),
                wanted_list: LinkedList::new(),
                required_links,
                required_by_links,
                wanted_links,
                wanted_by_links,
            }),
        }
    }

    /// Returns the [`State`] of this [`GraphNode`].
    pub(in crate::graph) fn state(&self) -> State {
        State::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Sets the [`State`] of this [`GraphNode`].
    pub(in crate::graph) fn set_state(&self, state: State) {
        self.state.store(state.to_u8(), Ordering::Relaxed);
    }

    /// Returns an [`Iterator`] over the required [`GraphNode`]s for this [`GraphNode`].
    pub(in crate::graph) fn required(&self) -> LinkIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().required_list.iter()
    }

    /// Returns an [`Iterator`] over the wanted [`GraphNode`]s for this [`GraphNode`].
    pub(in crate::graph) fn wanted(&self) -> LinkIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().wanted_list.iter()
    }
}

pub(in crate::graph) struct InternalRuntimeInit<'nodes> {
    /// List of all nodes that this [`GraphNode`] requires.
    pub(in crate::graph) required_list: LinkedList<'nodes, Internal<'nodes>>,
    /// List of all nodes that this [`GraphNode`] wants.
    pub(in crate::graph) wanted_list: LinkedList<'nodes, Internal<'nodes>>,

    // Storage for [`Link`]s used to assemble the [`InternalRuntimeInit::required_list`] and
    // [`InternalRuntimeInit::wanted_list`] associated with each [`GraphNode`].
    pub(in crate::graph) required_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    pub(in crate::graph) required_by_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],

    pub(in crate::graph) wanted_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    pub(in crate::graph) wanted_by_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
}

/// The execution state of an [`GraphNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::graph) enum State {
    /// The [`GraphNode`] is disabled and thus is not considered in graph calculations.
    Disabled,
    /// The [`GraphNode`] is enabled and thus is considered in graph calculations.
    Enabled,

    /// The [`GraphNode`] is waiting for processing during the current graph operation.
    WaitingForProcessing,

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
            2 => Self::WaitingForProcessing,
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
            Self::WaitingForProcessing => 2,
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
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {{
        {
            static REQUIRED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; const {
                <[$crate::graph::node::Internal]>::len(&$required)
            }] = [const {
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty())
            }; <[$crate::graph::node::Internal]>::len(&$required)];
            static REQUIRED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; const {
                <[$crate::graph::node::Internal]>::len(&$required_by)
            }] = [const {
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty())
            }; <[$crate::graph::node::Internal]>::len(&$required_by)];
            static WANTED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; const {
                <[$crate::graph::node::Internal]>::len(&$wanted)
            }] = [const {
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty())
            }; <[$crate::graph::node::Internal]>::len(&$wanted)];
            static WANTED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; const {
                <[$crate::graph::node::Internal]>::len(&$wanted_by)
            }] = [const {
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty())
            }; <[$crate::graph::node::Internal]>::len(&$wanted_by)];

            $crate::graph::node::GraphNode::new(
                $name,
                $life_cycle_funcs,
                $active,
                &$required,
                &REQUIRED_LINKS,
                &$required_by,
                &REQUIRED_BY_LINKS,
                &$wanted,
                &WANTED_LINKS,
                &$wanted_by,
                &WANTED_BY_LINKS,
            )
        }
    }};
}

/// Defines a new [`GraphNode`].
#[macro_export]
macro_rules! define_node {
    (
        $static_name:ident,
        $name:expr,
        life_cycle_funcs = $life_cycle_funcs:expr,
        active = $active:expr,
        type = $type:ty,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        static $static_name: $crate::graph::node::GraphNode<$type> = {
            #[used]
            #[unsafe(link_section = ".graph.internal")]
            static INTERNAL_REFERENCE: $crate::util::SendSyncPointer<
                $crate::graph::node::Internal
            > = $crate::util::SendSyncPointer(&raw const $static_name.internal);

            $crate::make_node!(
                $name,
                $life_cycle_funcs,
                active = $active,
                required = $required,
                required_by = $required_by,
                wanted = $wanted,
                wanted_by = $wanted_by,
            )
        };
    };
    (
        $static_name:ident,
        $name:expr,
        active,
        type = $type:ty,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = None,
            active = true,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        active,
        type = $type:ty,
        startup = $startup_func:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, None)),
            active = true,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        active,
        type = $type:ty,
        startup = $startup_func:expr,
        shutdown = $shutdown_func:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, Some($shutdown_func))),
            active = true,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        inactive,
        type = $type:ty,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = None,
            active = false,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        inactive,
        type = $type:ty,
        startup = $startup_func:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, None)),
            active = false,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        inactive,
        type = $type:ty,
        startup = $startup_func:expr,
        shutdown = $shutdown_func:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, Some($shutdown_func))),
            active = false,
            type = $type,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
}

#[cfg(test)]
mod test {
    use crate::graph::node::State;

    #[test]
    fn state_roundtrips() {
        let states = [
            State::Disabled,
            State::Enabled,
            State::WaitingForProcessing,
            State::Active,
            State::Failed,
        ];
        for state in states {
            assert_eq!(state, State::from_u8(state.to_u8()));
        }
    }
}

//! Definitions and interfaces providing a [`GraphNode`]-based initialization system.

use core::{
    mem::{self, MaybeUninit},
    ops,
    ptr::NonNull,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

use sync::ControlledModificationCell;

use crate::graph::linked_list::{Link, LinkValueIter, LinkedList};

/// A node in the initialization graph. It can provided startup and shutdown functionality, as well
/// as containing a statically available instance of `T` through the [`Reference`] API that is
/// controlled by the startup and shutdown functionality.
pub struct GraphNode<'nodes, T> {
    /// The startup function.
    startup_func: Option<StartupFunc<T>>,
    /// The shutdown function.
    shutdown_func: Option<ShutdownFunc<T>>,

    /// The interior data.
    data: NodeData<T>,

    /// Internal untyped interface.
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
        rev_required_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; R],
        required_by: &'nodes [&'nodes Internal<'nodes>; RB],
        required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; RB],
        rev_required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>;
                    RB],
        wanted: &'nodes [&'nodes Internal<'nodes>; W],
        wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        rev_wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        wanted_by: &'nodes [&'nodes Internal<'nodes>; WB],
        wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; WB],
        rev_wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>;
                    WB],
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

            internal: Internal::new::<R, RB, W, WB, T>(
                name,
                mem::offset_of!(Self, internal),
                enabled,
                required,
                required_links,
                rev_required_links,
                required_by,
                required_by_links,
                rev_required_by_links,
                wanted,
                wanted_links,
                rev_wanted_links,
                wanted_by,
                wanted_by_links,
                rev_wanted_by_links,
            ),
        }
    }
}

impl<'nodes, T> GraphNode<'nodes, T> {
    /// Takes a [`Reference`] to the data associated with this [`GraphNode`].
    #[expect(
        clippy::missing_panics_doc,
        reason = "panicking is extraordinarily unlikely"
    )]
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

/// A thread-safe reference counted accessor for [`GraphNode`] data.
pub struct Reference<'node, T> {
    /// The [`NodeData`] to which this [`Reference`] refers.
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
        debug_assert!(previous_references > 1);
    }
}

/// Basis on which the [`Reference`] implementation is implemented.
struct NodeData<T> {
    /// Atomically manipulated reference counter.
    pub(in crate::graph) references: AtomicUsize,
    /// The stored data.
    pub(in crate::graph) data: ControlledModificationCell<MaybeUninit<T>>,
}

/// The protype required for startup functions.
type StartupFunc<T> = fn() -> Result<T, ()>;
/// The protype required for shutdown functions.
type ShutdownFunc<T> = fn(T) -> Result<(), T>;

/// The untyped interior of [`GraphNode`].
#[doc(hidden)]
pub struct Internal<'nodes> {
    /// The name of the [`GraphNode`].
    ///
    /// This is not unique.
    pub(in crate::graph) name: &'static str,

    /// The offset to the associated [`GraphNode`].
    pub(in crate::graph) node_offset: usize,

    /// The untyped startup function.
    pub(in crate::graph) startup_func: fn(node_ptr: NonNull<()>) -> Result<(), ()>,
    /// The untyped shutdown function.
    pub(in crate::graph) shutdown_func: fn(node_ptr: NonNull<()>) -> Result<(), ()>,

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
    pub const fn new<const R: usize, const RB: usize, const W: usize, const WB: usize, T>(
        name: &'static str,
        node_offset: usize,
        enabled: bool,
        required: &'nodes [&'nodes Internal<'nodes>; R],
        required_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; R],
        rev_required_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; R],
        required_by: &'nodes [&'nodes Internal<'nodes>; RB],
        required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; RB],
        rev_required_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>;
                    RB],
        wanted: &'nodes [&'nodes Internal<'nodes>; W],
        wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        rev_wanted_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; W],
        wanted_by: &'nodes [&'nodes Internal<'nodes>; WB],
        wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>; WB],
        rev_wanted_by_links: &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>;
                    WB],
    ) -> Self {
        let state_value = if enabled {
            State::Enabled
        } else {
            State::Disabled
        };

        Self {
            name,

            state: AtomicU8::new(state_value.to_u8()),

            node_offset,
            startup_func: Self::startup_func::<T>,
            shutdown_func: Self::shutdown_func::<T>,

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
                rev_required_list: LinkedList::new(),
                rev_wanted_list: LinkedList::new(),
                rev_required_links,
                rev_required_by_links,
                rev_wanted_links,
                rev_wanted_by_links,
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
    pub(in crate::graph) fn required(&self) -> LinkValueIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().required_list.iter()
    }

    /// Returns an [`Iterator`] over the wanted [`GraphNode`]s for this [`GraphNode`].
    pub(in crate::graph) fn wanted(&self) -> LinkValueIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().wanted_list.iter()
    }

    /// Returns an [`Iterator`] over the required [`GraphNode`]s for this [`GraphNode`].
    pub(in crate::graph) fn rev_required(&self) -> LinkValueIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().rev_required_list.iter()
    }

    /// Returns an [`Iterator`] over the wanted [`GraphNode`]s for this [`GraphNode`].
    pub(in crate::graph) fn rev_wanted(&self) -> LinkValueIter<'nodes, Internal<'nodes>> {
        self.runtime_init.get().rev_wanted_list.iter()
    }

    /// Calls the startup function associated with the associated [`GraphNode`].
    pub(in crate::graph) fn startup(&self) -> Result<(), ()> {
        let internal_ptr = NonNull::from_ref(self);
        // SAFETY:
        //
        // This is an interal function to the graph subsystem and thus all invocations can be
        // assumed to be safe.
        let graph_node_ptr = unsafe { internal_ptr.byte_sub(self.node_offset).cast::<()>() };

        (self.startup_func)(graph_node_ptr)
    }

    fn startup_func<T>(node_ptr: NonNull<()>) -> Result<(), ()> {
        let node = unsafe { node_ptr.cast::<GraphNode<T>>().as_ref() };
        let Some(startup_func) = node.startup_func else {
            return Ok(());
        };

        let data = startup_func()?;
        unsafe { node.data.data.get_mut().write(data) };
        node.data.references.store(1, Ordering::Release);
        Ok(())
    }

    fn shutdown_func<T>(node_ptr: NonNull<()>) -> Result<(), ()> {
        let node = unsafe { node_ptr.cast::<GraphNode<T>>().as_ref() };
        if node.startup_func.is_none() {
            return Ok(());
        }

        let result =
            node.data
                .references
                .compare_exchange(1, 0, Ordering::Acquire, Ordering::Relaxed);
        if let Err(_) = result {
            crate::error!("GraphNode data reference maintained during shutdown");
            return Err(());
        }

        let data = unsafe { node.data.data.get_mut().assume_init_read() };
        let Some(shutdown_func) = node.shutdown_func else {
            return Ok(());
        };

        let result = shutdown_func(data);
        if let Err(data) = result {
            unsafe { node.data.data.get_mut().write(data) };
            node.data.references.store(1, Ordering::Release);
            return Err(());
        }

        Ok(())
    }
}

unsafe impl Send for Internal<'_> {}
unsafe impl Sync for Internal<'_> {}

/// [`LinkedList`]s and storage for [`Link`]s in each [`LinkedList`].
pub(in crate::graph) struct InternalRuntimeInit<'nodes> {
    /// List of all nodes that this [`GraphNode`] requires.
    pub(in crate::graph) required_list: LinkedList<'nodes, Internal<'nodes>>,
    /// List of all nodes that this [`GraphNode`] wants.
    pub(in crate::graph) wanted_list: LinkedList<'nodes, Internal<'nodes>>,

    // Storage for [`Link`]s used to assemble the [`InternalRuntimeInit::required_list`] and
    // [`InternalRuntimeInit::wanted_list`] associated with each [`GraphNode`].
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) required_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) required_by_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],

    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) wanted_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) wanted_by_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],

    /// List of all nodes that this [`GraphNode`] is required by.
    pub(in crate::graph) rev_required_list: LinkedList<'nodes, Internal<'nodes>>,
    /// List of all nodes that this [`GraphNode`] is wanted by.
    pub(in crate::graph) rev_wanted_list: LinkedList<'nodes, Internal<'nodes>>,

    // Storage for [`Link`]s used to assemble the [`InternalRuntimeInit::rev_required_list`] and
    // [`InternalRuntimeInit::rev_wanted_list`] associated with each [`GraphNode`].
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) rev_required_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) rev_required_by_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],

    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) rev_wanted_links:
        &'nodes [ControlledModificationCell<Link<'nodes, Internal<'nodes>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    pub(in crate::graph) rev_wanted_by_links:
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
        required = [$($required_item:expr),* $(,)?],
        required_by = [$($required_by_item:expr),* $(,)?],
        wanted = [$($wanted_item:expr),* $(,)?],
        wanted_by = [$($wanted_by_item:expr),* $(,)?] $(,)?
    ) => {{
        {
            const REQUIRED_COUNT: usize = <[()]>::len(&[ $( { let _ = &$required_item; () }),*]);
            const REQUIRED_BY_COUNT: usize = <[()]>::len(&[ $( { let _ = &$required_by_item; () }),*]);
            const WANTED_COUNT: usize = <[()]>::len(&[ $( { let _ = &$wanted_item; () }),*]);
            const WANTED_BY_COUNT: usize = <[()]>::len(&[ $( { let _ = &$wanted_by_item; () }),*]);

            static REQUIRED: [&$crate::graph::Internal; REQUIRED_COUNT] = [$( &$required_item.internal ),*];
            static REQUIRED_BY: [&$crate::graph::Internal; REQUIRED_BY_COUNT] = [$( &$required_by_item.internal ),*];
            static WANTED: [&$crate::graph::Internal; WANTED_COUNT] = [$( &$wanted_item.internal ),*];
            static WANTED_BY: [&$crate::graph::Internal; WANTED_BY_COUNT] = [$( &$wanted_by_item.internal ),*];

            static REQUIRED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; REQUIRED_COUNT] = [ $( { let _ = &$required_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static REQUIRED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; REQUIRED_BY_COUNT] = [ $( { let _ = &$required_by_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static WANTED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; WANTED_COUNT] = [ $( { let _ = &$wanted_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static WANTED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; WANTED_BY_COUNT] = [ $( { let _ = &$wanted_by_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];

            static REV_REQUIRED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; REQUIRED_COUNT] = [ $( { let _ = &$required_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static REV_REQUIRED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; REQUIRED_BY_COUNT] = [ $( { let _ = &$required_by_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static REV_WANTED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; WANTED_COUNT] = [ $( { let _ = &$wanted_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];
            static REV_WANTED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::node::Internal>,
            >; WANTED_BY_COUNT] = [ $( { let _ = &$wanted_by_item;
                ::sync::ControlledModificationCell::new($crate::graph::linked_list::Link::empty()) }),*
            ];

            $crate::graph::node::GraphNode::new(
                $name,
                $life_cycle_funcs,
                $active,
                &REQUIRED,
                &REQUIRED_LINKS,
                &REV_REQUIRED_LINKS,
                &REQUIRED_BY,
                &REQUIRED_BY_LINKS,
                &REV_REQUIRED_BY_LINKS,
                &WANTED,
                &WANTED_LINKS,
                &REV_WANTED_LINKS,
                &WANTED_BY,
                &WANTED_BY_LINKS,
                &REV_WANTED_BY_LINKS,
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
        active = $active:expr,
        required = [$($required_item:expr),* $(,)?],
        required_by = [$($required_by_item:expr),* $(,)?],
        wanted = [$($wanted_item:expr),* $(,)?],
        wanted_by = [$($wanted_by_item:expr),* $(,)?] $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = None,
            active = $active,
            type = (),
            required = [ $($required_item), *],
            required_by = [ $($required_by_item), *],
            wanted = [ $($wanted_item), *],
            wanted_by = [ $($wanted_by_item), *],
        );
    };
    (
        $static_name:ident,
        $name:expr,
        active = $active:expr,
        type = $type:ty,
        startup = $startup_func:expr,
        required = [$($required_item:expr),* $(,)?],
        required_by = [$($required_by_item:expr),* $(,)?],
        wanted = [$($wanted_item:expr),* $(,)?],
        wanted_by = [$($wanted_by_item:expr),* $(,)?] $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, None)),
            active = $active,
            type = $type,
            required = [ $($required_item), *],
            required_by = [ $($required_by_item), *],
            wanted = [ $($wanted_item), *],
            wanted_by = [ $($wanted_by_item), *],
        );
    };
    (
        $static_name:ident,
        $name:expr,
        active = $active:expr,
        type = $type:ty,
        startup = $startup_func:expr,
        shutdown = $shutdown_func:expr,
        required = [$($required_item:expr),* $(,)?],
        required_by = [$($required_by_item:expr),* $(,)?],
        wanted = [$($wanted_item:expr),* $(,)?],
        wanted_by = [$($wanted_by_item:expr),* $(,)?] $(,)?
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            life_cycle_funcs = Some(($startup_func, Some($shutdown_func))),
            active = $active,
            type = $type,
            required = [ $($required_item), *],
            required_by = [ $($required_by_item), *],
            wanted = [ $($wanted_item), *],
            wanted_by = [ $($wanted_by_item), *],
        );
    };
    (
        $static_name:ident,
        $name:expr,
        life_cycle_funcs = $life_cycle_funcs:expr,
        active = $active:expr,
        type = $type:ty,
        required = [$($required_item:expr),* $(,)?],
        required_by = [$($required_by_item:expr),* $(,)?],
        wanted = [$($wanted_item:expr),* $(,)?],
        wanted_by = [$($wanted_by_item:expr),* $(,)?] $(,)?
    ) => {
        static $static_name: $crate::graph::GraphNode<$type> = {
            #[used]
            #[unsafe(link_section = ".graph.internal")]
            static INTERNAL_REFERENCE: $crate::util::SendSyncNonNull<
                $crate::graph::Internal
            > = $crate::util::SendSyncNonNull(::core::ptr::NonNull::from_ref(&$static_name.internal));

            $crate::make_node!(
                $name,
                $life_cycle_funcs,
                active = $active,
                required = [ $($required_item), *],
                required_by = [ $($required_by_item), *],
                wanted = [ $($wanted_item), *],
                wanted_by = [ $($wanted_by_item), *],
            )
        };
    }
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

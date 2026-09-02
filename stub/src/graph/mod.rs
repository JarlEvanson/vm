//! Implementation of a DAG-based initialization routine.

use core::{
    mem, ptr, slice,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use sync::{ControlledModificationCell, Spinlock};

use crate::graph::linked_list::{Link, LinkIter, LinkedList};

mod linked_list;

/// Lock over the initialization subsystem.
static GRAPH_LOCK: Spinlock<()> = Spinlock::new(());
/// Indicator that the subsystem has been initialized.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Runs the [`InitGraphNode`] with `name` and all of its dependencies.
///
/// # Panics
///
/// Panics if this function is called in a reentrant manner.
pub fn run(node: &'static GraphNode) {
    let Ok(lock) = GRAPH_LOCK.try_lock() else {
        panic!("graph subsystem cannot be called in a reentrant manner");
    };

    let graph = graph_nodes();
    if !INITIALIZED.load(Ordering::Relaxed) {
        // SAFETY:
        //
        // TODO:
        unsafe { initialize(graph) }
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    run_internal(graph, node);

    // Force the lock to be dropped after finishing the execution of the graph.
    drop(lock);
}

/// Returns the list of embedded [`GraphNode`]s.
fn graph_nodes() -> &'static [GraphNode<'static>] {
    #[cfg(not(test))]
    {
        unsafe extern "Rust" {
            #[link_name = "graph_nodes_start"]
            static GRAPH_NODES_START: GraphNode<'static>;
            #[link_name = "graph_nodes_end"]
            static GRAPH_NODES_END: GraphNode<'static>;
        }

        let start = &raw const GRAPH_NODES_START;
        let end = &raw const GRAPH_NODES_END;

        let size = (end.addr() - start.addr()) / mem::size_of::<GraphNode>();
        // SAFETY:
        //
        // The contained [`GraphNode`]s were implemented using a macro and are initialized.
        unsafe { slice::from_raw_parts(start, size) }
    }
    #[cfg(test)]
    test::graph_nodes()
}

/// Initializes the linked lists of required, after, and desired after [`GraphNode`]s.
///
/// # Safety
///
/// - All [`GraphNode`]s provided in `graph` must be under the exclusive control of this
///   [`initialize()`] call.
/// - All [`GraphNode`]s must have all of their referenced [`GraphNode`]s be under the exclusive
///   control of this [`initialize()`] call.
unsafe fn initialize<'a>(graph: &'a [GraphNode<'a>]) {
    for node in graph {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        unsafe { node.link.get_mut().set_value(Some(node)) }
    }

    for node in graph {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        let runtime_info = unsafe { node.runtime_info.get_mut() };

        for (&required, link) in node.required.iter().zip(runtime_info.required_links.iter()) {
            assert_eq!(
                ptr::from_ref(required),
                ptr::from_ref(node),
                "a node must not require itself"
            );

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            let link_mut = unsafe { link.get_mut() };
            link_mut.set_value(Some(required));

            runtime_info.required_list.push_back(link);
        }

        for (&required_by, link) in node
            .required_by
            .iter()
            .zip(runtime_info.required_by_links.iter())
        {
            assert_eq!(
                ptr::from_ref(required_by),
                ptr::from_ref(node),
                "a node must not be required by itself"
            );

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            let link_mut = unsafe { link.get_mut() };
            link_mut.set_value(Some(node));

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            unsafe {
                required_by
                    .runtime_info
                    .get_mut()
                    .required_list
                    .push_back(link)
            }
        }

        for (&wanted, link) in node.wanted.iter().zip(runtime_info.wanted_links.iter()) {
            assert_eq!(
                ptr::from_ref(wanted),
                ptr::from_ref(node),
                "a node must not require itself"
            );

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            let link_mut = unsafe { link.get_mut() };
            link_mut.set_value(Some(wanted));

            runtime_info.wanted_list.push_back(link);
        }

        for (&wanted_by, link) in node
            .wanted_by
            .iter()
            .zip(runtime_info.wanted_by_links.iter())
        {
            assert_eq!(
                ptr::from_ref(wanted_by),
                ptr::from_ref(node),
                "a node must not be wanted by itself"
            );

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            let link_mut = unsafe { link.get_mut() };
            link_mut.set_value(Some(node));

            // SAFETY:
            //
            // The invariants of this function ensure that this operation is safe.
            unsafe { wanted_by.runtime_info.get_mut().wanted_list.push_back(link) }
        }
    }
}

/// Runs an execution graph until the target `node` is executed.
fn run_internal<'a>(graph: &'a [GraphNode<'a>], node: &'static GraphNode) {
    todo!()
}

/// Sets all [`GraphNode`]s provided in `graph` that are in [`State::WaitingForProcessing`]
/// to [`State::Enabled`].
///
/// # Safety
///
/// All [`GraphNode`]s provided in `graph` must be under the exclusive control of this
/// [`clear_waiting()`] call.
unsafe fn clear_waiting<'a>(graph: &'a [GraphNode<'a>]) {
    for node in graph {
        if node.state() == State::WaitingForProcessing {
            node.set_state(State::Enabled);
        }
    }
}

/// Representation of a task or stage in the execution graph.
pub struct GraphNode<'a> {
    /// The name of the [`GraphNode`].
    ///
    /// This is not unique.
    name: &'a str,

    /// The state of this [`GraphNode`].
    state: AtomicU8,

    /// An intrusive link used for computing the execution graph.
    link: ControlledModificationCell<Link<'a, GraphNode<'a>>>,

    /// [`GraphNode`]s that this [`GraphNode`] requires to successfully execute before it executes
    /// and thus adds to the graph calculations when this [`GraphNode`] is considered for
    /// execution.
    ///
    /// [`GraphNode`]s in this list are always executed before this [`GraphNode`].
    required: &'a [&'a GraphNode<'a>],
    /// [`GraphNode`]s for which this [`GraphNode`] is required (used to enable decentralized
    /// dependencies).
    required_by: &'a [&'a GraphNode<'a>],

    /// [`GraphNode`]s that this [`GraphNode`] wants to execute before it executes and thus adds to
    /// the graph calculations when this [`GraphNode`] is considered for execution and the wanted
    /// [`GraphNode`] is enabled.
    ///
    /// [`GraphNode`]s in this list are executed before this [`GraphNode`] if added to the execution
    /// graph and no cycle forms that involves the two [`GraphNode`]s. If a cycle forms, this wanted
    /// relationship is relaxed to break the cycle. Furthermore, the execution of this [`GraphNode`]
    /// will proceed even if wanted [`GraphNode`]s fail during execution.
    wanted: &'a [&'a GraphNode<'a>],
    /// [`GraphNode`]s for which this [`GraphNode`] is wanted (used to enable decentralized
    /// dependencies).
    wanted_by: &'a [&'a GraphNode<'a>],

    /// Information modified at runtime.
    runtime_info: ControlledModificationCell<RuntimeInfo<'a>>,
}

type StartupFunc = Option<()>;
type ShutdownFunc = Option<()>;

impl<'a> GraphNode<'a> {
    #[doc(hidden)]
    pub const fn new<const R: usize, const RB: usize, const W: usize, const WB: usize>(
        name: &'a str,
        startup: StartupFunc,
        shutdown: ShutdownFunc,
        enabled: bool,
        required: &'a [&'a GraphNode<'a>; R],
        required_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>; R],
        required_by: &'a [&'a GraphNode<'a>; RB],
        required_by_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>; RB],
        wanted: &'a [&'a GraphNode<'a>; W],
        wanted_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>; W],
        wanted_by: &'a [&'a GraphNode<'a>; WB],
        wanted_by_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>; WB],
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

            runtime_info: ControlledModificationCell::new(RuntimeInfo {
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
    fn state(&self) -> State {
        State::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Sets the [`State`] of this [`GraphNode`].
    fn set_state(&self, state: State) {
        self.state.store(state.to_u8(), Ordering::Relaxed);
    }

    /// Returns an [`Iterator`] over the required [`GraphNode`]s for this [`GraphNode`].
    fn required(&self) -> LinkIter<'a, GraphNode<'a>> {
        self.runtime_info.get().required_list.iter()
    }

    /// Returns an [`Iterator`] over the wanted [`GraphNode`]s for this [`GraphNode`].
    fn wanted(&self) -> LinkIter<'a, GraphNode<'a>> {
        self.runtime_info.get().wanted_list.iter()
    }
}

struct RuntimeInfo<'a> {
    /// List of all nodes that this [`GraphNode`] requires.
    required_list: LinkedList<'a, GraphNode<'a>>,
    /// List of all nodes that this [`GraphNode`] wants.
    wanted_list: LinkedList<'a, GraphNode<'a>>,

    // Storage for [`Link`]s used to assemble the [`RuntimeInfo::required_list`] and
    // [`RuntimeInfo::wanted_list`] associated with each [`GraphNode`].
    #[expect(clippy::missing_docs_in_private_items)]
    required_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    required_by_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>],

    #[expect(clippy::missing_docs_in_private_items)]
    wanted_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>],
    #[expect(clippy::missing_docs_in_private_items)]
    wanted_by_links: &'a [ControlledModificationCell<Link<'a, GraphNode<'a>>>],
}

/// The execution state of an [`GraphNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// The [`GraphNode`] is disabled and thus is not considered in graph calculations.
    Disabled,
    /// The [`GraphNode`] is enabled and thus is considered in graph calculations.
    Enabled,

    /// The [`GraphNode`] is waiting for processing during the current graph operation.
    WaitingForProcessing,

    /// The [`GraphNode`] is currently active.
    Active,

    /// The [`GraphNode`] is startup failed.
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
#[macro_export]
macro_rules! make_node {
    (
        $name:expr,
        $startup_func:expr,
        $shutdown_func:expr,
        active = $active:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr,
    ) => {{
        {
            static REQUIRED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::GraphNode>,
            >; const {
                <[$crate::graph::GraphNode]>::len(&$required)
            }] = [const { ::sync::ControlledModificationCell::new($crate::graph::Link::empty()) };
                <[$crate::graph::GraphNode]>::len(&$required)];
            static REQUIRED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::GraphNode>,
            >; const {
                <[$crate::graph::GraphNode]>::len(&$required_by)
            }] = [const { ::sync::ControlledModificationCell::new($crate::graph::Link::empty()) };
                <[$crate::graph::GraphNode]>::len(&$required_by)];
            static WANTED_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::GraphNode>,
            >;
                const { <[$crate::graph::GraphNode]>::len(&$wanted) }] =
                [const { ::sync::ControlledModificationCell::new($crate::graph::Link::empty()) };
                    <[$crate::graph::GraphNode]>::len(&$wanted)];
            static WANTED_BY_LINKS: [::sync::ControlledModificationCell<
                $crate::graph::linked_list::Link<$crate::graph::GraphNode>,
            >; const {
                <[$crate::graph::GraphNode]>::len(&$wanted_by)
            }] = [const { ::sync::ControlledModificationCell::new($crate::graph::Link::empty()) };
                <[$crate::graph::GraphNode]>::len(&$wanted_by)];
            $crate::graph::GraphNode::new(
                $name,
                $startup_func,
                $shutdown_func,
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
        $startup_func:expr,
        $shutdown_func:expr,
        active = $active:expr,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr,
    ) => {
        #[used]
        #[unsafe(link_section = ".graph_node")]
        static $static_name: $crate::graph::GraphNode = make_node!(
            $name,
            $startup_func,
            $shutdown_func,
            active = $active,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        );
    };
    (
        $static_name:ident,
        $name:expr,
        startup = $startup_func:expr,
        shutdown = $shutdown_func:expr,
        active,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr,
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            startup = $startup_func,
            shutdown = $shutdown_func,
            active = true,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        )
    };
    (
        $static_name:ident,
        $name:expr,
        $startup_func:expr,
        $shutdown_func:expr,
        inactive,
        required = $required:expr,
        required_by = $required_by:expr,
        wanted = $wanted:expr,
        wanted_by = $wanted_by:expr,
    ) => {
        $crate::define_node!(
            $static_name,
            $name,
            startup = $startup_func,
            shutdown = $shutdown_func,
            active = false,
            required = $required,
            required_by = $required_by,
            wanted = $wanted,
            wanted_by = $wanted_by,
        )
    };
}

#[cfg(test)]
mod test {
    use crate::graph::{GraphNode, State};

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

    define_node!(
        TEST,
        "a",
        None,
        None,
        active = true,
        required = [],
        required_by = [],
        wanted = [],
        wanted_by = [],
    );

    pub fn graph_nodes() -> &'static [GraphNode<'static>] {
    }

    #[test]
    fn run_infrastructure() {
        super::run(&TEST);
    }
}

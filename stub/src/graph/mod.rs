//! Implementation of a graph-based initialization routine.

use core::{
    mem, ptr, slice,
    sync::atomic::{AtomicBool, Ordering},
};

use sync::Spinlock;

use crate::{
    graph::{
        linked_list::LinkedList,
        node::{GraphNode, Internal, State},
    },
    util::SendSyncPointer,
};

#[doc(hidden)]
pub(crate) mod linked_list;
#[doc(hidden)]
pub(crate) mod node;

/// Lock over the initialization subsystem.
static GRAPH_LOCK: Spinlock<()> = Spinlock::new(());
/// Indicator that the subsystem has been initialized.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Runs the [`GraphNode`] with `name` and all of its dependencies.
///
/// # Panics
///
/// Panics if this function is called in a reentrant manner.
pub fn run<T>(node: &'static GraphNode<'static, T>) {
    let Ok(lock) = GRAPH_LOCK.try_lock() else {
        panic!("graph subsystem cannot be called in a reentrant manner");
    };

    let graph = graph_nodes();
    let graph = internal_iter(graph);

    if !INITIALIZED.load(Ordering::Relaxed) {
        // SAFETY:
        //
        // TODO:
        unsafe { initialize(graph.clone()) }
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    run_internal(graph, &node.internal);

    // Force the lock to be dropped after finishing the execution of the graph.
    drop(lock);
}

/// Returns the list of embedded [`Internal`]s.
fn graph_nodes() -> &'static [SendSyncPointer<Internal<'static>>] {
    unsafe extern "Rust" {
        #[link_name = "graph_internal_start"]
        static GRAPH_INTERNAL_START: SendSyncPointer<Internal<'static>>;
        #[link_name = "graph_internal_end"]
        static GRAPH_INTERNAL_END: SendSyncPointer<Internal<'static>>;
    }

    let start = &raw const GRAPH_INTERNAL_START;
    let end = &raw const GRAPH_INTERNAL_END;

    let size = (end.addr() - start.addr()) / mem::size_of::<SendSyncPointer<Internal>>();
    // SAFETY:
    //
    // The contained [`SendSyncPo<Internal>`]s were implemented using a macro and are initialized.
    unsafe { slice::from_raw_parts(start, size) }
}

/// Converts the provided list of [`SendSyncPointer<Internal>]s into a list of [`Internal`]s.
fn internal_iter<'a>(
    graph: &'a [SendSyncPointer<Internal<'a>>],
) -> impl Iterator<Item = &'a Internal<'a>> + Clone {
    graph.iter().map(|ptr| {
        let internal_ptr = ptr.0;
        unsafe { &*internal_ptr }
    })
}

/// Initializes the [`LinkedList`]s of required and wants [`Internal`]s.
///
/// # Safety
///
/// - All [`SendSyncPointer`]s provided in `graph` must be under the exclusive control of this
///   [`initialize()`] call.
/// - All [`Internal`]s must have all of their referenced [`GraphNode`]s be under the exclusive
///   control of this [`initialize()`] call.
unsafe fn initialize<'a>(graph: impl Iterator<Item = &'a Internal<'a>> + Clone) {
    for node in graph.clone() {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        unsafe { node.link.get_mut().set_value(Some(node)) }
    }

    for node in graph {
        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        let runtime_info = unsafe { node.runtime_init.get_mut() };

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
                    .runtime_init
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
            unsafe { wanted_by.runtime_init.get_mut().wanted_list.push_back(link) }
        }
    }
}

/// Runs an execution graph until the target `node` is executed.
fn run_internal<'a>(graph: impl Iterator<Item = &'a Internal<'a>> + Clone, node: &'a Internal<'a>) {
    assert_ne!(
        node.state(),
        State::Disabled,
        "disabled nodes must not be executed"
    );

    // Reset markings from previous executions.
    for node in graph.clone() {
        if node.state() == State::WaitingForProcessing {
            node.set_state(State::Enabled);
        }
    }

    let mut queue = LinkedList::new();

    // Mark all nodes that should be run as `State::WaitingForProcessing`.
    queue.push_back(&node.link);
    while let Some(node) = queue.pop_front() {
        let node = node.get().value();
        for requirement in node.required() {
            if matches!(requirement.state(), State::Disabled) {
                continue;
            } else if matches!(requirement.state(), State::Failed) {
                panic!(
                    "node required by '{}' is in a failed state: {}",
                    node.name, requirement.name
                );
            }

            requirement.set_state(State::WaitingForProcessing);
            queue.push_back(&requirement.link);
        }

        for wanted in node.wanted() {
            if matches!(wanted.state(), State::Disabled) {
                continue;
            } else if matches!(wanted.state(), State::Failed) {
                crate::trace!(
                    "node wanted by '{}' is in a failed state: {}",
                    node.name,
                    wanted.name
                );
                continue;
            }

            wanted.set_state(State::WaitingForProcessing);
            queue.push_back(&wanted.link);
        }
    }

    // Gather all nodes that should be run in a single list.
    let mut node_count = 0;
    for node in graph.clone() {
        if node.state() != State::WaitingForProcessing {
            continue;
        }

        queue.push_back(&node.link);
        node_count += 1;
    }

    let mut loops_since_last_execution = 0;
    'outer: while let Some(node) = queue.pop_front() {
        let node = node.get().value();
        assert_eq!(node.state(), State::WaitingForProcessing);

        if loops_since_last_execution == node_count {
            unreachable!("cycle in execution graph");
        }

        for requirement in node.required() {
            if requirement.state() == State::WaitingForProcessing {
                loops_since_last_execution += 1;
                queue.push_back(&node.link);
                continue 'outer;
            } else if requirement.state() == State::Failed {
                panic!(
                    "node required by '{}' failed during execution: {}",
                    node.name, requirement.name
                );
            }
        }

        for wanted in node.wanted() {
            if wanted.state() == State::WaitingForProcessing {
                loops_since_last_execution += 1;
                queue.push_back(&node.link);
                continue 'outer;
            }
        }

        todo!("implement node execution");

        loops_since_last_execution = 0;
        node_count -= 1;
    }
}

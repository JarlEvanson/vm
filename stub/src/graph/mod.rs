//! Implementation of a graph-based initialization routine.

use core::ptr;

use crate::{
    graph::{linked_list::LinkedList, node::State},
    util::SendSyncNonNull,
};

mod linked_list;
mod node;

pub use node::{GraphNode, Internal, Reference};

/// An [`Iterator`] over the [`GraphNode`]s.
#[derive(Clone)]
struct GraphNodeIter<'nodes>(&'nodes [SendSyncNonNull<Internal<'nodes>>]);

impl<'nodes> Iterator for GraphNodeIter<'nodes> {
    type Item = &'nodes Internal<'nodes>;

    fn next(&mut self) -> Option<Self::Item> {
        let internal_ptr = self.0.first()?;
        self.0 = &self.0[1..];
        // SAFETY:
        //
        // The provided ist is correct since this is a constrained module.
        unsafe { Some(internal_ptr.0.as_ref()) }
    }
}

/// Initializes the [`LinkedList`]s of required and wants [`Internal`]s.
///
/// # Safety
///
/// - All [`SendSyncPointer`]s provided in `graph` must be under the exclusive control of this
///   [`initialize_internal()`] call.
/// - All [`Internal`]s must have all of their referenced [`GraphNode`]s be under the exclusive
///   control of this [`initialize_internal()`] call.
unsafe fn initialize_internal<'a>(graph: GraphNodeIter<'a>) {
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
            assert_ne!(
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
            assert_ne!(
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
            assert_ne!(
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
            assert_ne!(
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
///
/// # Safety
///
/// - All [`SendSyncPointer`]s provided in `graph` must be under the exclusive control of this
///   [`startup_internal()`] call.
/// - All [`Internal`]s must have all of their referenced [`GraphNode`]s be under the exclusive
///   control of this [`startup_internal()`] call.
unsafe fn startup_internal<'a>(graph: GraphNodeIter<'a>, node: &'a Internal<'a>) {
    assert_ne!(
        node.state(),
        State::Disabled,
        "disabled nodes must not be executed"
    );

    for node in graph.clone() {
        assert_ne!(
            node.state(),
            State::WaitingForProcessing,
            "partiallly complete runs are not supported"
        );
    }

    let mut queue = LinkedList::new();

    // Mark all nodes that should be run as `State::WaitingForProcessing`.
    if !matches!(node.state(), State::Active) {
        node.set_state(State::WaitingForProcessing);
    }
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
            } else if matches!(requirement.state(), State::Active) {
                continue;
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
            } else if matches!(wanted.state(), State::Active) {
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

        match node.startup() {
            Ok(()) => node.set_state(State::Active),
            Err(()) => node.set_state(State::Failed),
        }

        loops_since_last_execution = 0;
        node_count -= 1;
    }
}

/// Shuts down an execution graph.
///
/// # Safety
///
/// - All [`SendSyncPointer`]s provided in `graph` must be under the exclusive control of this
///   [`shutdown_internal()`] call.
/// - All [`Internal`]s must have all of their referenced [`GraphNode`]s be under the exclusive
///   control of this [`shutdown_internal()`] call.
unsafe fn shutdown_internal<'a>(graph: GraphNodeIter<'a>) {
    for node in graph.clone() {
        assert_ne!(
            node.state(),
            State::WaitingForProcessing,
            "partially complete runs are not supported"
        );
    }

    let mut queue = LinkedList::new();
    
    let mut node_count = 0;
    for node in graph.clone() {
        if !matches!(node.state(), State::Active) {
            continue;
        }

        queue.push_back(&node.link);
        node_count += 1;
    }

    while let Some(node) = queue.pop_front() {
        let node = node.get().value();
        assert_ne!(node.state(), State::WaitingForProcessing);

        if loops_since_last_execution == node_count {
            unreachable!("cycle in execution graph");
        }
    }

    todo!()
}

#[cfg(test)]
mod test {
    use core::{
        ptr::NonNull,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::{
        define_node,
        graph::{GraphNodeIter, Internal, initialize_internal, startup_internal},
        util::SendSyncNonNull,
    };

    #[test]
    fn graph_startup_works() {
        static A: AtomicUsize = AtomicUsize::new(0);

        define_node!(
            NODE_A,
            "NODE A",
            active = true,
            type = (),
            startup = a_startup,
            shutdown = a_shutdown,
            required = [],
            required_by = [],
            wanted = [],
            wanted_by = [],
        );

        fn a_startup() -> Result<(), ()> {
            assert_eq!(A.fetch_add(1, Ordering::Relaxed), 0);
            Ok(())
        }

        fn a_shutdown(_: ()) -> Result<(), ()> {
            assert_eq!(A.fetch_sub(1, Ordering::Relaxed), 1);
            Ok(())
        }

        define_node!(
            NODE_B,
            "NODE B",
            active = true,
            type = bool,
            startup = b_startup,
            required = [NODE_A],
            required_by = [],
            wanted = [],
            wanted_by = [],
        );

        fn b_startup() -> Result<bool, ()> {
            let _ = A.fetch_add(1, Ordering::Relaxed);
            Ok(true)
        }

        define_node!(
            NODE_C,
            "NODE C",
            active = true,
            type = (),
            startup = c_startup,
            required = [NODE_B],
            required_by = [],
            wanted = [],
            wanted_by = [],
        );

        fn c_startup() -> Result<(), ()> {
            let _ = A.fetch_sub(1, Ordering::Relaxed);
            assert_eq!(*NODE_B.ref_data().unwrap(), true);
            Ok(())
        }

        static NODES: [SendSyncNonNull<Internal>; 3] = [
            SendSyncNonNull(NonNull::from_ref(&NODE_A.internal)),
            SendSyncNonNull(NonNull::from_ref(&NODE_B.internal)),
            SendSyncNonNull(NonNull::from_ref(&NODE_C.internal)),
        ];

        let iter = GraphNodeIter(&NODES);

        unsafe { initialize_internal(iter.clone()) };
        unsafe { startup_internal(iter, &NODE_C.internal) }
    }
}

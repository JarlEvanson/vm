//! Implementation of a graph-based initialization subsystem.

use core::assert_matches;

use crate::{
    graph::{linked_list::LinkedList, node::State},
    util::SendSyncNonNull,
};

mod linked_list;
mod node;

pub use node::{Edge, GraphNode, Internal};

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
/// All [`Internal`] structures reachable from the [`SendSyncNonNull`] pointers in `graph` must be
/// under the exclusive control of this call for the duration of this call.
unsafe fn initialize_internal<'a>(graph: GraphNodeIter<'a>) {
    let internal_ptr_iter = graph.0.iter();
    for (node, internal_ptr) in graph.clone().zip(internal_ptr_iter) {
        // SAFETY:
        //
        // All [`GraphNodeIter`] instances of [`Internal`] reachable from `graph` are contained
        // inside a [`GraphNode`].
        let node_ptr = unsafe { internal_ptr.0.byte_sub(node.node_offset()).cast::<()>() };

        // SAFETY:
        //
        // The invariants of this function ensure that this operation is safe.
        unsafe { node.initialize(node_ptr) };
    }
}

/// Runs an execution graph until the target `node` is executed.
///
/// # Safety
///
/// All [`Internal`] structures reachable from the [`SendSyncNonNull`] pointers in `graph` must be
/// under the exclusive control of this call for the duration of this call. The `GraphNode::data` is
/// excluded.
unsafe fn activate_internal<'a>(graph: GraphNodeIter<'a>, node: &'a Internal<'a>) {
    assert_ne!(
        node.state(),
        State::Disabled,
        "disabled nodes must not be activated"
    );

    for node in graph.clone() {
        assert_ne!(
            node.state(),
            State::WaitingForActivation,
            "partiallly complete runs are not supported"
        );
        assert_ne!(
            node.state(),
            State::WaitingForRerun,
            "partiallly complete runs are not supported"
        );
    }

    let mut queue = LinkedList::new();

    // Mark all nodes that should be run as waiting to indicate that they should be processed.
    match node.state() {
        State::Disabled => unreachable!(),
        State::Enabled => node.set_state(State::WaitingForActivation),
        State::WaitingForActivation => unreachable!(),
        State::Active => {} // Process all dependencies.
        State::WaitingForRerun => unreachable!(),
        State::ActiveRerun => node.set_state(State::WaitingForRerun),
        State::Failed => panic!("failed nodes must not be activated"),
    }

    queue.push_back(node.link());
    while let Some(link) = queue.pop_front() {
        let node = link.get().value();
        for dependency in node.dependencies() {
            match dependency.state() {
                State::Disabled => continue,
                State::Enabled => dependency.set_state(State::WaitingForActivation),
                State::WaitingForActivation => continue,
                State::Active => {} // Process all dependencies.
                State::WaitingForRerun => continue,
                State::ActiveRerun => dependency.set_state(State::WaitingForRerun),
                State::Failed => {
                    panic!(
                        "dependency of '{}' is in a failed state: {}",
                        node.name(),
                        dependency.name()
                    );
                }
            }

            queue.push_back(dependency.link());
        }
    }

    let mut standard = LinkedList::new();
    let mut extended = LinkedList::new();

    // Gather all nodes that should be run in a single list.
    let mut standard_count = 0;
    let mut extended_count = 0;
    for node in graph {
        match node.state() {
            State::Disabled | State::Enabled => continue,
            State::WaitingForActivation => {}
            State::Active => continue,
            State::WaitingForRerun => {}
            State::ActiveRerun | State::Failed => continue,
        }

        if node.extended_lifetime() {
            extended.push_back(node.link());
            extended_count += 1;
        } else {
            queue.push_back(node.link());
            standard_count += 1;
        }
    }

    'execution_loop: {
        let mut loops_since_last_execution = 0;

        'try_node: loop {
            if standard_count == 0 && extended_count == 0 {
                break 'execution_loop;
            }

            let link = if loops_since_last_execution < extended_count {
                extended.pop_front().unwrap()
            } else if loops_since_last_execution - extended_count < standard_count {
                standard.pop_front().unwrap()
            } else {
                panic!("cycle in execution graph");
            };

            let node = link.get().value();
            assert_matches!(
                node.state(),
                State::WaitingForActivation | State::WaitingForRerun
            );

            for dependency in node.dependencies() {
                match dependency.state() {
                    State::Disabled => continue,
                    State::Enabled => unreachable!(),
                    State::WaitingForActivation | State::WaitingForRerun => {
                        loops_since_last_execution += 1;
                        if node.extended_lifetime() {
                            extended.push_back(link);
                        } else {
                            queue.push_back(link);
                        }
                        continue 'try_node;
                    }
                    State::Active | State::ActiveRerun => {}
                    State::Failed => panic!(
                        "dependency of '{}' failed during activation: {}",
                        node.name(),
                        dependency.name(),
                    ),
                }
            }

            // SAFETY:
            //
            // The invariants of this function fulfill the invariants of `node.activate()`.
            let result = unsafe { node.activate() };
            match result {
                Ok(()) => match node.state() {
                    State::Disabled | State::Enabled => unreachable!(),
                    State::WaitingForActivation => node.set_state(State::Active),
                    State::Active => unreachable!(),
                    State::WaitingForRerun => node.set_state(State::ActiveRerun),
                    State::ActiveRerun => unreachable!(),
                    State::Failed => unreachable!(),
                },
                Err(()) => node.set_state(State::Failed),
            }

            loops_since_last_execution = 0;
            if node.extended_lifetime() {
                extended_count -= 1;
            } else {
                standard_count -= 1;
            }
        }
    }

    if node.state() == State::Failed {
        panic!("activation of requested node '{}' failed", node.name());
    }
}

#[cfg(test)]
mod test {
    use core::{
        ptr::NonNull,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::{
        define_node,
        graph::{GraphNodeIter, Internal, activate_internal, initialize_internal},
        util::SendSyncNonNull,
    };

    #[test]
    fn initialize() {
        define_node!(
            A,
            "A",
            active = true,
            extended_lifetime = true,
            required_enabled = [],
            required_disabled = [],
            required_by_enabled = [B],
            required_by_disabled = [],
        );

        define_node!(
            B,
            "B",
            active = true,
            extended_lifetime = true,
            required_enabled = [A],
            required_disabled = [],
            required_by_enabled = [],
            required_by_disabled = [C],
        );

        define_node!(
            C,
            "C",
            active = true,
            extended_lifetime = true,
            required_enabled = [],
            required_disabled = [B],
            required_by_enabled = [],
            required_by_disabled = [],
        );

        static NODES: [SendSyncNonNull<Internal>; 3] = [
            SendSyncNonNull(NonNull::from_ref(A.internal())),
            SendSyncNonNull(NonNull::from_ref(B.internal())),
            SendSyncNonNull(NonNull::from_ref(C.internal())),
        ];

        // SAFETY:
        //
        // Thse statics are protected by their definition inside a function and thus are not able to
        // be referenced aside of this function.
        unsafe { initialize_internal(GraphNodeIter(&NODES)) }
    }

    #[test]
    fn graph_startup_works() {
        static DATA_A: AtomicUsize = AtomicUsize::new(0);

        define_node!(
            A,
            "A",
            type = (),
            activation = a_startup,
            deactivation = a_shutdown,
            active = true,
            extended_lifetime = true,
            required_enabled = [],
            required_disabled = [],
            required_by_enabled = [],
            required_by_disabled = [],
        );

        fn a_startup() -> Result<(), ()> {
            assert_eq!(DATA_A.fetch_add(1, Ordering::Relaxed), 0);
            Ok(())
        }

        fn a_shutdown(_: ()) -> Result<(), ()> {
            assert_eq!(DATA_A.fetch_sub(1, Ordering::Relaxed), 1);
            Ok(())
        }

        define_node!(
            B,
            "B",
            type = bool,
            activation = b_startup,
            active = true,
            extended_lifetime = true,
            required_enabled = [A],
            required_disabled = [],
            required_by_enabled = [],
            required_by_disabled = [],
        );

        fn b_startup() -> Result<bool, ()> {
            let _ = DATA_A.fetch_add(1, Ordering::Relaxed);
            Ok(true)
        }

        define_node!(
            C,
            "C",
            type = (),
            activation = c_startup,
            active = true,
            extended_lifetime = true,
            required_enabled = [B],
            required_disabled = [],
            required_by_enabled = [],
            required_by_disabled = [],
        );

        fn c_startup() -> Result<(), ()> {
            let _ = DATA_A.fetch_sub(1, Ordering::Relaxed);
            assert_eq!(*B.ref_data(), true);
            Ok(())
        }

        static NODES: [SendSyncNonNull<Internal>; 3] = [
            SendSyncNonNull(NonNull::from_ref(A.internal())),
            SendSyncNonNull(NonNull::from_ref(B.internal())),
            SendSyncNonNull(NonNull::from_ref(C.internal())),
        ];

        let iter = GraphNodeIter(&NODES);

        unsafe { initialize_internal(iter.clone()) };
        unsafe { activate_internal(iter, C.internal()) }
    }
}

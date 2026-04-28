//! Implementation of a DAG-based initialization routine.
//!
//! This is based on Managarm's `InitGraph`.

use core::{
    mem, slice,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use sync::{ControlledModificationCell, Spinlock};

unsafe extern "Rust" {
    #[link_name = "init_node_start"]
    static INIT_NODE_START: InitGraphNode<'static>;
    #[link_name = "init_node_end"]
    static INIT_NODE_END: InitGraphNode<'static>;
}

/// Lock over the initialization subsystem.
static INITGRAPH_LOCK: Spinlock<()> = Spinlock::new(());
/// Indicator that the subsystem has been initialized.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Runs the [`InitGraphNode`] with `name` and all of its dependencies.
///
/// # Panics
///
/// Panics if this function is called in a reentrant manner.
pub fn run(node: &'static InitGraphNode) {
    let Ok(lock) = INITGRAPH_LOCK.try_lock() else {
        panic!("initgraph subsystem cannot be called in a reentrant manner");
    };

    let nodes = init_nodes();
    if !INITIALIZED.load(Ordering::Relaxed) {
        initialize(nodes);
        INITIALIZED.store(true, Ordering::Relaxed);
    }

    run_internal(node, nodes);

    drop(lock);
}

/// Returns the list of embedded [`InitGraphNode`]s.
fn init_nodes() -> &'static [InitGraphNode<'static>] {
    let start = &raw const INIT_NODE_START;
    let end = &raw const INIT_NODE_END;

    let size = (end.addr() - start.addr()) / mem::size_of::<InitGraphNode>();
    // SAFETY:
    //
    // The contained [`InitGraphNode`]s were implemented using a macro and are initialized.
    unsafe { slice::from_raw_parts(start, size) }
}

/// Executes an initialization graph until the target `node` is executed.
fn run_internal<'a>(node: &'a InitGraphNode<'a>, nodes: &'a [InitGraphNode<'a>]) {
    let mut queue = LinkList::new();

    assert_ne!(
        node.state(),
        State::Inactive,
        "inactive nodes must not be executed"
    );
    if node.state() == State::Done {
        return;
    }

    // Reset markings from previous executions.
    clear_wanted(nodes);

    // Mark all nodes that should be run as `State::Wanted`.
    queue.push_back(&node.link);
    while let Some(node) = queue.pop_front() {
        let node = node.get().node();
        for requirement in node.requires() {
            if requirement.state() != State::Active {
                continue;
            }

            requirement.set_state(State::Wanted);
            queue.push_back(&requirement.link);
        }
    }

    // Gather all nodes that should be run in a single list.
    let mut node_count = 0;
    for node in nodes {
        if node.state() != State::Wanted {
            continue;
        }

        queue.push_back(&node.link);
        node_count += 1;
    }

    let mut loops_since_last_execution = 0;
    'outer: while let Some(node) = queue.pop_front() {
        let node = node.get().node();
        assert_eq!(node.state(), State::Wanted);

        if loops_since_last_execution == node_count {
            unreachable!("cycle in execution graph");
        }

        for requirement in node.requires() {
            if requirement.state() == State::Wanted {
                loops_since_last_execution += 1;
                queue.push_back(&node.link);
                continue 'outer;
            }
        }

        for after_node in node.after() {
            if after_node.state() == State::Wanted {
                loops_since_last_execution += 1;
                queue.push_back(&node.link);
                continue 'outer;
            }
        }

        node.execute();
        node.set_state(State::Done);

        node_count -= 1;
    }
}

/// Clears all nodes of their [`State::Wanted`] status.
fn clear_wanted<'a>(nodes: &'a [InitGraphNode<'a>]) {
    for node in nodes {
        if node.state() == State::Wanted {
            node.set_state(State::Active);
        }
    }
}

/// Initializes the linked lists of required and after nodes.
fn initialize<'a>(nodes: &'a [InitGraphNode<'a>]) {
    for (i, node) in nodes.iter().enumerate() {
        for test_node in nodes.iter().skip(i + 1) {
            assert_ne!(
                node.name, test_node.name,
                "execution node names must be unique"
            );
        }

        // SAFETY:
        //
        // This function and all related functions are controlled by a spinlock.
        unsafe { node.link.get_mut().node = Some(node) }
    }

    for node in nodes {
        let CompileTimeDesc::Task {
            func: _,
            requires,
            dependents,
            after,
            before,
        } = node.compile_time_desc
        else {
            continue;
        };

        // SAFETY:
        //
        // This function and all related functions are controlled by a spinlock.
        let runtime_desc = unsafe { node.runtime_desc.get_mut() };

        'outer: for (&required, link) in requires.iter().zip(runtime_desc.requires_links.iter()) {
            assert_ne!(required, node.name, "a node must not require itself");

            for test_node in nodes {
                if required == test_node.name {
                    // SAFETY:
                    //
                    // This node is protected by the spinlock and will not be referenced by itself.
                    let link_mut = unsafe { link.get_mut() };
                    link_mut.node = Some(test_node);

                    runtime_desc.requires_list.push_back(link);
                    continue 'outer;
                }
            }

            unreachable!("invalid execution DAG: missing node name {}", required);
        }

        'outer: for (&after, link) in after.iter().zip(runtime_desc.after_links.iter()) {
            assert_ne!(after, node.name, "a node must not be after itself");

            for test_node in nodes {
                if after == test_node.name {
                    // SAFETY:
                    //
                    // This node is protected by the spinlock and will not be referenced by itself.
                    let link_mut = unsafe { link.get_mut() };
                    link_mut.node = Some(test_node);

                    runtime_desc.after_list.push_back(link);
                    continue 'outer;
                }
            }

            unreachable!("invalid execution DAG: missing node name {}", after);
        }

        'outer: for (&dependent, link) in
            dependents.iter().zip(runtime_desc.dependents_links.iter())
        {
            assert_ne!(
                dependent, node.name,
                "a node must not be dependent on itself"
            );

            for test_node in nodes {
                if dependent == test_node.name {
                    // SAFETY:
                    //
                    // This node is protected by the spinlock and will not be referenced by itself.
                    let link_mut = unsafe { link.get_mut() };
                    link_mut.node = Some(test_node);

                    // SAFETY:
                    //
                    // This function and all related functions are controlled by a spinlock.
                    let runtime_desc = unsafe { test_node.runtime_desc.get_mut() };
                    runtime_desc.requires_list.push_back(link);
                    continue 'outer;
                }
            }

            unreachable!("invalid execution DAG: missing node name {}", dependent);
        }

        'outer: for (&before, link) in before.iter().zip(runtime_desc.before_links.iter()) {
            assert_ne!(before, node.name, "a node must not be before on itself");

            for test_node in nodes {
                if before == test_node.name {
                    // SAFETY:
                    //
                    // This node is protected by the spinlock and will not be referenced by itself.
                    let link_mut = unsafe { link.get_mut() };
                    link_mut.node = Some(test_node);

                    // SAFETY:
                    //
                    // This function and all related functions are controlled by a spinlock.
                    let runtime_desc = unsafe { test_node.runtime_desc.get_mut() };
                    runtime_desc.after_list.push_back(link);
                    continue 'outer;
                }
            }

            unreachable!("invalid execution DAG: missing node name {}", before);
        }
    }
}

/// Representation of a task or stage in the initialization graph.
#[derive(Debug)]
pub struct InitGraphNode<'a> {
    /// The name of the node.
    name: &'a str,

    /// The state of the [`InitGraphNode`].
    state: AtomicU8,

    /// An intrusive link.
    link: ControlledModificationCell<Link<'a>>,

    /// The compile representation of the [`InitGraphNode`] from which the runtime representation
    /// will be derived.
    compile_time_desc: CompileTimeDesc<'a>,

    /// The runtime representation of the [`InitGraphNode`].
    runtime_desc: ControlledModificationCell<RuntimeDesc<'a>>,
}

impl<'a> InitGraphNode<'a> {
    #[doc(hidden)]
    #[expect(clippy::too_many_arguments)]
    pub const fn new_task<const R: usize, const D: usize, const A: usize, const B: usize>(
        name: &'a str,
        func: fn(),
        active: bool,
        requires: &'a [&'a str; R],
        requires_links: &'a [ControlledModificationCell<Link<'a>>; R],
        dependents: &'a [&'a str; D],
        dependents_links: &'a [ControlledModificationCell<Link<'a>>; D],
        after: &'a [&'a str; A],
        after_links: &'a [ControlledModificationCell<Link<'a>>; A],
        before: &'a [&'a str; B],
        before_links: &'a [ControlledModificationCell<Link<'a>>; B],
    ) -> Self {
        let state = if active {
            State::Active
        } else {
            State::Inactive
        };
        let state_val = Self::state_to_u8(state);

        Self {
            name,
            state: AtomicU8::new(state_val),
            link: ControlledModificationCell::new(Link::empty()),
            compile_time_desc: CompileTimeDesc::Task {
                func,
                requires,
                dependents,
                after,
                before,
            },
            runtime_desc: ControlledModificationCell::new(RuntimeDesc {
                requires_list: LinkList::new(),
                after_list: LinkList::new(),

                requires_links,
                dependents_links,
                after_links,
                before_links,
            }),
        }
    }

    #[doc(hidden)]
    pub const fn new_stage(name: &'a str, active: bool) -> Self {
        let state = if active {
            State::Active
        } else {
            State::Inactive
        };
        let state_val = Self::state_to_u8(state);

        Self {
            name,
            state: AtomicU8::new(state_val),
            link: ControlledModificationCell::new(Link::empty()),
            compile_time_desc: CompileTimeDesc::Stage,
            runtime_desc: ControlledModificationCell::new(RuntimeDesc {
                requires_list: LinkList::new(),
                after_list: LinkList::new(),

                requires_links: &[],
                dependents_links: &[],
                after_links: &[],
                before_links: &[],
            }),
        }
    }

    /// Activates this [`InitGraphNode`], thereby signalling that it is valid for processing.
    pub fn activate(&self) {
        self.set_state(State::Active);
    }

    /// Deactivates this [`InitGraphNode`], thereby signalling that it is not valid for processing.
    pub fn deactivate(&self) {
        self.set_state(State::Inactive);
    }

    /// Returns the current [`State`] of this [`InitGraphNode`].
    fn state(&self) -> State {
        let val = self.state.load(Ordering::Relaxed);
        match val {
            0 => State::Inactive,
            1 => State::Active,
            2 => State::Wanted,
            3 => State::Done,
            _ => unreachable!(),
        }
    }

    /// Sets the [`State`] of this [`InitGraphNode`].
    fn set_state(&self, state: State) {
        self.state
            .store(Self::state_to_u8(state), Ordering::Relaxed);
    }

    /// Converts the provided [`State`] into its [`u8`] value.
    const fn state_to_u8(state: State) -> u8 {
        match state {
            State::Inactive => 0,
            State::Active => 1,
            State::Wanted => 2,
            State::Done => 3,
        }
    }

    /// Executes the function associated with this [`InitGraphNode`].
    fn execute(&self) {
        match self.compile_time_desc {
            CompileTimeDesc::Stage => {}
            CompileTimeDesc::Task {
                func,
                requires: _,
                dependents: _,
                after: _,
                before: _,
            } => func(),
        }
    }

    /// Returns an [`Iterator`] over the nodes that this [`InitGraphNode`] requires before
    /// executing itself.
    fn requires(&self) -> LinkIter<'a> {
        self.runtime_desc.get().requires_list.iter()
    }

    /// Returns an [`Iterator`] over the nodes that this [`InitGraphNode`] wants to run after if in
    /// the same [`run()`] execution.
    fn after(&self) -> LinkIter<'a> {
        self.runtime_desc.get().after_list.iter()
    }
}

/// The information that represents the node at compile time.
///
/// This is what the runtime representation is derived from.
#[derive(Debug)]
enum CompileTimeDesc<'a> {
    /// A sychronization point for other [`InitGraphNode`]s.
    Stage,
    /// A task or operation that should be performed.
    Task {
        /// The function that should be performed.
        func: fn(),

        /// [`InitGraphNode`]s that this [`InitGraphNode`] requires the successful execution of before
        /// executing itself.
        requires: &'a [&'a str],

        /// [`InitGraphNode`]s that require the successful execution of this [`InitGraphNode`].
        dependents: &'a [&'a str],

        /// [`InitGraphNode`]s that this [`InitGraphNode`] will run after, if they exist in the
        /// [`InitGraphNode`] tree.
        after: &'a [&'a str],

        /// [`InitGraphNode`]s that this [`InitGraphNode`] will run before, if they exist in the
        /// [`InitGraphNode`] tree.
        before: &'a [&'a str],
    },
}

/// State for the [`InitGraphNode`] that is constructed at runtime.
#[derive(Debug)]
struct RuntimeDesc<'a> {
    /// List of all nodes that this [`InitGraphNode`] requires before executing itself.
    requires_list: LinkList<'a>,
    /// List of all nodes that this [`InitGraphNode`] will run after, if they exist in the same
    /// [`run()`] execution.
    after_list: LinkList<'a>,

    // Storage.
    #[expect(clippy::missing_docs_in_private_items)]
    requires_links: &'a [ControlledModificationCell<Link<'a>>],
    #[expect(clippy::missing_docs_in_private_items)]
    dependents_links: &'a [ControlledModificationCell<Link<'a>>],
    #[expect(clippy::missing_docs_in_private_items)]
    after_links: &'a [ControlledModificationCell<Link<'a>>],
    #[expect(clippy::missing_docs_in_private_items)]
    before_links: &'a [ControlledModificationCell<Link<'a>>],
}

/// The execution state of an [`InitGraphNode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// The [`InitGraphNode`] is not active and is not considered in initialization graph
    /// calculations.
    Inactive,
    /// The [`InitGraphNode`] is active and considered in initialization graph calculations.
    Active,
    /// The [`InitGraphNode`] should be executed.
    Wanted,
    /// The [`InitGraphNode`] has been executed.
    Done,
}

/// A single chain in the [`InitGraphNode`] [`LinkList`].
#[doc(hidden)]
#[derive(Debug)]
pub struct Link<'a> {
    /// The associated [`InitGraphNode`].
    node: Option<&'a InitGraphNode<'a>>,
    /// The next [`Link`] in the linked list.
    next: Option<&'a ControlledModificationCell<Link<'a>>>,
}

impl<'a> Link<'a> {
    /// Constructs an empty [`Link`].
    #[doc(hidden)]
    pub const fn empty() -> Self {
        Self {
            node: None,
            next: None,
        }
    }

    /// The [`InitGraphNode`] associated with this [`Link`].
    const fn node(&self) -> &'a InitGraphNode<'a> {
        self.node.unwrap()
    }
}

/// A singly linked list of [`InitGraphNode`]s.
#[derive(Debug)]
struct LinkList<'a> {
    /// The start of the [`LinkList`].
    head: Option<&'a ControlledModificationCell<Link<'a>>>,
    /// The last node of the [`LinkList`].
    tail: Option<&'a ControlledModificationCell<Link<'a>>>,
}

impl<'a> LinkList<'a> {
    /// Constructs an empty [`LinkList`].
    const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    /// Retrieves the first node from the start of this [`LinkList`].
    fn pop_front(&mut self) -> Option<&'a ControlledModificationCell<Link<'a>>> {
        let head = self.head?;
        self.head = head.get().next;
        if self.head.is_none() {
            self.tail = None;
        }

        Some(head)
    }

    /// Places the provided `link` at the end of this [`LinkList`].
    fn push_back(&mut self, link: &'a ControlledModificationCell<Link<'a>>) {
        // SAFETY:
        //
        // The user does not have any other links active.
        let link_mut = unsafe { link.get_mut() };
        link_mut.next = None;

        if self.head.is_none() {
            self.head = Some(link);
        }

        if let Some(tail) = self.tail {
            // SAFETY:
            //
            // The user does not have any other links active.
            let tail_mut = unsafe { tail.get_mut() };
            tail_mut.next = Some(link);
        }

        self.tail = Some(link);
    }

    /// Returns an [`Iterator`] over the [`InitGraphNode`]s in this [`LinkList`].
    fn iter(&self) -> LinkIter<'a> {
        LinkIter {
            current_link: self.head,
        }
    }
}

/// An [`Iterator`] over a singly linked list of [`InitGraphNode`]s.
struct LinkIter<'a> {
    /// The current [`Link`] in the list.
    current_link: Option<&'a ControlledModificationCell<Link<'a>>>,
}

impl<'a> Iterator for LinkIter<'a> {
    type Item = &'a InitGraphNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current_link = self.current_link?.get();
        self.current_link = current_link.next;
        current_link.node
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! make_task {
    (
        $name:expr,
        func = $func:expr,
        active = $active:expr,
        requires = $requires:expr,
        dependents = $dependents:expr,
        after = $after:expr,
        before = $before:expr
    ) => {{
        static REQUIRES_LINKS: [::sync::ControlledModificationCell<$crate::initgraph::Link>;
            const { <[&'static str]>::len($requires) }] =
            [const { ::sync::ControlledModificationCell::new($crate::initgraph::Link::empty()) };
                <[&'static str]>::len($requires)];
        static DEPENDENTS_LINKS: [::sync::ControlledModificationCell<$crate::initgraph::Link>;
            const { <[&'static str]>::len($dependents) }] =
            [const { ::sync::ControlledModificationCell::new($crate::initgraph::Link::empty()) };
                <[&'static str]>::len($dependents)];
        static AFTER_LINKS: [::sync::ControlledModificationCell<$crate::initgraph::Link>; const {
            <[&'static str]>::len($after)
        }] = [const { ::sync::ControlledModificationCell::new($crate::initgraph::Link::empty()) };
            <[&'static str]>::len($after)];
        static BEFORE_LINKS: [::sync::ControlledModificationCell<$crate::initgraph::Link>;
            const { <[&'static str]>::len($before) }] =
            [const { ::sync::ControlledModificationCell::new($crate::initgraph::Link::empty()) };
                <[&'static str]>::len($before)];
        $crate::initgraph::InitGraphNode::new_task(
            $name,
            $func,
            $active,
            $requires,
            &REQUIRES_LINKS,
            $dependents,
            &DEPENDENTS_LINKS,
            $after,
            &AFTER_LINKS,
            $before,
            &BEFORE_LINKS,
        )
    }};
    (
        $name:expr,
        func = $func:expr,
        requires = $requires:expr,
        dependents = $dependents:expr,
        after = $after:expr,
        before = $before:expr
    ) => {
        $crate::make_task!(
            $name,
            func = $func,
            active = true,
            requires = $requires,
            dependents = $dependents,
            after = $after,
            before = $before
        )
    };
}

/// Defines a new [`InitGraphNode`] task.
#[macro_export]
macro_rules! define_task {
    (
        $static_name:ident,
        $name:expr,
        func = $func:expr,
        inactive,
        requires = $requires:expr,
        dependents = $dependents:expr,
        after = $after:expr,
        before = $before:expr
    ) => {
        #[unsafe(link_section = ".init_node")]
        static $static_name: $crate::initgraph::InitGraphNode = $crate::make_task!(
            $name,
            func = $func,
            active = false,
            requires = $requires,
            dependents = $dependents,
            after = $after,
            before = $before
        );
    };
    (
        $static_name:ident,
        $name:expr,
        func = $func:expr,
        active,
        requires = $requires:expr,
        dependents = $dependents:expr,
        after = $after:expr,
        before = $before:expr
    ) => {
        #[unsafe(link_section = ".init_node")]
        static $static_name: $crate::initgraph::InitGraphNode = $crate::make_task!(
            $name,
            func = $func,
            active = true,
            requires = $requires,
            dependents = $dependents,
            after = $after,
            before = $before
        );
    };
}

/// Defines a new [`InitGraphNode`] stage.
#[macro_export]
macro_rules! define_stage {
    ($static_name:ident, $name:expr, active) => {
        #[unsafe(link_section = ".init_node")]
        static $static_name: $crate::initgraph::InitGraphNode =
            $crate::initgraph::InitGraphNode::new_stage($name, true);
    };
    ($static_name:ident, $name:expr, inactive) => {
        #[unsafe(link_section = ".init_node")]
        static $static_name: $crate::initgraph::InitGraphNode =
            $crate::initgraph::InitGraphNode::new_stage($name, false);
    };
}

#[cfg(test)]
mod test {
    use super::{InitGraphNode, initialize};

    fn test() {}

    #[test]
    fn forward_links_resolve() {
        const A: InitGraphNode = make_task!(
            "A",
            func = test,
            requires = &["B"],
            dependents = &[],
            after = &["B"],
            before = &[]
        );

        const B: InitGraphNode = make_task!(
            "B",
            func = test,
            requires = &[],
            dependents = &[],
            after = &[],
            before = &[]
        );

        static AB: [InitGraphNode; 2] = [A, B];

        initialize(&AB);

        assert_eq!(AB[0].requires().count(), 1);
        assert_eq!(AB[1].requires().count(), 0);

        assert_eq!(AB[0].after().count(), 1);
        assert_eq!(AB[1].after().count(), 0);
    }

    #[test]
    fn reverse_links_resolve() {
        const A: InitGraphNode = make_task!(
            "A",
            func = test,
            requires = &[],
            dependents = &["B"],
            after = &[],
            before = &["B"]
        );

        const B: InitGraphNode = make_task!(
            "B",
            func = test,
            requires = &[],
            dependents = &[],
            after = &[],
            before = &[]
        );

        static AB: [InitGraphNode; 2] = [A, B];

        initialize(&AB);

        assert_eq!(AB[0].requires().count(), 0);
        assert_eq!(AB[1].requires().count(), 1);

        assert_eq!(AB[0].after().count(), 0);
        assert_eq!(AB[1].after().count(), 1);
    }

    #[test]
    #[should_panic]
    fn duplicate_nodes_panic() {
        const A0: InitGraphNode = make_task!(
            "A",
            func = test,
            requires = &[],
            dependents = &[],
            after = &[],
            before = &[]
        );

        const A1: InitGraphNode = make_task!(
            "A",
            func = test,
            requires = &[],
            dependents = &[],
            after = &[],
            before = &[]
        );

        static AA: [InitGraphNode; 2] = [A0, A1];

        initialize(&AA);
    }
}

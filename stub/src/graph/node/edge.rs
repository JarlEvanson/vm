//! Implementation of the [`GraphNode`][gn] dependency [`Edge`]s.
//!
//! [gn]: super::GraphNode

use core::sync::atomic::AtomicBool;

    /// A dependency relationship.
#[derive(Debug)]
pub struct Edge {
    /// If `true`, the relationship indicated by this [`Edge`] is enabled and thus is considered in
    /// graph computations.
    enabled: AtomicBool,
}

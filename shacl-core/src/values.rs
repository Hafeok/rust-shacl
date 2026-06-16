//! Value nodes (§5, §6.8, `REQ-RPT-1`) — the nodes a shape's constraints are evaluated against.
//!
//! This is the §11.4 `value_nodes(shape, focus)` entry point: the single place the engine turns a
//! (shape, focus node) pair into the set of value nodes that every component validator then inspects.
//! For a node shape that set is just the focus node; for a property shape it is the value nodes
//! reached from the focus along `sh:path` (§4), deduplicated into a set (`REQ-PATH-8`).

use crate::graph::RdfGraph;
use crate::path::reach;
use shacl_model::shape::Shape;
use shacl_model::term::Term;

/// Compute the value nodes of `shape` for `focus` (`REQ-RPT-1`, §6.8).
///
/// - **Node shape** → `{ focus }` (the focus node itself).
/// - **Property shape** → `{ o | (focus, o) ∈ ⟦path⟧ }`, the path's value nodes (§4.1).
///
/// The result is deduplicated and order-stable (insertion order from [`reach`]); validators receive
/// it as a slice and never touch path evaluation themselves (§11.3).
#[must_use]
pub fn value_nodes<G: RdfGraph + ?Sized>(graph: &G, shape: &Shape, focus: &Term) -> Vec<Term> {
    match shape {
        Shape::Node(_) => vec![focus.clone()],
        Shape::Property(p) => reach(graph, focus, &p.path).into_iter().collect(),
    }
}

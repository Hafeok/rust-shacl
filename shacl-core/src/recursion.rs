//! Shapes-graph recursion guard (§9.1, ADR-002). v1 ships *reject-on-recursion*: if the shape
//! reference graph contains a cycle, the shapes graph is rejected rather than validated (the spec
//! leaves recursion semantics undefined and explicitly sanctions non-support).
//!
//! The reference graph has an edge `S → T` whenever shape `S` references shape `T` through a
//! shape-valued parameter (`sh:node`/`sh:property`/`sh:not`/`sh:and`/`sh:or`/`sh:xone`/
//! `sh:qualifiedValueShape`/`sh:memberShape`). [`shape_ref_cycle`] returns a shape that participates
//! in a cycle (via a white/grey/black DFS that flags back edges), or `None` if the graph is acyclic.

use crate::engine::term_to_shape_id;
use shacl_model::shape::{Constraint, Shape, ShapeId};
use std::collections::{HashMap, HashSet};

const SH: &str = "http://www.w3.org/ns/shacl#";

/// The shape-valued parameter predicates whose values are references to other shapes.
const SHAPE_PARAMS: &[&str] = &[
    "node",
    "property",
    "not",
    "and",
    "or",
    "xone",
    "qualifiedValueShape",
    "memberShape",
    "someValue",
    "reifierShape",
    "nodeByExpression",
];

/// The shape ids referenced by `shape` through any shape-valued parameter.
fn shape_refs(shape: &Shape) -> Vec<ShapeId> {
    let mut refs = Vec::new();
    for c in shape.constraints() {
        collect_refs(c, &mut refs);
    }
    refs
}

fn collect_refs(c: &Constraint, refs: &mut Vec<ShapeId>) {
    for (pred, value) in &c.params {
        let Some(local) = pred.as_str().strip_prefix(SH) else {
            continue;
        };
        if SHAPE_PARAMS.contains(&local) {
            if let Some(id) = term_to_shape_id(value) {
                refs.push(id);
            }
        }
    }
}

/// Return a [`ShapeId`] participating in a reference cycle, or `None` if the shapes graph is acyclic
/// (and therefore safe to validate). A self-reference counts as a cycle.
#[must_use]
pub fn shape_ref_cycle(shapes: &[Shape]) -> Option<ShapeId> {
    // Adjacency restricted to edges whose target is a known shape (dangling refs can't form cycles).
    let known: HashSet<&ShapeId> = shapes.iter().map(Shape::id).collect();
    let adj: HashMap<ShapeId, Vec<ShapeId>> = shapes
        .iter()
        .map(|s| {
            let edges = shape_refs(s)
                .into_iter()
                .filter(|t| known.contains(t))
                .collect();
            (s.id().clone(), edges)
        })
        .collect();

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Grey,
        Black,
    }
    let mut state: HashMap<ShapeId, Mark> = HashMap::new();

    // Iterative DFS with an explicit stack; a grey target on the stack is a back edge → cycle.
    for start in adj.keys() {
        if state.contains_key(start) {
            continue;
        }
        // Stack frames: (node, index of next child to visit).
        let mut stack: Vec<(ShapeId, usize)> = vec![(start.clone(), 0)];
        state.insert(start.clone(), Mark::Grey);
        while let Some((node, idx)) = stack.last().cloned() {
            let children = &adj[&node];
            if idx < children.len() {
                stack.last_mut().unwrap().1 += 1;
                let child = &children[idx];
                match state.get(child) {
                    Some(Mark::Grey) => return Some(child.clone()), // back edge → cycle
                    Some(Mark::Black) => {}
                    None => {
                        state.insert(child.clone(), Mark::Grey);
                        stack.push((child.clone(), 0));
                    }
                }
            } else {
                state.insert(node, Mark::Black);
                stack.pop();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use shacl_model::shape::{NodeShape, Severity};
    use shacl_model::term::{NamedNode, Term};

    fn sh(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{SH}{local}"))
    }
    fn id(name: &str) -> ShapeId {
        ShapeId::Named(NamedNode::new_unchecked(format!(
            "http://example.com/{name}"
        )))
    }

    /// A node shape `name` with an `sh:node` reference to each of `refs`.
    fn shape_with_refs(name: &str, refs: &[&str]) -> Shape {
        let params = refs
            .iter()
            .map(|r| {
                (
                    sh("node"),
                    Term::NamedNode(NamedNode::new_unchecked(format!("http://example.com/{r}"))),
                )
            })
            .collect();
        Shape::Node(NodeShape {
            id: id(name),
            targets: vec![],
            constraints: vec![Constraint {
                component: sh("NodeConstraintComponent"),
                params,
                severity: None,
                deactivated: false,
            }],
            severity: Severity::default(),
            deactivated: false,
        })
    }

    #[test]
    fn acyclic_graph_has_no_cycle() {
        let shapes = vec![
            shape_with_refs("A", &["B"]),
            shape_with_refs("B", &["C"]),
            shape_with_refs("C", &[]),
        ];
        assert!(shape_ref_cycle(&shapes).is_none());
    }

    #[test]
    fn direct_cycle_detected() {
        let shapes = vec![shape_with_refs("A", &["B"]), shape_with_refs("B", &["A"])];
        assert!(shape_ref_cycle(&shapes).is_some());
    }

    #[test]
    fn self_reference_is_a_cycle() {
        let shapes = vec![shape_with_refs("A", &["A"])];
        assert_eq!(shape_ref_cycle(&shapes), Some(id("A")));
    }

    #[test]
    fn dangling_reference_is_not_a_cycle() {
        // A → B but B is not in the shapes set.
        let shapes = vec![shape_with_refs("A", &["B"])];
        assert!(shape_ref_cycle(&shapes).is_none());
    }
}

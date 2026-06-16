//! Target declarations (§3.1.3, `REQ-TGT-1..7`). A target produces the focus nodes for a shape.

use crate::term::{NamedNode, Term};

/// A single target declaration on a shape.
#[derive(Debug, Clone)]
pub enum Target {
    /// `sh:targetNode` — explicit node(s). `REQ-TGT-1`.
    Node(Term),
    /// `sh:targetClass` — SHACL instances of the class. `REQ-TGT-2`.
    Class(NamedNode),
    /// Implicit class target: shape is also an `rdfs:Class`/`sh:ShapeClass`. `REQ-TGT-3`.
    ImplicitClass(NamedNode),
    /// `sh:targetSubjectsOf` — subjects of triples with this predicate. `REQ-TGT-4`.
    SubjectsOf(NamedNode),
    /// `sh:targetObjectsOf` — objects of triples with this predicate. `REQ-TGT-4`.
    ObjectsOf(NamedNode),
    /// `sh:targetWhere` — nodes that conform to the inner shape. `REQ-TGT-5`, ADR-007
    /// (naive iteration in v1).
    Where(crate::shape::ShapeId),
    /// `sh:shape` — declared in the *data* graph, linking a node to this shape. `REQ-TGT-6`.
    /// Marker only; resolution reads the data graph, unlike the others.
    ExplicitShape,
}

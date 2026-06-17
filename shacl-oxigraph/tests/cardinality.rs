//! Cardinality component tests (§7.2): `sh:minCount` / `sh:maxCount` over [`MemGraph`]. Verifies the
//! count is over distinct value nodes, that violations carry no `sh:value`, and the boundary cases
//! (`REQ-MINCOUNT-1..3`, `CMP-MAXCOUNT`).

use shacl_core::validate;
use shacl_model::path::Path;
use shacl_model::shape::{Constraint, PropertyShape, Shape, ShapeId};
use shacl_model::target::Target;
use shacl_model::term::{Literal, NamedNode, Term};
use shacl_oxigraph::mem::MemGraph;

const EX: &str = "http://example.com/";
const SH: &str = "http://www.w3.org/ns/shacl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{EX}{local}"))
}
fn sh(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{local}"))
}
fn node(local: &str) -> Term {
    Term::NamedNode(iri(local))
}
fn int(n: &str) -> Term {
    Term::Literal(Literal::new_typed_literal(
        n,
        NamedNode::new_unchecked(format!("{XSD}integer")),
    ))
}

/// Property shape on `ex:p` targeting `ex:a`, with one cardinality constraint.
fn shape(component: &str, param: &str, count: &str) -> Shape {
    Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("Shape")),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![Constraint {
            component: sh(component),
            params: vec![(sh(param), int(count))],
            severity: None,
            deactivated: false,
        }],
        severity: Default::default(),
        deactivated: false,
    })
}

#[test]
fn min_count_violation_when_too_few() {
    // ex:a has zero ex:p values; sh:minCount 1 → one result, no sh:value (violation is absence).
    let g = MemGraph::new();
    let report = validate(&g, &[shape("MinCountConstraintComponent", "minCount", "1")]);
    assert_eq!(report.results.len(), 1, "0 < 1 violates minCount");
    let r = &report.results[0];
    assert_eq!(r.value, None, "minCount violation carries no sh:value");
    assert_eq!(
        r.source_constraint_component,
        sh("MinCountConstraintComponent")
    );
    assert_eq!(r.focus_node, node("a"));
    assert_eq!(r.result_path.as_deref(), Some("<http://example.com/p>"));
}

#[test]
fn min_count_satisfied_at_boundary() {
    // One value node, sh:minCount 1 → conforms (not strictly less than).
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    let report = validate(&g, &[shape("MinCountConstraintComponent", "minCount", "1")]);
    assert!(
        report.conforms(),
        "1 value node satisfies minCount 1: {report:?}"
    );
}

#[test]
fn min_count_zero_never_violates() {
    // REQ-MINCOUNT-3: sh:minCount 0 with no values still conforms.
    let g = MemGraph::new();
    let report = validate(&g, &[shape("MinCountConstraintComponent", "minCount", "0")]);
    assert!(report.conforms(), "minCount 0 never produces a result");
}

#[test]
fn max_count_violation_when_too_many() {
    // Two distinct value nodes, sh:maxCount 1 → one result, no sh:value.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("a"), iri("p"), node("c"));
    let report = validate(&g, &[shape("MaxCountConstraintComponent", "maxCount", "1")]);
    assert_eq!(report.results.len(), 1, "2 > 1 violates maxCount");
    assert_eq!(report.results[0].value, None);
    assert_eq!(
        report.results[0].source_constraint_component,
        sh("MaxCountConstraintComponent")
    );
}

#[test]
fn max_count_satisfied_at_boundary() {
    // Exactly maxCount value nodes → conforms.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    let report = validate(&g, &[shape("MaxCountConstraintComponent", "maxCount", "1")]);
    assert!(
        report.conforms(),
        "1 value node satisfies maxCount 1: {report:?}"
    );
}

#[test]
fn count_is_over_distinct_value_nodes() {
    // REQ-MINCOUNT-2: a duplicate triple yields one distinct value node, so maxCount 1 conforms.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("a"), iri("p"), node("b")); // duplicate
    let report = validate(&g, &[shape("MaxCountConstraintComponent", "maxCount", "1")]);
    assert!(
        report.conforms(),
        "duplicate value node counts once: {report:?}"
    );
}

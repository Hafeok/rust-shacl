//! Phase 8c property-pair component tests (§7.6) over [`MemGraph`]. The shape's path is `ex:p`; the
//! paired predicate is `ex:q`; the focus is `ex:a`.

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

fn shape(component: &str, param: &str) -> Shape {
    Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("Shape")),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![Constraint {
            component: sh(component),
            params: vec![(sh(param), Term::NamedNode(iri("q")))],
            severity: None,
            deactivated: false,
        }],
        severity: Default::default(),
        deactivated: false,
    })
}

fn run(g: &MemGraph, component: &str, param: &str) -> usize {
    validate(g, &[shape(component, param)]).results.len()
}

#[test]
fn equals_symmetric_difference() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("x"));
    g.insert(node("a"), iri("q"), node("x"));
    assert_eq!(
        run(&g, "EqualsConstraintComponent", "equals"),
        0,
        "equal sets conform"
    );

    g.insert(node("a"), iri("p"), node("y")); // y in p but not q
    g.insert(node("a"), iri("q"), node("z")); // z in q but not p
    assert_eq!(
        run(&g, "EqualsConstraintComponent", "equals"),
        2,
        "y and z are the sym. diff."
    );
}

#[test]
fn disjoint() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("x"));
    g.insert(node("a"), iri("q"), node("y"));
    assert_eq!(
        run(&g, "DisjointConstraintComponent", "disjoint"),
        0,
        "no overlap conforms"
    );

    g.insert(node("a"), iri("q"), node("x")); // now x is shared
    assert_eq!(
        run(&g, "DisjointConstraintComponent", "disjoint"),
        1,
        "shared x violates"
    );
}

#[test]
fn subset_of() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("x"));
    g.insert(node("a"), iri("q"), node("x"));
    g.insert(node("a"), iri("q"), node("y"));
    assert_eq!(
        run(&g, "SubsetOfConstraintComponent", "subsetOf"),
        0,
        "value set is a subset"
    );

    g.insert(node("a"), iri("p"), node("z")); // z not in q
    assert_eq!(
        run(&g, "SubsetOfConstraintComponent", "subsetOf"),
        1,
        "z breaks subset"
    );
}

#[test]
fn less_than() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), int("3"));
    g.insert(node("a"), iri("q"), int("5"));
    assert_eq!(
        run(&g, "LessThanConstraintComponent", "lessThan"),
        0,
        "3 < 5"
    );

    g.insert(node("a"), iri("q"), int("3")); // now p has 3, q has {5,3}; 3 < 3 is false
    assert_eq!(
        run(&g, "LessThanConstraintComponent", "lessThan"),
        1,
        "3 not < 3"
    );
    // but lessThanOrEquals tolerates equality
    assert_eq!(
        run(
            &g,
            "LessThanOrEqualsConstraintComponent",
            "lessThanOrEquals"
        ),
        0,
        "3 <= 3"
    );
}

#[test]
fn less_than_incomparable_violates() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), int("3"));
    g.insert(node("a"), iri("q"), node("notanumber")); // IRI vs int → type error
    assert_eq!(run(&g, "LessThanConstraintComponent", "lessThan"), 1);
}

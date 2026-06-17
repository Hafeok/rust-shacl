//! Phase 8d rdf:List component tests (§7.5) over [`MemGraph`]. A list `(m1 m2 …)` is encoded with
//! rdf:first/rdf:rest cells terminating in rdf:nil; the value node is the list head.

use shacl_core::validate;
use shacl_model::path::Path;
use shacl_model::shape::{Constraint, PropertyShape, Shape, ShapeId};
use shacl_model::target::Target;
use shacl_model::term::{BlankNode, Literal, NamedNode, Term};
use shacl_oxigraph::mem::MemGraph;

const EX: &str = "http://example.com/";
const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{EX}{local}"))
}
fn sh(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{local}"))
}
fn rdf(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{RDF}{local}"))
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

/// Materialise `members` as an rdf:List in `g` and return its head term.
fn make_list(g: &mut MemGraph, members: &[Term]) -> Term {
    let nil = Term::NamedNode(rdf("nil"));
    let mut head = nil.clone();
    for (i, m) in members.iter().enumerate().rev() {
        let cell = Term::BlankNode(BlankNode::new_unchecked(format!("cell{i}")));
        g.insert(cell.clone(), rdf("first"), m.clone());
        g.insert(cell.clone(), rdf("rest"), head);
        head = cell;
    }
    head
}

fn shape(component: &str, params: Vec<(NamedNode, Term)>) -> Shape {
    Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("Shape")),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![Constraint {
            component: sh(component),
            params,
            severity: None,
            deactivated: false,
        }],
        severity: Default::default(),
        deactivated: false,
    })
}

fn run(g: &MemGraph, component: &str, params: Vec<(NamedNode, Term)>) -> usize {
    validate(g, &[shape(component, params)]).results.len()
}

#[test]
fn list_length_bounds() {
    let mut g = MemGraph::new();
    let list = make_list(&mut g, &[int("1"), int("2"), int("3")]); // length 3
    g.insert(node("a"), iri("p"), list);

    assert_eq!(
        run(
            &g,
            "MinListLengthConstraintComponent",
            vec![(sh("minListLength"), int("3"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "MinListLengthConstraintComponent",
            vec![(sh("minListLength"), int("4"))]
        ),
        1
    );
    assert_eq!(
        run(
            &g,
            "MaxListLengthConstraintComponent",
            vec![(sh("maxListLength"), int("3"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "MaxListLengthConstraintComponent",
            vec![(sh("maxListLength"), int("2"))]
        ),
        1
    );
}

#[test]
fn non_list_value_violates_length() {
    // A plain IRI value node is not a well-formed list → minListLength violates.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("notalist"));
    assert_eq!(
        run(
            &g,
            "MinListLengthConstraintComponent",
            vec![(sh("minListLength"), int("0"))]
        ),
        1
    );
}

#[test]
fn unique_members() {
    let mut g = MemGraph::new();
    let dup = make_list(&mut g, &[int("1"), int("2"), int("1")]); // 1 repeats
    g.insert(node("a"), iri("p"), dup);
    assert_eq!(
        run(
            &g,
            "UniqueMembersConstraintComponent",
            vec![(sh("uniqueMembers"), int_bool(true))]
        ),
        1
    );

    let mut g2 = MemGraph::new();
    let uniq = make_list(&mut g2, &[int("1"), int("2"), int("3")]);
    g2.insert(node("a"), iri("p"), uniq);
    assert_eq!(
        run(
            &g2,
            "UniqueMembersConstraintComponent",
            vec![(sh("uniqueMembers"), int_bool(true))]
        ),
        0
    );
}

fn int_bool(b: bool) -> Term {
    Term::Literal(Literal::new_typed_literal(
        if b { "true" } else { "false" },
        NamedNode::new_unchecked(format!("{XSD}boolean")),
    ))
}

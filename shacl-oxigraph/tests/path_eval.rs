//! Path-evaluation conformance against a real backend (§4, `REQ-PATH-1..8`), exercised over
//! [`MemGraph`]. shacl-core cannot depend on a backend (REQ-ARCH-1), so the integration tests for
//! its path evaluator live here, where `MemGraph` (build step 3, §11.5) provides the `RdfGraph`.

use std::collections::HashSet;

use shacl_core::path::reach;
use shacl_core::value_nodes;
use shacl_model::path::Path;
use shacl_model::shape::{NodeShape, PropertyShape, Shape, ShapeId};
use shacl_model::term::{NamedNode, Term};
use shacl_oxigraph::mem::MemGraph;

const EX: &str = "http://example.com/";

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{EX}{local}"))
}
fn node(local: &str) -> Term {
    Term::NamedNode(iri(local))
}
fn pred(local: &str) -> Path {
    Path::Predicate(iri(local))
}
fn set(terms: impl IntoIterator<Item = Term>) -> HashSet<Term> {
    terms.into_iter().collect()
}

/// Graph: a -p-> b -p-> c -p-> a (a `p`-cycle), plus a -q-> x and b -q-> y.
fn fixture() -> MemGraph {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("b"), iri("p"), node("c"));
    g.insert(node("c"), iri("p"), node("a"));
    g.insert(node("a"), iri("q"), node("x"));
    g.insert(node("b"), iri("q"), node("y"));
    g
}

#[test]
fn predicate_path() {
    // REQ-PATH-1: ⟦p⟧(a) = objects of (a, p, *).
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &pred("p"))
        .into_iter()
        .collect();
    assert_eq!(got, set([node("b")]));
}

#[test]
fn inverse_predicate_path() {
    // REQ-PATH-4: ⟦^p⟧(b) = subjects s with (s, p, b).
    let inv = Path::Inverse(Box::new(pred("p")));
    let got: HashSet<Term> = reach(&fixture(), &node("b"), &inv).into_iter().collect();
    assert_eq!(got, set([node("a")]));
}

#[test]
fn sequence_path() {
    // REQ-PATH-2: ⟦p/p⟧(a) = {c}.
    let seq = Path::Sequence(vec![pred("p"), pred("p")]);
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &seq).into_iter().collect();
    assert_eq!(got, set([node("c")]));
}

#[test]
fn alternative_path() {
    // REQ-PATH-3: ⟦p|q⟧(a) = {b, x}.
    let alt = Path::Alternative(vec![pred("p"), pred("q")]);
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &alt).into_iter().collect();
    assert_eq!(got, set([node("b"), node("x")]));
}

#[test]
fn zero_or_more_terminates_on_cyclic_data() {
    // REQ-PATH-5/7: ⟦p*⟧(a) over the a→b→c→a cycle = {a, b, c}, reached without looping forever.
    let star = Path::ZeroOrMore(Box::new(pred("p")));
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &star).into_iter().collect();
    assert_eq!(got, set([node("a"), node("b"), node("c")]));
}

#[test]
fn one_or_more_includes_start_only_via_cycle() {
    // REQ-PATH-7: ⟦p+⟧(a) includes a because a is reachable from a in ≥1 step (the cycle).
    let plus = Path::OneOrMore(Box::new(pred("p")));
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &plus).into_iter().collect();
    assert_eq!(got, set([node("a"), node("b"), node("c")]));

    // With a non-cyclic predicate, + excludes the start: ⟦q+⟧(a) = {x}.
    let plus_q = Path::OneOrMore(Box::new(pred("q")));
    let got_q: HashSet<Term> = reach(&fixture(), &node("a"), &plus_q).into_iter().collect();
    assert_eq!(got_q, set([node("x")]));
}

#[test]
fn zero_or_one_path() {
    // REQ-PATH-4: ⟦p?⟧(a) = Δ ∪ ⟦p⟧ = {a, b}.
    let opt = Path::ZeroOrOne(Box::new(pred("p")));
    let got: HashSet<Term> = reach(&fixture(), &node("a"), &opt).into_iter().collect();
    assert_eq!(got, set([node("a"), node("b")]));
}

#[test]
fn inverse_of_sequence() {
    // REQ-PATH-4: ^(p/p) from c = {a}: reverse+invert the components.
    let inv_seq = Path::Inverse(Box::new(Path::Sequence(vec![pred("p"), pred("p")])));
    let got: HashSet<Term> = reach(&fixture(), &node("c"), &inv_seq)
        .into_iter()
        .collect();
    assert_eq!(got, set([node("a")]));
}

fn dummy_id() -> ShapeId {
    ShapeId::Named(iri("Shape"))
}

#[test]
fn value_nodes_of_node_shape_is_focus() {
    // REQ-RPT-1: node shape value nodes = { focus }.
    let shape = Shape::Node(NodeShape {
        id: dummy_id(),
        targets: vec![],
        constraints: vec![],
        severity: Default::default(),
        deactivated: false,
    });
    assert_eq!(value_nodes(&fixture(), &shape, &node("a")), vec![node("a")]);
}

#[test]
fn value_nodes_of_property_shape_follows_path() {
    // REQ-RPT-1 / §6.8: property shape value nodes = path's value nodes from focus.
    let shape = Shape::Property(PropertyShape {
        id: dummy_id(),
        path: pred("p"),
        targets: vec![],
        constraints: vec![],
        severity: Default::default(),
        deactivated: false,
    });
    let got: HashSet<Term> = value_nodes(&fixture(), &shape, &node("a"))
        .into_iter()
        .collect();
    assert_eq!(got, set([node("b")]));
}

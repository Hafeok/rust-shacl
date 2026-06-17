//! Phase 8b component tests: string (§7.4), range (§7.3), membership (§7.9) over [`MemGraph`].

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
fn lit(s: &str) -> Term {
    Term::Literal(Literal::new_simple_literal(s))
}
fn typed(s: &str, dt: &str) -> Term {
    Term::Literal(Literal::new_typed_literal(
        s,
        NamedNode::new_unchecked(format!("{XSD}{dt}")),
    ))
}
fn lang(s: &str, tag: &str) -> Term {
    Term::Literal(Literal::new_language_tagged_literal_unchecked(s, tag))
}

/// Property shape on `ex:p` targeting `ex:a` with the given constraint params.
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
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    })
}

fn run(g: &MemGraph, component: &str, params: Vec<(NamedNode, Term)>) -> usize {
    validate(g, &[shape(component, params)]).results.len()
}

// ── length ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn min_max_length() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lit("abc")); // 3 chars
    assert_eq!(
        run(
            &g,
            "MinLengthConstraintComponent",
            vec![(sh("minLength"), typed("4", "integer"))]
        ),
        1
    );
    assert_eq!(
        run(
            &g,
            "MinLengthConstraintComponent",
            vec![(sh("minLength"), typed("3", "integer"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "MaxLengthConstraintComponent",
            vec![(sh("maxLength"), typed("2", "integer"))]
        ),
        1
    );
    assert_eq!(
        run(
            &g,
            "MaxLengthConstraintComponent",
            vec![(sh("maxLength"), typed("3", "integer"))]
        ),
        0
    );
}

#[test]
fn length_on_blank_node_violates() {
    let mut g = MemGraph::new();
    g.insert(
        node("a"),
        iri("p"),
        Term::BlankNode(oxrdf::BlankNode::new_unchecked("b0")),
    );
    assert_eq!(
        run(
            &g,
            "MinLengthConstraintComponent",
            vec![(sh("minLength"), typed("1", "integer"))]
        ),
        1
    );
    assert_eq!(
        run(
            &g,
            "MaxLengthConstraintComponent",
            vec![(sh("maxLength"), typed("9", "integer"))]
        ),
        1
    );
}

// ── pattern ─────────────────────────────────────────────────────────────────────────────────────

#[test]
fn pattern_match_and_flags() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lit("Hello"));
    assert_eq!(
        run(
            &g,
            "PatternConstraintComponent",
            vec![(sh("pattern"), lit("^Hello$"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "PatternConstraintComponent",
            vec![(sh("pattern"), lit("^bye"))]
        ),
        1
    );
    // case-insensitive flag
    assert_eq!(
        run(
            &g,
            "PatternConstraintComponent",
            vec![(sh("pattern"), lit("^hello$")), (sh("flags"), lit("i"))]
        ),
        0
    );
}

#[test]
fn pattern_on_iri_matches_iri_string() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("widget"));
    assert_eq!(
        run(
            &g,
            "PatternConstraintComponent",
            vec![(sh("pattern"), lit("widget$"))]
        ),
        0
    );
}

// ── singleLine ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn single_line() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lit("one\ntwo"));
    g.insert(node("a"), iri("p"), lit("oneline"));
    assert_eq!(
        run(
            &g,
            "SingleLineConstraintComponent",
            vec![(sh("singleLine"), typed("true", "boolean"))]
        ),
        1
    );
    // false → no constraint enforced
    assert_eq!(
        run(
            &g,
            "SingleLineConstraintComponent",
            vec![(sh("singleLine"), typed("false", "boolean"))]
        ),
        0
    );
}

// ── languageIn / uniqueLang ─────────────────────────────────────────────────────────────────────

#[test]
fn language_in() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lang("hi", "en"));
    g.insert(node("a"), iri("p"), lang("salut", "fr-CA"));
    g.insert(node("a"), iri("p"), lang("hallo", "de"));
    // admit en + fr (fr-CA matches fr by basic filtering); de violates → 1 result
    assert_eq!(
        run(
            &g,
            "LanguageInConstraintComponent",
            vec![(sh("languageIn"), lit("en")), (sh("languageIn"), lit("fr"))]
        ),
        1
    );
}

#[test]
fn unique_lang() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lang("color", "en"));
    g.insert(node("a"), iri("p"), lang("colour", "en")); // duplicate en
    g.insert(node("a"), iri("p"), lang("couleur", "fr"));
    assert_eq!(
        run(
            &g,
            "UniqueLangConstraintComponent",
            vec![(sh("uniqueLang"), typed("true", "boolean"))]
        ),
        1
    );
}

// ── range ───────────────────────────────────────────────────────────────────────────────────────

#[test]
fn numeric_range() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), typed("5", "integer"));
    assert_eq!(
        run(
            &g,
            "MinInclusiveConstraintComponent",
            vec![(sh("minInclusive"), typed("5", "integer"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "MinExclusiveConstraintComponent",
            vec![(sh("minExclusive"), typed("5", "integer"))]
        ),
        1
    );
    assert_eq!(
        run(
            &g,
            "MaxInclusiveConstraintComponent",
            vec![(sh("maxInclusive"), typed("5", "integer"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "MaxExclusiveConstraintComponent",
            vec![(sh("maxExclusive"), typed("5", "integer"))]
        ),
        1
    );
    // cross-type numeric: 5 (integer) vs 5.5 (decimal)
    assert_eq!(
        run(
            &g,
            "MaxInclusiveConstraintComponent",
            vec![(sh("maxInclusive"), typed("5.5", "decimal"))]
        ),
        0
    );
}

#[test]
fn range_incomparable_violates() {
    // a string value against a numeric bound is a SPARQL type error → violation.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), lit("notanumber"));
    assert_eq!(
        run(
            &g,
            "MinInclusiveConstraintComponent",
            vec![(sh("minInclusive"), typed("0", "integer"))]
        ),
        1
    );
}

// ── membership ──────────────────────────────────────────────────────────────────────────────────

#[test]
fn has_value() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("a"), iri("p"), node("c"));
    assert_eq!(
        run(
            &g,
            "HasValueConstraintComponent",
            vec![(sh("hasValue"), node("b"))]
        ),
        0
    );
    assert_eq!(
        run(
            &g,
            "HasValueConstraintComponent",
            vec![(sh("hasValue"), node("z"))]
        ),
        1
    );
}

#[test]
fn in_set() {
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("a"), iri("p"), node("x")); // not in set → 1 violation
    assert_eq!(
        run(
            &g,
            "InConstraintComponent",
            vec![(sh("in"), node("b")), (sh("in"), node("c"))]
        ),
        1
    );
}

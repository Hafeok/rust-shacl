//! Phase 11 tests (§8): the `oxigraph::Store` backend (`RdfGraph` + `SparqlGraph` with `$this`
//! pre-binding) and SPARQL-based constraints (`sh:select`).

use shacl_core::graph::{Bindings, RdfGraph, SparqlGraph};
use shacl_model::shape::{Severity, ShapeId};
use shacl_model::term::{NamedNode, Term};
use shacl_oxigraph::store::OxiStore;
use shacl_sparql::constraint::{validate_select, Outcome, SelectConstraint};
use shacl_sparql::prefixes::with_prefixes;

const EX: &str = "http://example.com/";

fn iri(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{EX}{local}"))
}
fn node(local: &str) -> Term {
    Term::NamedNode(iri(local))
}

fn data() -> OxiStore {
    OxiStore::from_turtle(
        r#"
        @prefix ex: <http://example.com/> .
        ex:alice ex:age 30 .
        ex:bob   ex:age 150 .
        "#,
    )
    .expect("valid turtle")
}

#[test]
fn rdfgraph_pattern_access() {
    let g = data();
    // (ex:alice, ex:age, *) → exactly one triple.
    let n = g
        .triples(Some(&node("alice")), Some(&iri("age")), None)
        .count();
    assert_eq!(n, 1);
    // wildcard subject for ex:age → two triples.
    assert_eq!(g.triples(None, Some(&iri("age")), None).count(), 2);
}

#[test]
fn sparql_select_with_prebound_this() {
    let g = data();
    let prebound = Bindings {
        pairs: vec![("this".to_string(), node("bob"))],
    };
    let q = with_prefixes(
        &[("ex".to_string(), EX.to_string())],
        "SELECT ?value WHERE { $this ex:age ?value }",
    );
    let solutions = g.select(&q, &prebound).expect("query ok");
    assert_eq!(solutions.len(), 1, "bob has one age");
}

#[test]
fn sparql_ask_with_prebound_this() {
    let g = data();
    let prebound = Bindings {
        pairs: vec![("this".to_string(), node("alice"))],
    };
    let q = with_prefixes(
        &[("ex".to_string(), EX.to_string())],
        "ASK { $this ex:age ?a }",
    );
    assert!(g.ask(&q, &prebound).expect("ask ok"));
}

#[test]
fn sparql_based_constraint_flags_violation() {
    // Constraint: age over 130 is a violation. bob (150) violates; alice (30) does not.
    let g = data();
    let select = with_prefixes(
        &[("ex".to_string(), EX.to_string())],
        "SELECT $this ?value WHERE { $this ex:age ?value . FILTER(?value > 130) }",
    );
    let c = SelectConstraint {
        select,
        source_shape: ShapeId::Named(iri("AgeShape")),
        severity: Severity::Violation,
        message: Some("age too large".to_string()),
    };

    match validate_select(&g, &node("bob"), None, &c) {
        Outcome::Results(rs) => {
            assert_eq!(rs.len(), 1);
            assert_eq!(rs[0].focus_node, node("bob"));
            assert_eq!(
                rs[0].value,
                Some(Term::Literal(
                    shacl_model::term::Literal::new_typed_literal(
                        "150",
                        NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
                    )
                ))
            );
            assert_eq!(rs[0].messages, vec!["age too large".to_string()]);
            assert_eq!(
                rs[0].source_constraint_component,
                NamedNode::new_unchecked("http://www.w3.org/ns/shacl#SPARQLConstraintComponent")
            );
        }
        Outcome::Failure(e) => panic!("unexpected failure: {e}"),
    }

    match validate_select(&g, &node("alice"), None, &c) {
        Outcome::Results(rs) => assert!(rs.is_empty(), "alice conforms"),
        Outcome::Failure(e) => panic!("unexpected failure: {e}"),
    }
}

#[test]
fn sparql_failure_is_distinct() {
    // A solution binding ?failure true signals a failure, not a violation.
    let g = data();
    let select = with_prefixes(
        &[("ex".to_string(), EX.to_string())],
        "SELECT $this (true AS ?failure) WHERE { $this ex:age ?value }",
    );
    let c = SelectConstraint {
        select,
        source_shape: ShapeId::Named(iri("AgeShape")),
        severity: Severity::Violation,
        message: None,
    };
    assert!(matches!(
        validate_select(&g, &node("bob"), None, &c),
        Outcome::Failure(_)
    ));
}

//! End-to-end validation against the real Product-Framework SHACL shapes (vendored from
//! `product-cli/schema/shapes/`). Proves the host-application surface: `sh:message` flows into
//! `sh:resultMessage`, and `sh:sparql` constraints run via the SPARQL backend.

use shacl_oxigraph::validate_turtle;

const SHAPES: &str = include_str!("fixtures_product_shapes.ttl");
const HOW_SHAPES: &str = include_str!("fixtures_product_how.ttl");

#[test]
fn what_shapes_parse_and_run_with_messages() {
    // An Entity missing its required pf:definition and pf:inContext (§3.1) → two violations,
    // each carrying the shape's framework-rule message.
    let data = r#"
        @prefix pf: <https://productframework.org/ns#> .
        @prefix ex: <http://example.com/> .
        ex:Order a pf:Entity .
    "#;
    let report = validate_turtle(SHAPES, data).expect("valid turtle");
    assert!(
        !report.conforms(),
        "an entity with no definition/context must fail"
    );

    // Every produced result must carry its §3.1 framework message (sh:message → sh:resultMessage).
    assert!(
        report.results.iter().all(|r| !r.messages.is_empty()),
        "results must carry framework messages: {:?}",
        report.results
    );
    assert!(
        report.results.iter().any(|r| r
            .messages
            .iter()
            .any(|m| m.contains("business-language definition"))),
        "the §3.1 definition message must appear"
    );
}

#[test]
fn conforming_what_graph_passes() {
    // A fully-specified entity in a named bounded context conforms.
    let data = r#"
        @prefix pf:   <https://productframework.org/ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex:   <http://example.com/> .
        ex:Ordering a pf:BoundedContext ; rdfs:label "Ordering" .
        ex:Order a pf:Entity ;
            pf:definition "A customer's request to purchase items." ;
            pf:inContext ex:Ordering .
    "#;
    let report = validate_turtle(SHAPES, data).expect("valid turtle");
    assert!(
        report.conforms(),
        "well-formed entity should conform: {:?}",
        report.results
    );
}

#[test]
fn how_shapes_with_sparql_constraint_parse_and_run() {
    // The How shapes include a sh:sparql trace-truth rule. It must parse and execute (not error),
    // and a work unit applying a principle with no passing verification must be flagged.
    let data = r#"
        @prefix pf: <https://productframework.org/ns#> .
        @prefix ex: <http://example.com/> .
        ex:wu a pf:WorkUnit ; pf:applies ex:p1 .
        ex:p1 a pf:Principle .
    "#;
    let report = validate_turtle(HOW_SHAPES, data).expect("how shapes parse + run");
    // The trace-truth SPARQL rule should fire (principle applied, no verification enforces it).
    assert!(
        report
            .results
            .iter()
            .any(|r| r.messages.iter().any(|m| m.contains("trace must be true"))),
        "the §5/§4.1 trace-truth SPARQL constraint must flag the unverified principle: {:?}",
        report.results
    );
}

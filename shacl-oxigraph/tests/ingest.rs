//! Phase 10 ingestion tests (§3, `REQ-ING-*`): parse real Turtle 1.2 shapes + data and validate
//! end-to-end, exercising the full pipeline (Turtle → Shape → engine → report).

use shacl_oxigraph::ingest::{parse_data, parse_shapes};

use shacl_core::validate;

const SHAPES: &str = r#"
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix ex:   <http://example.com/> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
        sh:path ex:name ;
        sh:minCount 1 ;
        sh:datatype xsd:string ;
    ] ;
    sh:property [
        sh:path ex:age ;
        sh:maxCount 1 ;
        sh:minInclusive 0 ;
    ] .
"#;

#[test]
fn parses_node_and_property_shapes() {
    let shapes = parse_shapes(SHAPES).expect("valid turtle");
    // One node shape + two (blank) property shapes.
    assert_eq!(shapes.len(), 3, "got {} shapes", shapes.len());
    let node_shapes = shapes
        .iter()
        .filter(|s| matches!(s, shacl_model::shape::Shape::Node(_)))
        .count();
    assert_eq!(node_shapes, 1);
}

#[test]
fn conforming_data_passes() {
    let shapes = parse_shapes(SHAPES).unwrap();
    let data = parse_data(
        r#"
        @prefix ex:  <http://example.com/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:alice a ex:Person ;
            ex:name "Alice" ;
            ex:age 30 .
        "#,
    )
    .unwrap();
    let report = validate(&data, &shapes);
    assert!(report.conforms(), "alice is valid: {report:?}");
}

#[test]
fn violating_data_is_reported() {
    let shapes = parse_shapes(SHAPES).unwrap();
    // bob: missing ex:name (minCount 1), age -5 (minInclusive 0), two ages (maxCount 1).
    let data = parse_data(
        r#"
        @prefix ex:  <http://example.com/> .
        ex:bob a ex:Person ;
            ex:age -5 ;
            ex:age 7 .
        "#,
    )
    .unwrap();
    let report = validate(&data, &shapes);
    assert!(!report.conforms());
    let components: Vec<String> = report
        .results
        .iter()
        .map(|r| r.source_constraint_component.as_str().to_string())
        .collect();
    assert!(
        components
            .iter()
            .any(|c| c.ends_with("MinCountConstraintComponent")),
        "missing name"
    );
    assert!(
        components
            .iter()
            .any(|c| c.ends_with("MaxCountConstraintComponent")),
        "two ages"
    );
    assert!(
        components
            .iter()
            .any(|c| c.ends_with("MinInclusiveConstraintComponent")),
        "negative age"
    );
}

#[test]
fn complex_path_round_trips() {
    // sh:path ( ex:a [ sh:inversePath ex:b ] ) — a sequence containing an inverse.
    let shapes = parse_shapes(
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.com/> .
        ex:S a sh:PropertyShape ;
            sh:path ( ex:a [ sh:inversePath ex:b ] ) ;
            sh:minCount 1 .
        "#,
    )
    .unwrap();
    assert_eq!(shapes.len(), 1);
    if let shacl_model::shape::Shape::Property(p) = &shapes[0] {
        use shacl_model::path::Path;
        assert!(matches!(p.path, Path::Sequence(_)), "got {:?}", p.path);
    } else {
        panic!("expected a property shape");
    }
}

#[test]
fn malformed_turtle_is_an_error() {
    assert!(parse_shapes("this is not turtle @@@").is_err());
}

#[test]
fn deactivated_shape_is_parsed() {
    let shapes = parse_shapes(
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.com/> .
        ex:S a sh:NodeShape ;
            sh:deactivated true ;
            sh:targetNode ex:x ;
            sh:class ex:Never .
        "#,
    )
    .unwrap();
    assert!(shapes.iter().any(|s| s.deactivated()));
}

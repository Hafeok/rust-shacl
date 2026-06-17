//! End-to-end engine tests (§6): targets → value nodes → constraint dispatch → report, run over
//! [`MemGraph`]. Covers the three §7.1 value-type components currently wired (nodeKind, class,
//! datatype) plus target resolution (`REQ-TGT-1/2/4`) and direct-focus validation (`REQ-TGT-7`).

use shacl_core::{validate, validate_focus, ValidationReport};
use shacl_model::path::Path;
use shacl_model::shape::{Constraint, NodeShape, PropertyShape, Shape, ShapeId};
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
fn xsd(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{XSD}{local}"))
}
fn node(local: &str) -> Term {
    Term::NamedNode(iri(local))
}
fn rdf_type() -> NamedNode {
    NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

fn constraint(component: &str, params: Vec<(NamedNode, Term)>) -> Constraint {
    Constraint {
        component: sh(component),
        params,
        severity: None,
        deactivated: false,
    }
}

fn node_shape(targets: Vec<shacl_model::target::Target>, constraints: Vec<Constraint>) -> Shape {
    Shape::Node(NodeShape {
        id: ShapeId::Named(iri("Shape")),
        targets,
        constraints,
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    })
}

#[test]
fn nodekind_iri_violation_on_literal_value() {
    // PropertyShape: sh:path ex:p, sh:nodeKind sh:IRI. ex:a ex:p "lit" → one violation on the literal.
    use shacl_model::target::Target;
    let mut g = MemGraph::new();
    g.insert(
        node("a"),
        iri("p"),
        Term::Literal(Literal::new_simple_literal("lit")),
    );
    g.insert(node("a"), iri("p"), node("b")); // b is an IRI → conforms

    let shape = Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("Shape")),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![constraint(
            "NodeKindConstraintComponent",
            vec![(sh("nodeKind"), Term::NamedNode(sh("IRI")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    });

    let report = validate(&g, &[shape]);
    assert!(
        !report.conforms(),
        "literal value violates sh:nodeKind sh:IRI"
    );
    assert_eq!(report.results.len(), 1);
    let r = &report.results[0];
    assert_eq!(
        r.value,
        Some(Term::Literal(Literal::new_simple_literal("lit")))
    );
    assert_eq!(
        r.source_constraint_component,
        sh("NodeKindConstraintComponent")
    );
    assert_eq!(r.result_path.as_deref(), Some("<http://example.com/p>"));
}

#[test]
fn class_target_and_subclass_walk() {
    // ex:Dog rdfs:subClassOf ex:Animal. ex:rex a ex:Dog. NodeShape targetClass ex:Animal,
    // sh:class ex:Animal → rex conforms (transitive). ex:rock a ex:Mineral → not targeted.
    use shacl_model::target::Target;
    let subclassof = NamedNode::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subClassOf");
    let mut g = MemGraph::new();
    g.insert(node("Dog"), subclassof, node("Animal"));
    g.insert(node("rex"), rdf_type(), node("Dog"));
    g.insert(node("rock"), rdf_type(), node("Mineral"));

    let shape = node_shape(
        vec![Target::Class(iri("Animal"))],
        vec![constraint(
            "ClassConstraintComponent",
            vec![(sh("class"), Term::NamedNode(iri("Animal")))],
        )],
    );
    let report = validate(&g, &[shape]);
    assert!(
        report.conforms(),
        "rex is a SHACL instance of Animal via subClassOf; report: {report:?}"
    );
}

#[test]
fn class_violation_reported_for_non_instance() {
    // ex:rex a ex:Dog, but shape requires sh:class ex:Cat → violation.
    use shacl_model::target::Target;
    let mut g = MemGraph::new();
    g.insert(node("rex"), rdf_type(), node("Dog"));

    let shape = node_shape(
        vec![Target::Node(node("rex"))],
        vec![constraint(
            "ClassConstraintComponent",
            vec![(sh("class"), Term::NamedNode(iri("Cat")))],
        )],
    );
    let report = validate(&g, &[shape]);
    assert_eq!(report.results.len(), 1);
    assert_eq!(
        report.results[0].source_constraint_component,
        sh("ClassConstraintComponent")
    );
    assert_eq!(report.results[0].value, Some(node("rex")));
}

#[test]
fn datatype_matches_and_lexical_placeholder() {
    // sh:datatype xsd:integer over targetSubjectsOf ex:age. "42"^^xsd:integer conforms;
    // a plain IRI value does not.
    use shacl_model::target::Target;
    let int = Term::Literal(Literal::new_typed_literal("42", xsd("integer")));
    let mut g = MemGraph::new();
    g.insert(node("p1"), iri("age"), int.clone());

    let shape = Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("AgeShape")),
        path: Path::Predicate(iri("age")),
        targets: vec![Target::SubjectsOf(iri("age"))],
        constraints: vec![constraint(
            "DatatypeConstraintComponent",
            vec![(sh("datatype"), Term::NamedNode(xsd("integer")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    });
    let report = validate(&g, &[shape]);
    assert!(
        report.conforms(),
        "42^^xsd:integer conforms to sh:datatype xsd:integer; {report:?}"
    );
}

#[test]
fn datatype_illformed_lexical_violates_even_with_matching_iri() {
    // REQ-DATATYPE-2: "abc"^^xsd:integer has the right datatype IRI but an illegal lexical form.
    use shacl_model::target::Target;
    let bad = Term::Literal(Literal::new_typed_literal("abc", xsd("integer")));
    let mut g = MemGraph::new();
    g.insert(node("p1"), iri("age"), bad.clone());

    let shape = Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("AgeShape")),
        path: Path::Predicate(iri("age")),
        targets: vec![Target::SubjectsOf(iri("age"))],
        constraints: vec![constraint(
            "DatatypeConstraintComponent",
            vec![(sh("datatype"), Term::NamedNode(xsd("integer")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    });
    let report = validate(&g, &[shape]);
    assert_eq!(
        report.results.len(),
        1,
        "illegal integer lexical form must violate"
    );
    assert_eq!(report.results[0].value, Some(bad));
}

#[test]
fn datatype_wrong_iri_violates() {
    // REQ-DATATYPE-1: "42"^^xsd:integer against sh:datatype xsd:string → IRI mismatch.
    use shacl_model::target::Target;
    let mut g = MemGraph::new();
    g.insert(
        node("p1"),
        iri("age"),
        Term::Literal(Literal::new_typed_literal("42", xsd("integer"))),
    );
    let shape = Shape::Property(PropertyShape {
        id: ShapeId::Named(iri("AgeShape")),
        path: Path::Predicate(iri("age")),
        targets: vec![Target::SubjectsOf(iri("age"))],
        constraints: vec![constraint(
            "DatatypeConstraintComponent",
            vec![(sh("datatype"), Term::NamedNode(xsd("string")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Default::default(),
        deactivated: false,
    });
    assert_eq!(validate(&g, &[shape]).results.len(), 1);
}

#[test]
fn direct_focus_bypasses_targets() {
    // REQ-TGT-7: validate one node directly against one shape, ignoring targets.
    let mut g = MemGraph::new();
    g.insert(node("x"), rdf_type(), node("Dog"));
    let shape = node_shape(
        vec![], // no targets
        vec![constraint(
            "ClassConstraintComponent",
            vec![(sh("class"), Term::NamedNode(iri("Cat")))],
        )],
    );
    let mut report = ValidationReport::default();
    validate_focus(&g, &shape, &node("x"), &mut report);
    assert_eq!(
        report.results.len(),
        1,
        "direct focus validates regardless of targets"
    );
}

#[test]
fn deactivated_shape_produces_nothing() {
    use shacl_model::target::Target;
    let mut g = MemGraph::new();
    g.insert(node("rex"), rdf_type(), node("Dog"));
    let mut shape = node_shape(
        vec![Target::Node(node("rex"))],
        vec![constraint(
            "ClassConstraintComponent",
            vec![(sh("class"), Term::NamedNode(iri("Cat")))],
        )],
    );
    if let Shape::Node(ref mut n) = shape {
        n.deactivated = true;
    }
    let report = validate(&g, &[shape]);
    assert!(report.conforms() && report.results.is_empty());
}

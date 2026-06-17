//! Phase 9c shape-logic component tests (§7.7–7.8): sh:not/and/or/xone, sh:node/property,
//! sh:qualifiedValueShape — exercising the registry + recursive conformance over [`MemGraph`].

use shacl_core::{shape_ref_cycle, validate};
use shacl_model::path::Path;
use shacl_model::shape::{Constraint, NodeShape, PropertyShape, Severity, Shape, ShapeId};
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
fn id(local: &str) -> ShapeId {
    ShapeId::Named(iri(local))
}
fn constraint(component: &str, params: Vec<(NamedNode, Term)>) -> Constraint {
    Constraint {
        component: sh(component),
        params,
        severity: None,
        deactivated: false,
    }
}

/// A node shape with the given id, targets and constraints.
fn nshape(name: &str, targets: Vec<Target>, constraints: Vec<Constraint>) -> Shape {
    Shape::Node(NodeShape {
        id: id(name),
        targets,
        constraints,
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    })
}

/// A helper node shape that requires its focus to be an IRI (sh:nodeKind sh:IRI).
fn requires_iri(name: &str) -> Shape {
    nshape(
        name,
        vec![],
        vec![constraint(
            "NodeKindConstraintComponent",
            vec![(sh("nodeKind"), Term::NamedNode(sh("IRI")))],
        )],
    )
}

#[test]
fn not_negates_inner_shape() {
    // ex:a ex:p "lit" and ex:b. Outer property shape on ex:p with sh:not(requires-IRI):
    // the literal value conforms to NOT(requires-IRI) (it is not an IRI) → ok;
    // the IRI value ex:b violates NOT (it IS an IRI).
    let mut g = MemGraph::new();
    g.insert(
        node("a"),
        iri("p"),
        Term::Literal(Literal::new_simple_literal("lit")),
    );
    g.insert(node("a"), iri("p"), node("b"));

    let outer = Shape::Property(PropertyShape {
        id: id("Outer"),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![constraint(
            "NotConstraintComponent",
            vec![(sh("not"), Term::NamedNode(iri("Inner")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });
    let inner = requires_iri("Inner");

    let report = validate(&g, &[outer, inner]);
    assert_eq!(
        report.results.len(),
        1,
        "only the IRI value violates sh:not"
    );
}

#[test]
fn and_requires_all() {
    // Outer node shape on ex:a with sh:and(Inner1, Inner2); Inner1 requires IRI (ok for ex:a),
    // Inner2 requires a literal nodeKind → ex:a (an IRI) fails Inner2 → violation.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("t"), node("x")); // give ex:a some presence

    let outer = nshape(
        "Outer",
        vec![Target::Node(node("a"))],
        vec![constraint(
            "AndConstraintComponent",
            vec![
                (sh("and"), Term::NamedNode(iri("I1"))),
                (sh("and"), Term::NamedNode(iri("I2"))),
            ],
        )],
    );
    let i1 = requires_iri("I1");
    let i2 = nshape(
        "I2",
        vec![],
        vec![constraint(
            "NodeKindConstraintComponent",
            vec![(sh("nodeKind"), Term::NamedNode(sh("Literal")))],
        )],
    );
    assert_eq!(validate(&g, &[outer, i1, i2]).results.len(), 1);
}

#[test]
fn or_requires_some() {
    // sh:or(requires-IRI, requires-literal): an IRI focus conforms via the first disjunct.
    let g = MemGraph::new();
    let outer = nshape(
        "Outer",
        vec![Target::Node(node("a"))],
        vec![constraint(
            "OrConstraintComponent",
            vec![
                (sh("or"), Term::NamedNode(iri("I1"))),
                (sh("or"), Term::NamedNode(iri("I2"))),
            ],
        )],
    );
    let i1 = requires_iri("I1");
    let i2 = nshape(
        "I2",
        vec![],
        vec![constraint(
            "NodeKindConstraintComponent",
            vec![(sh("nodeKind"), Term::NamedNode(sh("Literal")))],
        )],
    );
    assert!(
        validate(&g, &[outer, i1, i2]).conforms(),
        "IRI satisfies the IRI disjunct"
    );
}

#[test]
fn xone_requires_exactly_one() {
    // sh:xone(requires-IRI, requires-BlankNodeOrIRI): an IRI conforms to BOTH → xone violated.
    let g = MemGraph::new();
    let outer = nshape(
        "Outer",
        vec![Target::Node(node("a"))],
        vec![constraint(
            "XoneConstraintComponent",
            vec![
                (sh("xone"), Term::NamedNode(iri("I1"))),
                (sh("xone"), Term::NamedNode(iri("I2"))),
            ],
        )],
    );
    let i1 = requires_iri("I1");
    let i2 = nshape(
        "I2",
        vec![],
        vec![constraint(
            "NodeKindConstraintComponent",
            vec![(sh("nodeKind"), Term::NamedNode(sh("BlankNodeOrIRI")))],
        )],
    );
    assert_eq!(
        validate(&g, &[outer, i1, i2]).results.len(),
        1,
        "IRI matches both → not exactly one"
    );
}

#[test]
fn node_summarises_property_bubbles() {
    // ex:a ex:p ex:b ; ex:b has no ex:q. Inner property shape: sh:path ex:q, sh:minCount 1.
    // Outer-via-sh:node → one summarising NodeConstraintComponent result for ex:b.
    // Outer-via-sh:property → the inner MinCount result bubbles up.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));

    let inner = Shape::Property(PropertyShape {
        id: id("Inner"),
        path: Path::Predicate(iri("q")),
        targets: vec![],
        constraints: vec![constraint(
            "MinCountConstraintComponent",
            vec![(
                sh("minCount"),
                Term::Literal(Literal::new_typed_literal(
                    "1",
                    NamedNode::new_unchecked(format!("{XSD}integer")),
                )),
            )],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });

    // sh:node form (property shape on ex:p whose value ex:b must conform to Inner)
    let outer_node = Shape::Property(PropertyShape {
        id: id("OuterNode"),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![constraint(
            "NodeConstraintComponent",
            vec![(sh("node"), Term::NamedNode(iri("Inner")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });
    let r1 = validate(&g, &[outer_node, inner.clone()]);
    assert_eq!(r1.results.len(), 1);
    assert_eq!(
        r1.results[0].source_constraint_component,
        sh("NodeConstraintComponent")
    );
    assert_eq!(r1.results[0].value, Some(node("b")));

    let outer_prop = Shape::Property(PropertyShape {
        id: id("OuterProp"),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![constraint(
            "PropertyConstraintComponent",
            vec![(sh("property"), Term::NamedNode(iri("Inner")))],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });
    let r2 = validate(&g, &[outer_prop, inner]);
    assert_eq!(r2.results.len(), 1);
    assert_eq!(
        r2.results[0].source_constraint_component,
        sh("MinCountConstraintComponent")
    );
}

#[test]
fn qualified_min_count() {
    // ex:a ex:p {ex:b (IRI), "lit"}. qualifiedValueShape requires IRI, qualifiedMinCount 2.
    // Only one value (ex:b) conforms → violation.
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(
        node("a"),
        iri("p"),
        Term::Literal(Literal::new_simple_literal("lit")),
    );

    let outer = Shape::Property(PropertyShape {
        id: id("Outer"),
        path: Path::Predicate(iri("p")),
        targets: vec![Target::Node(node("a"))],
        constraints: vec![constraint(
            "QualifiedMinCountConstraintComponent",
            vec![
                (sh("qualifiedValueShape"), Term::NamedNode(iri("Inner"))),
                (
                    sh("qualifiedMinCount"),
                    Term::Literal(Literal::new_typed_literal(
                        "2",
                        NamedNode::new_unchecked(format!("{XSD}integer")),
                    )),
                ),
            ],
        )],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });
    let inner = requires_iri("Inner");
    assert_eq!(
        validate(&g, &[outer, inner]).results.len(),
        1,
        "only 1 of 2 values conforms"
    );
}

#[test]
fn closed_rejects_extra_predicates() {
    // ex:a has ex:p and ex:q. The node shape declares sh:property for ex:p only and sh:closed true,
    // ignoring rdf:type → ex:q is an extra predicate → one violation (resultPath ex:q, value its obj).
    let mut g = MemGraph::new();
    g.insert(node("a"), iri("p"), node("b"));
    g.insert(node("a"), iri("q"), node("c"));

    let pshape = Shape::Property(PropertyShape {
        id: id("PShape"),
        path: Path::Predicate(iri("p")),
        targets: vec![],
        constraints: vec![],
        messages: vec![],
        sparql: vec![],
        severity: Severity::default(),
        deactivated: false,
    });
    let outer = nshape(
        "Outer",
        vec![Target::Node(node("a"))],
        vec![
            constraint(
                "PropertyConstraintComponent",
                vec![(sh("property"), Term::NamedNode(iri("PShape")))],
            ),
            constraint(
                "ClosedConstraintComponent",
                vec![(
                    sh("closed"),
                    Term::Literal(Literal::new_typed_literal(
                        "true",
                        NamedNode::new_unchecked(format!("{XSD}boolean")),
                    )),
                )],
            ),
        ],
    );
    let report = validate(&g, &[outer, pshape]);
    assert_eq!(report.results.len(), 1, "ex:q is not an allowed predicate");
    let r = &report.results[0];
    assert_eq!(
        r.source_constraint_component,
        sh("ClosedConstraintComponent")
    );
    assert_eq!(r.value, Some(node("c")));
    assert_eq!(r.result_path.as_deref(), Some("<http://example.com/q>"));
}

#[test]
fn recursion_guard_flags_cycle() {
    // Two shapes referencing each other via sh:node → the guard reports a cycle.
    let a = nshape(
        "A",
        vec![],
        vec![constraint(
            "NodeConstraintComponent",
            vec![(sh("node"), Term::NamedNode(iri("B")))],
        )],
    );
    let b = nshape(
        "B",
        vec![],
        vec![constraint(
            "NodeConstraintComponent",
            vec![(sh("node"), Term::NamedNode(iri("A")))],
        )],
    );
    assert!(shape_ref_cycle(&[a, b]).is_some());
}

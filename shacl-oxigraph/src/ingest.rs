//! Shapes-graph ingestion (§3, `REQ-ING-*`, ADR-009): parse a Turtle 1.2 document into the
//! [`Shape`] model the engine consumes. Lives in `shacl-oxigraph` because it needs a concrete
//! parser (`oxttl`); `shacl-core` stays parser-agnostic.
//!
//! The pipeline is: Turtle → in-memory triples ([`MemGraph`]) → shape extraction. A node is treated
//! as a shape when it carries a `sh:path` (property shape) or any shape-defining predicate (target,
//! constraint parameter, or `sh:NodeShape`/`sh:PropertyShape` type — node shape). Each declared
//! parameter is grouped under its constraint component, list-valued parameters (`sh:in`, `sh:and`,
//! …) are flattened into repeated `(predicate, element)` pairs, and `sh:path` is parsed into the
//! [`Path`] AST (all seven kinds).

use crate::mem::MemGraph;
use shacl_core::constraints::list::rdf_list;
use shacl_core::RdfGraph;
use shacl_model::path::Path;
use shacl_model::shape::{Constraint, NodeShape, PropertyShape, Severity, Shape, ShapeId};
use shacl_model::target::Target;
use shacl_model::term::{NamedNode, NamedOrBlankNode, Term};
use std::collections::BTreeSet;

const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";

fn sh(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{local}"))
}
fn rdf(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{RDF}{local}"))
}

/// Each constraint component: its primary parameter, the component local name, and any secondary
/// parameters folded into the same [`Constraint`] (`sh:flags`, `sh:ignoredProperties`,
/// `sh:qualifiedValueShape`).
const COMPONENTS: &[(&str, &str, &[&str])] = &[
    ("class", "ClassConstraintComponent", &[]),
    ("datatype", "DatatypeConstraintComponent", &[]),
    ("nodeKind", "NodeKindConstraintComponent", &[]),
    ("minCount", "MinCountConstraintComponent", &[]),
    ("maxCount", "MaxCountConstraintComponent", &[]),
    ("minExclusive", "MinExclusiveConstraintComponent", &[]),
    ("minInclusive", "MinInclusiveConstraintComponent", &[]),
    ("maxExclusive", "MaxExclusiveConstraintComponent", &[]),
    ("maxInclusive", "MaxInclusiveConstraintComponent", &[]),
    ("minLength", "MinLengthConstraintComponent", &[]),
    ("maxLength", "MaxLengthConstraintComponent", &[]),
    ("pattern", "PatternConstraintComponent", &["flags"]),
    ("singleLine", "SingleLineConstraintComponent", &[]),
    ("languageIn", "LanguageInConstraintComponent", &[]),
    ("uniqueLang", "UniqueLangConstraintComponent", &[]),
    ("hasValue", "HasValueConstraintComponent", &[]),
    ("in", "InConstraintComponent", &[]),
    ("equals", "EqualsConstraintComponent", &[]),
    ("disjoint", "DisjointConstraintComponent", &[]),
    ("subsetOf", "SubsetOfConstraintComponent", &[]),
    ("lessThan", "LessThanConstraintComponent", &[]),
    (
        "lessThanOrEquals",
        "LessThanOrEqualsConstraintComponent",
        &[],
    ),
    ("not", "NotConstraintComponent", &[]),
    ("and", "AndConstraintComponent", &[]),
    ("or", "OrConstraintComponent", &[]),
    ("xone", "XoneConstraintComponent", &[]),
    ("node", "NodeConstraintComponent", &[]),
    ("property", "PropertyConstraintComponent", &[]),
    ("minListLength", "MinListLengthConstraintComponent", &[]),
    ("maxListLength", "MaxListLengthConstraintComponent", &[]),
    ("uniqueMembers", "UniqueMembersConstraintComponent", &[]),
    ("memberShape", "MemberShapeConstraintComponent", &[]),
    (
        "closed",
        "ClosedConstraintComponent",
        &["ignoredProperties"],
    ),
    (
        "qualifiedMinCount",
        "QualifiedMinCountConstraintComponent",
        &["qualifiedValueShape"],
    ),
    (
        "qualifiedMaxCount",
        "QualifiedMaxCountConstraintComponent",
        &["qualifiedValueShape"],
    ),
];

/// Always-list parameters: their object is an `rdf:List` head, flattened into the members.
const LIST_PARAMS: &[&str] = &["in", "languageIn", "and", "or", "xone", "ignoredProperties"];

/// Maybe-list parameters (1.2): each value is *either* a plain term (1.0) *or* an `rdf:List` head to
/// flatten. `sh:datatype`/`sh:nodeKind` list members are disjuncts (dispatch builds one set-valued
/// validator). NOTE: `sh:class` is intentionally excluded — a *list* value of `sh:class` is a
/// disjunction (instance-of-any) whereas *repeated* `sh:class` triples are a conjunction, and the
/// flat param model cannot tell them apart post-flattening (W3C `core/property/class-002`, a gap).
const MAYBE_LIST_PARAMS: &[&str] = &["datatype", "nodeKind"];

/// Parse a Turtle 1.2 document into its shapes (`REQ-ING-1..10`). Returns the parse error message on
/// malformed Turtle (`REQ-ING-1` → failure).
pub fn parse_shapes(turtle: &str) -> Result<Vec<Shape>, String> {
    let graph = parse_turtle(turtle, None)?;
    Ok(shapes_from_graph(&graph))
}

/// Like [`parse_shapes`] but resolving relative IRIs against `base` (W3C test files use `<>` and
/// relative entry IRIs that need a base, §10.1).
pub fn parse_shapes_with_base(turtle: &str, base: &str) -> Result<Vec<Shape>, String> {
    let graph = parse_turtle(turtle, Some(base))?;
    Ok(shapes_from_graph(&graph))
}

/// Parse a Turtle 1.2 document into the data graph it denotes (the data graph being validated).
pub fn parse_data(turtle: &str) -> Result<MemGraph, String> {
    parse_turtle(turtle, None)
}

/// Like [`parse_data`] but resolving relative IRIs against `base`.
pub fn parse_data_with_base(turtle: &str, base: &str) -> Result<MemGraph, String> {
    parse_turtle(turtle, Some(base))
}

fn parse_turtle(turtle: &str, base: Option<&str>) -> Result<MemGraph, String> {
    let mut parser = oxttl::TurtleParser::new();
    if let Some(b) = base {
        parser = parser.with_base_iri(b).map_err(|e| e.to_string())?;
    }
    let mut g = MemGraph::new();
    for t in parser.for_slice(turtle.as_bytes()) {
        let t = t.map_err(|e| e.to_string())?;
        let subject = match t.subject {
            NamedOrBlankNode::NamedNode(n) => Term::NamedNode(n),
            NamedOrBlankNode::BlankNode(b) => Term::BlankNode(b),
        };
        g.insert(subject, t.predicate, t.object);
    }
    Ok(g)
}

/// Extract every shape from the parsed graph.
fn shapes_from_graph(g: &MemGraph) -> Vec<Shape> {
    collect_shape_nodes(g)
        .into_iter()
        .map(|node| build_shape(g, &node))
        .collect()
}

/// Collect the subjects that denote shapes: any subject of `sh:path`, a target predicate, a
/// constraint parameter, or an `rdf:type sh:NodeShape`/`sh:PropertyShape`. Deduplicated (on the
/// term's canonical N-Triples string, since oxrdf `Term` is not `Ord`) and order-stable.
fn collect_shape_nodes(g: &MemGraph) -> Vec<Term> {
    let mut shape_preds: Vec<NamedNode> = vec![
        sh("path"),
        sh("targetNode"),
        sh("targetClass"),
        sh("targetSubjectsOf"),
        sh("targetObjectsOf"),
    ];
    for (primary, _, secondary) in COMPONENTS {
        shape_preds.push(sh(primary));
        for s in *secondary {
            shape_preds.push(sh(s));
        }
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<Term> = Vec::new();
    let mut push = |t: Term, out: &mut Vec<Term>| {
        if seen.insert(t.to_string()) {
            out.push(t);
        }
    };
    for pred in &shape_preds {
        for tr in g.triples(None, Some(pred), None) {
            push(tr.subject, &mut out);
        }
    }
    // Typed shapes (sh:NodeShape / sh:PropertyShape) even with no other predicate.
    let type_pred = rdf("type");
    for local in ["NodeShape", "PropertyShape"] {
        let ty = Term::NamedNode(sh(local));
        for tr in g.triples(None, Some(&type_pred), Some(&ty)) {
            push(tr.subject, &mut out);
        }
    }
    out
}

/// Build one [`Shape`] for the given node.
fn build_shape(g: &MemGraph, node: &Term) -> Shape {
    let id = match node {
        Term::NamedNode(n) => ShapeId::Named(n.clone()),
        Term::BlankNode(b) => ShapeId::Blank(b.as_str().to_string()),
        _ => ShapeId::Blank("invalid".to_string()),
    };
    let targets = parse_targets(g, node);
    let constraints = parse_constraints(g, node);
    let severity = parse_severity(g, node);
    let deactivated = bool_value(g, node, &sh("deactivated"));

    // A property shape is one with a sh:path; everything else is a node shape (REQ-ING-2).
    match single_object(g, node, &sh("path")).and_then(|p| parse_path(g, &p)) {
        Some(path) => Shape::Property(PropertyShape {
            id,
            path,
            targets,
            constraints,
            severity,
            deactivated,
        }),
        None => Shape::Node(NodeShape {
            id,
            targets,
            constraints,
            severity,
            deactivated,
        }),
    }
}

fn parse_targets(g: &MemGraph, node: &Term) -> Vec<Target> {
    let mut targets = Vec::new();
    for t in g.objects(node, &sh("targetNode")) {
        targets.push(Target::Node(t));
    }
    for t in g.objects(node, &sh("targetClass")) {
        if let Term::NamedNode(c) = t {
            targets.push(Target::Class(c));
        }
    }
    for t in g.objects(node, &sh("targetSubjectsOf")) {
        if let Term::NamedNode(p) = t {
            targets.push(Target::SubjectsOf(p));
        }
    }
    for t in g.objects(node, &sh("targetObjectsOf")) {
        if let Term::NamedNode(p) = t {
            targets.push(Target::ObjectsOf(p));
        }
    }
    // Implicit class target (REQ-TGT-3): an IRI shape that is also an rdfs:Class.
    if let Term::NamedNode(n) = node {
        let is_class = g
            .objects(node, &rdf("type"))
            .iter()
            .any(|t| matches!(t, Term::NamedNode(c) if c.as_str() == format!("{RDFS}Class")));
        if is_class {
            targets.push(Target::ImplicitClass(n.clone()));
        }
    }
    targets
}

fn parse_constraints(g: &MemGraph, node: &Term) -> Vec<Constraint> {
    let mut out = Vec::new();
    for (primary, component, secondary) in COMPONENTS {
        let primary_pred = sh(primary);
        // Presence is by predicate, not flattened values: an empty list param (e.g. `sh:in ()`,
        // `sh:xone ()`) is still a declared constraint with defined semantics.
        if g.objects(node, &primary_pred).is_empty() {
            continue;
        }
        let primary_values = param_values(g, node, primary, &primary_pred);
        let mut params: Vec<(NamedNode, Term)> = primary_values
            .into_iter()
            .map(|v| (primary_pred.clone(), v))
            .collect();
        for sec in *secondary {
            let sec_pred = sh(sec);
            for v in param_values(g, node, sec, &sec_pred) {
                params.push((sec_pred.clone(), v));
            }
        }
        out.push(Constraint {
            component: sh(component),
            params,
            severity: None,
            deactivated: false,
        });
    }
    out
}

/// The values of parameter `local` on `node`. Always-list params are flattened from their `rdf:List`
/// head; maybe-list params are flattened only when the value is actually a list (else kept as-is).
fn param_values(g: &MemGraph, node: &Term, local: &str, pred: &NamedNode) -> Vec<Term> {
    let objects: Vec<Term> = g.objects(node, pred).into_iter().collect();
    if LIST_PARAMS.contains(&local) {
        objects
            .iter()
            .filter_map(|head| rdf_list(g, head))
            .flatten()
            .collect()
    } else if MAYBE_LIST_PARAMS.contains(&local) {
        objects
            .into_iter()
            .flat_map(|o| rdf_list(g, &o).unwrap_or_else(|| vec![o]))
            .collect()
    } else {
        objects
    }
}

fn parse_severity(g: &MemGraph, node: &Term) -> Severity {
    match single_object(g, node, &sh("severity")) {
        Some(Term::NamedNode(n)) => match n.as_str().strip_prefix(SH) {
            Some("Trace") => Severity::Trace,
            Some("Debug") => Severity::Debug,
            Some("Info") => Severity::Info,
            Some("Warning") => Severity::Warning,
            _ => Severity::Violation,
        },
        _ => Severity::Violation,
    }
}

/// Parse a `sh:path` object into the [`Path`] AST (all seven kinds, §4.1).
fn parse_path(g: &MemGraph, node: &Term) -> Option<Path> {
    if let Term::NamedNode(n) = node {
        return Some(Path::Predicate(n.clone())); // predicate path
    }
    // Blank-node path structures.
    if let Some(inner) = single_object(g, node, &sh("inversePath")) {
        return parse_path(g, &inner).map(|p| Path::Inverse(Box::new(p)));
    }
    if let Some(list_head) = single_object(g, node, &sh("alternativePath")) {
        let parts = path_list(g, &list_head)?;
        return Some(Path::Alternative(parts));
    }
    if let Some(inner) = single_object(g, node, &sh("zeroOrMorePath")) {
        return parse_path(g, &inner).map(|p| Path::ZeroOrMore(Box::new(p)));
    }
    if let Some(inner) = single_object(g, node, &sh("oneOrMorePath")) {
        return parse_path(g, &inner).map(|p| Path::OneOrMore(Box::new(p)));
    }
    if let Some(inner) = single_object(g, node, &sh("zeroOrOnePath")) {
        return parse_path(g, &inner).map(|p| Path::ZeroOrOne(Box::new(p)));
    }
    // Otherwise: a bare rdf:List is a sequence path.
    if !g.objects(node, &rdf("first")).is_empty() {
        let parts = path_list(g, node)?;
        return Some(Path::Sequence(parts));
    }
    None
}

/// Parse each element of the `rdf:List` at `head` as a sub-path.
fn path_list(g: &MemGraph, head: &Term) -> Option<Vec<Path>> {
    rdf_list(g, head)?
        .iter()
        .map(|e| parse_path(g, e))
        .collect()
}

/// The single object of `(node, pred, *)`, or `None` if absent (first if several).
fn single_object(g: &MemGraph, node: &Term, pred: &NamedNode) -> Option<Term> {
    g.objects(node, pred).into_iter().next()
}

/// Is there a `(node, pred, "true"^^xsd:boolean)` (or `"true"`) triple?
fn bool_value(g: &MemGraph, node: &Term, pred: &NamedNode) -> bool {
    g.objects(node, pred).iter().any(|t| match t {
        Term::Literal(l) => matches!(l.value(), "true" | "1"),
        _ => false,
    })
}

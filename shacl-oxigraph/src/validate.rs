//! High-level validation entry points (the host-application surface). Combines the Core L1 engine
//! (§7, over [`OxiStore`] as an `RdfGraph`) with the SPARQL-based constraints (§8.1, run over the
//! same store as a `SparqlGraph`), so a single call validates a graph against shapes that mix Core
//! and `sh:sparql` rules and returns one report — with `sh:message`s populated.

use crate::ingest::parse_shapes;
use crate::store::OxiStore;
use shacl_core::report::ValidationReport;
use shacl_core::{build_registry, focus_nodes, validate};
use shacl_model::shape::Shape;
use shacl_sparql::constraint::{validate_select, Outcome, SelectConstraint};

/// Validate the `data_ttl` graph against the `shapes_ttl` shapes graph (both Turtle 1.2), running
/// Core *and* SPARQL-based constraints. Returns the merged report, or a parse error.
pub fn validate_turtle(shapes_ttl: &str, data_ttl: &str) -> Result<ValidationReport, String> {
    let shapes = parse_shapes(shapes_ttl)?;
    // The Core engine needs only `RdfGraph`; the SPARQL pass needs a `SparqlGraph`. An OxiStore is
    // both, so load the data once.
    let store = OxiStore::from_turtle(data_ttl)?;
    Ok(validate_store(&store, &shapes))
}

/// Validate an already-loaded [`OxiStore`] against parsed `shapes`, running Core constraints (§7)
/// then SPARQL-based constraints (§8.1) and merging the results. Use this when the host already has
/// an `oxigraph::Store` (wrap it with [`OxiStore::new`]).
#[must_use]
pub fn validate_store(store: &OxiStore, shapes: &[Shape]) -> ValidationReport {
    // Core L1 validation (over the store as an RdfGraph).
    let mut report = validate(store, shapes);

    // SPARQL-based constraints (§8.1): for each shape carrying `sh:sparql`, run each SELECT against
    // every focus node with `this` pre-bound, mapping solutions to results (REQ-SPQ-2..6).
    let registry = build_registry(shapes);
    for shape in shapes {
        if shape.deactivated() || shape.sparql().is_empty() {
            continue;
        }
        let path_sparql = match shape {
            Shape::Property(p) => Some(p.path.to_sparql()),
            Shape::Node(_) => None,
        };
        let foci = focus_nodes(store, &registry, shape);
        for sparql in shape.sparql() {
            let constraint = SelectConstraint {
                select: sparql.select.clone(),
                source_shape: shape.id().clone(),
                severity: shape.severity(),
                message: sparql.messages.first().cloned(),
            };
            for focus in &foci {
                match validate_select(store, focus, path_sparql.as_deref(), &constraint) {
                    Outcome::Results(rs) => report.results.extend(rs),
                    // A SPARQL processing failure (REQ-SPQ-3) is distinct from a violation; surface
                    // it as a violation-severity result so it is never silently dropped.
                    Outcome::Failure(msg) => report.results.push(failure_result(shape, &msg)),
                }
            }
        }
    }
    report
}

/// A synthetic result representing a SPARQL-constraint processing failure.
fn failure_result(shape: &Shape, msg: &str) -> shacl_core::ValidationResult {
    use shacl_model::shape::Severity;
    use shacl_model::term::{NamedNode, Term};
    shacl_core::ValidationResult {
        focus_node: Term::NamedNode(NamedNode::new_unchecked("urn:x-shacl:failure")),
        result_path: None,
        value: None,
        source_constraint_component: NamedNode::new_unchecked(
            "http://www.w3.org/ns/shacl#SPARQLConstraintComponent",
        ),
        source_shape: shape.id().clone(),
        severity: Severity::Violation,
        messages: vec![format!("SPARQL constraint failure: {msg}")],
    }
}

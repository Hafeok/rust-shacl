//! The validator seam (§11.3). Every §7 constraint component implements [`Validator`]; the engine
//! computes value nodes (§5) then dispatches each declared constraint to its validator.

use crate::engine::Registry;
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use shacl_model::shape::{Constraint, Severity, Shape};
use shacl_model::term::{NamedNodeRef, Term};

/// Context handed to a validator for one constraint on one focus node.
pub struct Ctx<'a, G: RdfGraph> {
    /// The data graph.
    pub graph: &'a G,
    /// The focus node being validated.
    pub focus: &'a Term,
    /// The shape declaring the constraint (for `sh:sourceShape`, §6.7.2.4).
    pub shape: &'a Shape,
    /// The specific constraint instance (component IRI + params + per-constraint severity).
    pub constraint: &'a Constraint,
    /// Effective severity for results from this constraint (per-constraint override else shape's).
    pub severity: Severity,
    /// The property path, if the shape is a property shape (for `sh:resultPath`, §6.7.2.2).
    pub path_sparql: Option<String>,
    /// Shape registry for resolving referenced shapes (`sh:node`/`sh:property`/logical/§7.5/7.8).
    pub registry: &'a Registry<'a>,
    /// Conformance-recursion depth (backstop against runaway recursion; the real guard is the
    /// shapes-graph SCC check, ADR-002 / [`crate::recursion`]).
    pub depth: usize,
}

/// A constraint-component validator. One impl per `sh:…ConstraintComponent`.
///
/// Implementations push one [`ValidationResult`] per violation into `out`. Value nodes are
/// precomputed by the engine per §5 (`REQ-RPT-1`) so validators never touch path evaluation
/// directly — they only inspect the value nodes and (where needed) the graph via `ctx.graph`.
pub trait Validator<G: RdfGraph> {
    /// The `sh:…ConstraintComponent` IRI this validator handles (`sh:sourceConstraintComponent`).
    fn component_iri(&self) -> NamedNodeRef<'static>;

    /// Validate the given value nodes; append a result per violation.
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>);
}

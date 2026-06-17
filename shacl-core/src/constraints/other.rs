//! Remaining §7.9 components that need cross-constraint context. Currently `sh:closed` (§7.9.1):
//! the focus node may only use predicates that appear as the predicate path of one of the shape's
//! own `sh:property` constraints, plus those listed in `sh:ignoredProperties`.
//!
//! `sh:rootClass` (§7.9.4) and `sh:uniqueValuesFor` (§7.9.5) are new-in-1.2 and under-specified in
//! the current draft; they are intentionally not implemented (tracked as known gaps).

use super::comp;
use crate::engine::{lookup, term_to_shape_id};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::path::Path;
use shacl_model::shape::Shape;
use shacl_model::term::{NamedNode, NamedNodeRef, Term};
use std::collections::HashSet;

const SH: &str = "http://www.w3.org/ns/shacl#";

/// `sh:ClosedConstraintComponent` (node-level). For each triple `(focus, P, O)` whose predicate `P`
/// is neither a permitted property nor ignored, one result is produced with `sh:resultPath` = `P`
/// and `sh:value` = `O`.
pub struct ClosedValidator {
    /// Predicates exempted from closure (`sh:ignoredProperties`).
    pub ignored: Vec<NamedNode>,
}

impl ClosedValidator {
    /// The predicates permitted by closure: the predicate-path IRI of every `sh:property` shape
    /// referenced by the focus shape, plus the ignored predicates.
    fn allowed<G: RdfGraph>(&self, ctx: &Ctx<'_, G>) -> HashSet<String> {
        let mut set: HashSet<String> = self
            .ignored
            .iter()
            .map(|n| n.as_str().to_string())
            .collect();
        for c in ctx.shape.constraints() {
            let local = c
                .component
                .as_str()
                .strip_prefix(SH)
                .unwrap_or(c.component.as_str());
            if local != "PropertyConstraintComponent" {
                continue;
            }
            for (pred, val) in &c.params {
                if pred.as_str() != format!("{SH}property") {
                    continue;
                }
                if let Some(id) = term_to_shape_id(val) {
                    if let Some(Shape::Property(p)) = lookup(ctx.registry, &id) {
                        if let Path::Predicate(iri) = &p.path {
                            set.insert(iri.as_str().to_string());
                        }
                    }
                }
            }
        }
        set
    }
}

impl<G: RdfGraph> Validator<G> for ClosedValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ClosedConstraintComponent")
    }
    fn validate(&self, _value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let allowed = self.allowed(ctx);
        for t in ctx.graph.triples(Some(ctx.focus), None, None) {
            if allowed.contains(t.predicate.as_str()) {
                continue;
            }
            out.push(ValidationResult {
                focus_node: ctx.focus.clone(),
                result_path: Some(format!("<{}>", t.predicate.as_str())),
                value: Some(t.object),
                source_constraint_component: comp("ClosedConstraintComponent"),
                source_shape: ctx.shape.id().clone(),
                severity: ctx.severity,
                messages: Vec::new(),
            });
        }
    }
}

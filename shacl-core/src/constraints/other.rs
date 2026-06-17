//! Remaining §7.9 components that need cross-constraint or cross-focus context: `sh:closed`
//! (§7.9.1, the focus may only use predicates declared by the shape's `sh:property` shapes plus
//! `sh:ignoredProperties`) and `sh:uniqueValuesFor` (§7.9.5, a property's values must be unique
//! across the shape's focus nodes). `sh:rootClass` (§7.9.4) lives in `value_type`.

use super::{comp, result_for};
use crate::engine::{focus_nodes, lookup, term_to_shape_id};
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

/// `sh:UniqueValuesForConstraintComponent` (§7.9.5). Across the shape's focus nodes, the values of
/// the given property must be unique. For the current focus, each *other* focus node that shares at
/// least one value yields a result whose `sh:value` is that other focus node.
pub struct UniqueValuesForValidator {
    /// The property whose values must be unique across focus nodes.
    pub property: NamedNode,
}

impl<G: RdfGraph> Validator<G> for UniqueValuesForValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#UniqueValuesForConstraintComponent")
    }
    fn validate(&self, _value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let my_values = ctx.graph.objects(ctx.focus, &self.property);
        if my_values.is_empty() {
            return;
        }
        // Recompute the shape's focus set to compare against the other focus nodes (cross-focus
        // constraint; O(n) per focus is acceptable for the conformance suite).
        for other in focus_nodes(ctx.graph, ctx.registry, ctx.shape) {
            if &other == ctx.focus {
                continue;
            }
            let shares = ctx
                .graph
                .objects(&other, &self.property)
                .iter()
                .any(|v| my_values.contains(v));
            if shares {
                out.push(result_for(
                    ctx,
                    Some(other),
                    comp("UniqueValuesForConstraintComponent"),
                ));
            }
        }
    }
}

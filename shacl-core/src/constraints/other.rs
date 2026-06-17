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
    /// `sh:ByTypes` mode (1.2): the permitted predicates are those declared by the shapes of the
    /// focus node's `rdf:type`s (and their superclasses), rather than the focus shape's own.
    pub by_types: bool,
}

impl ClosedValidator {
    /// The predicates permitted by closure, plus the ignored predicates. In the default mode these
    /// are the predicate paths declared by the focus shape's own `sh:property` shapes; in `sh:ByTypes`
    /// mode they are gathered from the shapes named by the focus node's types and their superclasses.
    fn allowed<G: RdfGraph>(&self, ctx: &Ctx<'_, G>) -> HashSet<String> {
        let mut set: HashSet<String> = self
            .ignored
            .iter()
            .map(|n| n.as_str().to_string())
            .collect();
        if self.by_types {
            // rdf:type is implicitly permitted under sh:ByTypes — it is how the focus declares the
            // types that drive the closure (W3C core/node/closed-003).
            set.insert("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string());
            let classes: HashSet<String> = focus_type_closure(ctx)
                .iter()
                .map(|c| c.as_str().to_string())
                .collect();
            // Every shape associated with one of the focus's types — by identity (an implicit class
            // shape) or via `sh:targetClass` — contributes its (and its `sh:node`-referenced shapes')
            // property predicates.
            let mut visited: HashSet<String> = HashSet::new();
            for shape in ctx.registry.values() {
                if shape_targets_class(shape, &classes) {
                    gather_shape_predicates(shape, ctx.registry, &mut set, &mut visited);
                }
            }
        } else {
            collect_property_predicates(ctx.shape, ctx.registry, &mut set);
        }
        set
    }
}

/// Is `shape` associated with one of `classes`: identified by it (implicit class shape) or declaring
/// a `sh:targetClass`/implicit-class target for it?
fn shape_targets_class(shape: &Shape, classes: &HashSet<String>) -> bool {
    use shacl_model::shape::ShapeId;
    use shacl_model::target::Target;
    if let ShapeId::Named(n) = shape.id() {
        if classes.contains(n.as_str()) {
            return true;
        }
    }
    shape.targets().iter().any(|t| match t {
        Target::Class(c) | Target::ImplicitClass(c) => classes.contains(c.as_str()),
        _ => false,
    })
}

/// Gather the property predicates of `shape` and, transitively, of every shape it references via
/// `sh:node` (the §7.9.1 `sh:ByTypes` closure follows shape composition). `visited` guards cycles.
fn gather_shape_predicates(
    shape: &Shape,
    registry: &crate::engine::Registry<'_>,
    set: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) {
    let key = format!("{:?}", shape.id());
    if !visited.insert(key) {
        return;
    }
    collect_property_predicates(shape, registry, set);
    for c in shape.constraints() {
        if c.component.as_str() != format!("{SH}NodeConstraintComponent") {
            continue;
        }
        for (pred, val) in &c.params {
            if pred.as_str() != format!("{SH}node") {
                continue;
            }
            if let Some(id) = term_to_shape_id(val) {
                if let Some(referenced) = lookup(registry, &id) {
                    gather_shape_predicates(referenced, registry, set, visited);
                }
            }
        }
    }
}

/// The focus node's types and all their transitive `rdfs:subClassOf` superclasses (as IRIs).
fn focus_type_closure<G: RdfGraph>(ctx: &Ctx<'_, G>) -> Vec<NamedNode> {
    let type_pred = NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let sub_pred = NamedNode::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subClassOf");
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for t in ctx.graph.objects(ctx.focus, &type_pred) {
        let supers = crate::closure::reachable_star(t, |c: &Term| {
            ctx.graph
                .triples(Some(c), Some(&sub_pred), None)
                .map(|tr| tr.object)
                .collect::<Vec<_>>()
        });
        for s in supers {
            if let Term::NamedNode(n) = s {
                if seen.insert(n.as_str().to_string()) {
                    out.push(n);
                }
            }
        }
    }
    out
}

/// Insert the predicate-path IRIs of every `sh:property` shape referenced by `shape`.
fn collect_property_predicates(
    shape: &Shape,
    registry: &crate::engine::Registry<'_>,
    set: &mut HashSet<String>,
) {
    for c in shape.constraints() {
        let local = c.component.as_str().strip_prefix(SH).unwrap_or("");
        if local != "PropertyConstraintComponent" {
            continue;
        }
        for (pred, val) in &c.params {
            if pred.as_str() != format!("{SH}property") {
                continue;
            }
            if let Some(id) = term_to_shape_id(val) {
                if let Some(Shape::Property(p)) = lookup(registry, &id) {
                    if let Path::Predicate(iri) = &p.path {
                        set.insert(iri.as_str().to_string());
                    }
                }
            }
        }
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

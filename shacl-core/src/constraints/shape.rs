//! Shape-based and logical constraint components (§7.7–7.8). These reference *other* shapes and so
//! recurse through the engine's conformance check ([`crate::engine::conforms`] /
//! [`crate::engine::validate_focus_collect`]); termination is guaranteed by the shapes-graph
//! recursion guard (ADR-002, [`crate::recursion`]).
//!
//! - Logical (§7.7): `sh:not`, `sh:and`, `sh:or`, `sh:xone`.
//! - Shape (§7.8): `sh:node` (one summarising result per non-conforming value node),
//!   `sh:property` (bubbles the property shape's own results), `sh:qualifiedValueShape`
//!   (+`sh:qualifiedMinCount`/`sh:qualifiedMaxCount`).
//! - List (§7.5): `sh:memberShape` lives here too because it recurses into a shape.

use super::{comp, result_for};
use crate::engine::{conforms, lookup, validate_focus_collect};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::shape::ShapeId;
use shacl_model::term::{NamedNodeRef, Term};

/// Conformance of `value` to the shape identified by `id`, looked up in the context registry.
/// A dangling reference (no such shape) conforms vacuously (nothing to violate).
fn value_conforms<G: RdfGraph>(ctx: &Ctx<'_, G>, id: &ShapeId, value: &Term) -> bool {
    match lookup(ctx.registry, id) {
        Some(shape) => conforms(ctx.graph, ctx.registry, shape, value, ctx.depth),
        None => true,
    }
}

// ── sh:not (§7.7.1) ─────────────────────────────────────────────────────────────────────────────

/// `sh:NotConstraintComponent`. A value node violates iff it **does** conform to the negated shape.
pub struct NotValidator {
    /// The negated shape.
    pub shape: ShapeId,
}

impl<G: RdfGraph> Validator<G> for NotValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NotConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if value_conforms(ctx, &self.shape, v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("NotConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:and / sh:or / sh:xone (§7.7.2–4) ─────────────────────────────────────────────────────────

/// `sh:AndConstraintComponent`. A value node must conform to **every** listed shape.
pub struct AndValidator {
    /// The conjoined shapes.
    pub shapes: Vec<ShapeId>,
}

impl<G: RdfGraph> Validator<G> for AndValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#AndConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !self.shapes.iter().all(|s| value_conforms(ctx, s, v)) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("AndConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:OrConstraintComponent`. A value node must conform to **at least one** listed shape.
pub struct OrValidator {
    /// The disjoined shapes.
    pub shapes: Vec<ShapeId>,
}

impl<G: RdfGraph> Validator<G> for OrValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#OrConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !self.shapes.iter().any(|s| value_conforms(ctx, s, v)) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("OrConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:XoneConstraintComponent`. A value node must conform to **exactly one** listed shape.
pub struct XoneValidator {
    /// The exclusively-disjoined shapes.
    pub shapes: Vec<ShapeId>,
}

impl<G: RdfGraph> Validator<G> for XoneValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#XoneConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let n = self
                .shapes
                .iter()
                .filter(|s| value_conforms(ctx, s, v))
                .count();
            if n != 1 {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("XoneConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:node (§7.8.1) ────────────────────────────────────────────────────────────────────────────

/// `sh:NodeConstraintComponent`. One summarising result per value node that does not conform to the
/// referenced node shape (the inner results are not bubbled — that is `sh:property`'s job).
pub struct NodeValidator {
    /// The referenced shape.
    pub shape: ShapeId,
}

impl<G: RdfGraph> Validator<G> for NodeValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !value_conforms(ctx, &self.shape, v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("NodeConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:property (§7.8.2) ────────────────────────────────────────────────────────────────────────

/// `sh:PropertyConstraintComponent`. Each value node is validated against the referenced property
/// shape (as its focus node); that shape's own results bubble up into this report.
pub struct PropertyValidator {
    /// The referenced property shape.
    pub shape: ShapeId,
}

impl<G: RdfGraph> Validator<G> for PropertyValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PropertyConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let Some(shape) = lookup(ctx.registry, &self.shape) else {
            return;
        };
        for v in value_nodes {
            out.extend(validate_focus_collect(
                ctx.graph,
                ctx.registry,
                shape,
                v,
                ctx.depth.saturating_add(1),
            ));
        }
    }
}

// ── sh:someValue (§7.8.3, new in 1.2) ───────────────────────────────────────────────────────────

/// `sh:SomeValueConstraintComponent`. **At least one** value node must conform to the referenced
/// shape; otherwise a single result (no `sh:value` — the violation is the absence of a conformer).
pub struct SomeValueValidator {
    /// The referenced shape.
    pub shape: ShapeId,
}

impl<G: RdfGraph> Validator<G> for SomeValueValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SomeValueConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        if !value_nodes
            .iter()
            .any(|v| value_conforms(ctx, &self.shape, v))
        {
            out.push(result_for(ctx, None, comp("SomeValueConstraintComponent")));
        }
    }
}

// ── sh:qualifiedValueShape (§7.8.4) ─────────────────────────────────────────────────────────────

/// `sh:QualifiedMinCountConstraintComponent` / `sh:QualifiedMaxCountConstraintComponent`. The number
/// of value nodes conforming to the qualified shape must lie within the declared bound. (The
/// `sh:qualifiedValueShapesDisjoint` refinement, §7.8.4, is a documented gap.)
pub struct QualifiedValidator {
    /// The qualified shape.
    pub shape: ShapeId,
    /// The count bound.
    pub bound: i64,
    /// `true` = `sh:qualifiedMinCount`, `false` = `sh:qualifiedMaxCount`.
    pub is_min: bool,
}

impl<G: RdfGraph> Validator<G> for QualifiedValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        if self.is_min {
            NamedNodeRef::new_unchecked(
                "http://www.w3.org/ns/shacl#QualifiedMinCountConstraintComponent",
            )
        } else {
            NamedNodeRef::new_unchecked(
                "http://www.w3.org/ns/shacl#QualifiedMaxCountConstraintComponent",
            )
        }
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let conforming = value_nodes
            .iter()
            .filter(|v| value_conforms(ctx, &self.shape, v))
            .count() as i64;
        let violated = if self.is_min {
            conforming < self.bound
        } else {
            conforming > self.bound
        };
        if violated {
            let component = if self.is_min {
                "QualifiedMinCountConstraintComponent"
            } else {
                "QualifiedMaxCountConstraintComponent"
            };
            out.push(result_for(ctx, None, comp(component)));
        }
    }
}

// ── sh:memberShape (§7.5.1) ─────────────────────────────────────────────────────────────────────

/// `sh:MemberShapeConstraintComponent`. Every member of each value node's `rdf:List` must conform to
/// the referenced shape; one result per non-conforming member (`sh:value` = the member).
pub struct MemberShapeValidator {
    /// The shape each list member must conform to.
    pub shape: ShapeId,
}

impl<G: RdfGraph> Validator<G> for MemberShapeValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MemberShapeConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let Some(members) = super::list::rdf_list(ctx.graph, v) else {
                continue; // not a well-formed list: a CMP-LISTLEN concern, not this one.
            };
            for m in members {
                if !value_conforms(ctx, &self.shape, &m) {
                    out.push(result_for(
                        ctx,
                        Some(m),
                        comp("MemberShapeConstraintComponent"),
                    ));
                }
            }
        }
    }
}

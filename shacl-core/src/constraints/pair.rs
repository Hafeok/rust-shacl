//! Property-pair constraint components (§7.6, property shapes only). Each compares this shape's
//! value nodes against the values of a sibling property on the *same focus node*:
//! `sh:equals` (§7.6.1), `sh:disjoint` (§7.6.2), `sh:subsetOf` (§7.6.3, new in 1.2),
//! `sh:lessThan` (§7.6.4), `sh:lessThanOrEquals` (§7.6.5).
//!
//! The "other" values are reached from the focus node along the paired **path** (a predicate or, in
//! 1.2, a sequence of predicates), evaluated over the data graph via [`crate::path::reach`].

use super::range::compare;
use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::path::reach;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::path::Path;
use shacl_model::term::{NamedNodeRef, Term};
use std::cmp::Ordering;

/// The values reached from the focus along the paired `path` — the property paired against the
/// value nodes.
fn other_values<G: RdfGraph>(ctx: &Ctx<'_, G>, path: &Path) -> Vec<Term> {
    reach(ctx.graph, ctx.focus, path).into_iter().collect()
}

/// `sh:EqualsConstraintComponent`. The value-node set must equal the paired property's value set;
/// one result per term in the symmetric difference (`sh:value` = that term).
pub struct EqualsValidator {
    /// The paired property path.
    pub path: Path,
}

impl<G: RdfGraph> Validator<G> for EqualsValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#EqualsConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.path);
        // V ∖ O, then O ∖ V — each offending term yields one result.
        for v in value_nodes {
            if !other.contains(v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("EqualsConstraintComponent"),
                ));
            }
        }
        for o in &other {
            if !value_nodes.contains(o) {
                out.push(result_for(
                    ctx,
                    Some(o.clone()),
                    comp("EqualsConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:DisjointConstraintComponent`. The value nodes and the paired property's values must share no
/// term; one result per shared term.
pub struct DisjointValidator {
    /// The paired property path.
    pub path: Path,
}

impl<G: RdfGraph> Validator<G> for DisjointValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DisjointConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.path);
        for v in value_nodes {
            if other.contains(v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("DisjointConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:SubsetOfConstraintComponent` (new in 1.2). Every value node must also be a value of the
/// paired property; one result per value node that is not.
pub struct SubsetOfValidator {
    /// The paired property path (the superset).
    pub path: Path,
}

impl<G: RdfGraph> Validator<G> for SubsetOfValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SubsetOfConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.path);
        for v in value_nodes {
            if !other.contains(v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("SubsetOfConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:LessThanConstraintComponent` / `sh:LessThanOrEqualsConstraintComponent`. Each value node must
/// compare strictly-less-than (resp. less-than-or-equal) every value of the paired property. A value
/// node that fails any pair — including an incomparable (type-error) pair — yields one result.
pub struct LessThanValidator {
    /// The paired property path.
    pub path: Path,
    /// `true` for `sh:lessThanOrEquals`, `false` for `sh:lessThan`.
    pub or_equals: bool,
}

impl<G: RdfGraph> Validator<G> for LessThanValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        if self.or_equals {
            NamedNodeRef::new_unchecked(
                "http://www.w3.org/ns/shacl#LessThanOrEqualsConstraintComponent",
            )
        } else {
            NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#LessThanConstraintComponent")
        }
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.path);
        let component = if self.or_equals {
            "LessThanOrEqualsConstraintComponent"
        } else {
            "LessThanConstraintComponent"
        };
        // One result per failing (value node, paired value) pair (§7.6.4–5), with the value node as
        // `sh:value`. A pair fails if the value node is not strictly-less-than (resp. ≤) the paired
        // value, including the incomparable (SPARQL type-error) case.
        for v in value_nodes {
            for o in &other {
                let ok = match compare(v, o) {
                    Some(Ordering::Less) => true,
                    Some(Ordering::Equal) => self.or_equals,
                    _ => false,
                };
                if !ok {
                    out.push(result_for(ctx, Some(v.clone()), comp(component)));
                }
            }
        }
    }
}

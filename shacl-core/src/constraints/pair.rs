//! Property-pair constraint components (§7.6, property shapes only). Each compares this shape's
//! value nodes against the values of a sibling property on the *same focus node*:
//! `sh:equals` (§7.6.1), `sh:disjoint` (§7.6.2), `sh:subsetOf` (§7.6.3, new in 1.2),
//! `sh:lessThan` (§7.6.4), `sh:lessThanOrEquals` (§7.6.5).
//!
//! The "other" values are the objects of `(focus, predicate, *)` in the data graph.

use super::range::compare;
use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNode, NamedNodeRef, Term};
use std::cmp::Ordering;

/// The values of `(focus, predicate, *)` — the property paired against the value nodes.
fn other_values<G: RdfGraph>(ctx: &Ctx<'_, G>, predicate: &NamedNode) -> Vec<Term> {
    ctx.graph
        .objects(ctx.focus, predicate)
        .into_iter()
        .collect()
}

/// `sh:EqualsConstraintComponent`. The value-node set must equal the paired property's value set;
/// one result per term in the symmetric difference (`sh:value` = that term).
pub struct EqualsValidator {
    /// The paired predicate.
    pub predicate: NamedNode,
}

impl<G: RdfGraph> Validator<G> for EqualsValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#EqualsConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.predicate);
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
    /// The paired predicate.
    pub predicate: NamedNode,
}

impl<G: RdfGraph> Validator<G> for DisjointValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DisjointConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.predicate);
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
    /// The paired predicate (the superset).
    pub predicate: NamedNode,
}

impl<G: RdfGraph> Validator<G> for SubsetOfValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SubsetOfConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let other = other_values(ctx, &self.predicate);
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
    /// The paired predicate.
    pub predicate: NamedNode,
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
        let other = other_values(ctx, &self.predicate);
        let component = if self.or_equals {
            "LessThanOrEqualsConstraintComponent"
        } else {
            "LessThanConstraintComponent"
        };
        for v in value_nodes {
            let ok = other.iter().all(|o| match compare(v, o) {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => self.or_equals,
                _ => false, // Greater, or incomparable (type error) → fails.
            });
            if !ok {
                out.push(result_for(ctx, Some(v.clone()), comp(component)));
            }
        }
    }
}

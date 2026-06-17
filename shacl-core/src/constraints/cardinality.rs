//! Cardinality constraint components (§7.2): `sh:minCount` (§7.2.1, `CMP-MINCOUNT`) and
//! `sh:maxCount` (§7.2.2, `CMP-MAXCOUNT`).
//!
//! Both apply to property shapes only and produce **at most one** result per focus node — the
//! violation is about the *count* of value nodes, not any individual value, so the result carries
//! **no** `sh:value` (`REQ-MINCOUNT-1`). The count is over distinct value nodes (`REQ-MINCOUNT-2`),
//! which the engine has already deduplicated when it computes the value-node set (§5).

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNodeRef, Term};

/// `sh:MinCountConstraintComponent`. `REQ-MINCOUNT-1..3`.
///
/// Violated iff the number of (distinct) value nodes is **less** than [`Self::min`]. A `min` of 0
/// (or any non-positive value) can never be violated (`REQ-MINCOUNT-3`): an empty value-node set
/// has length 0, which is not `< 0`.
pub struct MinCountValidator {
    /// The minimum number of distinct value nodes required (`sh:minCount`).
    pub min: i64,
}

impl<G: RdfGraph> Validator<G> for MinCountValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinCountConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        if (value_nodes.len() as i64) < self.min {
            // REQ-MINCOUNT-1: exactly one result, with no sh:value (the violation is absence).
            out.push(result_for(ctx, None, comp("MinCountConstraintComponent")));
        }
    }
}

/// `sh:MaxCountConstraintComponent`. `CMP-MAXCOUNT` (§7.2.2).
///
/// Violated iff the number of (distinct) value nodes is **greater** than [`Self::max`]. Like
/// `sh:minCount`, the result carries no `sh:value`.
pub struct MaxCountValidator {
    /// The maximum number of distinct value nodes permitted (`sh:maxCount`).
    pub max: i64,
}

impl<G: RdfGraph> Validator<G> for MaxCountValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxCountConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        if (value_nodes.len() as i64) > self.max {
            out.push(result_for(ctx, None, comp("MaxCountConstraintComponent")));
        }
    }
}

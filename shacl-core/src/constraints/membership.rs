//! Value-membership constraint components (§7.9): `sh:hasValue` (§7.9.2, `CMP-HASVALUE`) and
//! `sh:in` (§7.9.3, `CMP-IN`).

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNodeRef, Term};

/// `sh:HasValueConstraintComponent`. At least one value node must equal the given term; otherwise a
/// single result (with no `sh:value` — the violation is the focus lacking the value).
pub struct HasValueValidator {
    /// The term that must appear among the value nodes.
    pub value: Term,
}

impl<G: RdfGraph> Validator<G> for HasValueValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#HasValueConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        if !value_nodes.contains(&self.value) {
            out.push(result_for(ctx, None, comp("HasValueConstraintComponent")));
        }
    }
}

/// `sh:InConstraintComponent`. Every value node must be a member of the enumerated set; one result
/// per offending value node, carrying it as `sh:value`.
pub struct InValidator {
    /// The admitted set of terms (the flattened `rdf:List` value of `sh:in`).
    pub members: Vec<Term>,
}

impl<G: RdfGraph> Validator<G> for InValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#InConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !self.members.contains(v) {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("InConstraintComponent"),
                ));
            }
        }
    }
}

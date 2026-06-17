//! RDF-list constraint components (§7.5, all new in 1.2): `sh:minListLength`/`sh:maxListLength`
//! (§7.5.2–3, `CMP-LISTLEN-*`) and `sh:uniqueMembers` (§7.5.4, `CMP-UNIQUEMEMBERS`).
//!
//! Each value node is expected to be the head of a well-formed `rdf:List`; a value node that is not
//! a well-formed list violates these constraints (its length / uniqueness is undefined).
//! `sh:memberShape` (§7.5.1) recurses into a shape and is wired with the §7.7–7.8 shape components
//! once the recursion guard (ADR-002) lands.

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNode, NamedNodeRef, Term};
use std::collections::HashSet;

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

fn rdf(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{RDF}{local}"))
}

/// Walk an `rdf:List` from `head`, returning its members in order, or `None` if it is not a
/// well-formed list (a node missing exactly one `rdf:first`/`rdf:rest`, or a cyclic `rdf:rest`
/// chain — detected with a visited set so malformed data terminates).
pub fn rdf_list<G: RdfGraph + ?Sized>(graph: &G, head: &Term) -> Option<Vec<Term>> {
    let nil = Term::NamedNode(rdf("nil"));
    let (first_p, rest_p) = (rdf("first"), rdf("rest"));
    let mut out = Vec::new();
    let mut visited: HashSet<Term> = HashSet::new();
    let mut cur = head.clone();
    while cur != nil {
        if !visited.insert(cur.clone()) {
            return None; // cycle in rdf:rest
        }
        let firsts = graph.objects(&cur, &first_p);
        let rests = graph.objects(&cur, &rest_p);
        if firsts.len() != 1 || rests.len() != 1 {
            return None; // not a well-formed list cell
        }
        out.push(firsts.into_iter().next()?);
        cur = rests.into_iter().next()?;
    }
    Some(out)
}

/// `sh:MinListLengthConstraintComponent` / `sh:MaxListLengthConstraintComponent`. A value node must
/// be a well-formed list whose member count is within the bound.
pub struct ListLengthValidator {
    /// The bound value.
    pub bound: i64,
    /// `true` = minimum (`sh:minListLength`), `false` = maximum (`sh:maxListLength`).
    pub is_min: bool,
}

impl<G: RdfGraph> Validator<G> for ListLengthValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        if self.is_min {
            NamedNodeRef::new_unchecked(
                "http://www.w3.org/ns/shacl#MinListLengthConstraintComponent",
            )
        } else {
            NamedNodeRef::new_unchecked(
                "http://www.w3.org/ns/shacl#MaxListLengthConstraintComponent",
            )
        }
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let component = if self.is_min {
            "MinListLengthConstraintComponent"
        } else {
            "MaxListLengthConstraintComponent"
        };
        for v in value_nodes {
            let ok = rdf_list(ctx.graph, v).is_some_and(|m| {
                let len = m.len() as i64;
                if self.is_min {
                    len >= self.bound
                } else {
                    len <= self.bound
                }
            });
            if !ok {
                out.push(result_for(ctx, Some(v.clone()), comp(component)));
            }
        }
    }
}

/// `sh:UniqueMembersConstraintComponent`. When enabled, a value node's list members must be
/// pairwise distinct; a value node that is not a well-formed list, or has a duplicate member,
/// yields one result.
pub struct UniqueMembersValidator;

impl<G: RdfGraph> Validator<G> for UniqueMembersValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#UniqueMembersConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let unique = rdf_list(ctx.graph, v).is_some_and(|m| {
                let mut seen = HashSet::new();
                m.iter().all(|t| seen.insert(t.clone()))
            });
            if !unique {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("UniqueMembersConstraintComponent"),
                ));
            }
        }
    }
}

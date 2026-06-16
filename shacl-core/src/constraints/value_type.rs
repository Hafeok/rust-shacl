//! Value-type constraint components (§7.1): `sh:class` (§7.1.1, `CMP-CLASS`), `sh:datatype`
//! (§7.1.2, `CMP-DATATYPE`), `sh:nodeKind` (§7.1.3, `CMP-NODEKIND`).
//!
//! `nodeKind` is fully implemented — it is pure term-kind inspection (`REQ-NODEKIND-1`), the
//! simplest component, and the one chosen to wire up validator dispatch + report construction
//! end-to-end first (§11.5 step 6). `class` and `datatype` are sketched against their packets.

use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNode, NamedNodeRef, NodeKind, Term};

const SH: &str = "http://www.w3.org/ns/shacl#";

fn comp(name: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{name}"))
}

fn result_for(ctx: &Ctx<'_, impl RdfGraph>, value: Option<Term>, component: NamedNode)
    -> ValidationResult
{
    ValidationResult {
        focus_node: ctx.focus.clone(),
        result_path: ctx.path_sparql.clone(),
        value,
        source_constraint_component: component,
        source_shape: shape_id_of(ctx),
        severity: ctx.severity,
        messages: Vec::new(), // populated from sh:message by the engine (REQ-ING-9)
    }
}

fn shape_id_of(ctx: &Ctx<'_, impl RdfGraph>) -> shacl_model::shape::ShapeId {
    match ctx.shape {
        shacl_model::shape::Shape::Node(n) => n.id.clone(),
        shacl_model::shape::Shape::Property(p) => p.id.clone(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CMP-NODEKIND — sh:nodeKind (§7.1.3). FULLY IMPLEMENTED.
// ─────────────────────────────────────────────────────────────────────────────

/// `sh:NodeKindConstraintComponent`. `REQ-NODEKIND-1`.
pub struct NodeKindValidator {
    /// The single declared `sh:nodeKind` value (`REQ-NODEKIND-3`: exactly one).
    pub kind: NodeKind,
}

impl<G: RdfGraph> Validator<G> for NodeKindValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeKindConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !self.kind.admits(v) {
                out.push(result_for(ctx, Some(v.clone()), comp("NodeKindConstraintComponent")));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CMP-CLASS — sh:class (§7.1.1). SKETCH — uses helpers::is_shacl_instance.
// ─────────────────────────────────────────────────────────────────────────────

/// `sh:ClassConstraintComponent`. `REQ-CLASS-1..4`. One validator per `sh:class` value
/// (repeated values are independent conjunctive constraints, `REQ-CLASS-4`/`REQ-ING-4`).
pub struct ClassValidator {
    /// The required class.
    pub class: NamedNode,
}

impl<G: RdfGraph> Validator<G> for ClassValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#ClassConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !crate::constraints::helpers::is_shacl_instance(ctx.graph, v, &self.class) {
                out.push(result_for(ctx, Some(v.clone()), comp("ClassConstraintComponent")));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CMP-DATATYPE — sh:datatype (§7.1.2). STUB — REQ-DATATYPE-2 lexical check via oxsdatatypes.
// ─────────────────────────────────────────────────────────────────────────────

/// `sh:DatatypeConstraintComponent`. `REQ-DATATYPE-1..4`.
///
/// TODO (ADR-010): conformance requires BOTH (a) value is a literal whose datatype IRI equals
/// `self.datatype`, AND (b) the lexical form is valid for that datatype (`REQ-DATATYPE-2`). (b)
/// must use `oxsdatatypes` lexical/value-space parsing, not a naive IRI match. Stubbed until the
/// engine wiring lands so the first end-to-end test goes through NodeKind.
pub struct DatatypeValidator {
    /// The required datatype IRI (`REQ-DATATYPE-3`: exactly one).
    pub datatype: NamedNode,
}

impl<G: RdfGraph> Validator<G> for DatatypeValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DatatypeConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let conforms = match v {
                Term::Literal(lit) => {
                    let dt_matches = lit.datatype().as_str() == self.datatype.as_str();
                    // TODO REQ-DATATYPE-2: && oxsdatatypes lexical validity of lit.value() for dt.
                    dt_matches
                }
                _ => false,
            };
            if !conforms {
                out.push(result_for(ctx, Some(v.clone()), comp("DatatypeConstraintComponent")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // NodeKind is testable without a graph — admits() is pure (see term.rs unit coverage).
    #[test]
    fn nodekind_iri_admits_only_iri() {
        let iri = Term::NamedNode(NamedNode::new_unchecked("http://example.com/x"));
        assert!(NodeKind::Iri.admits(&iri));
        let lit = Term::Literal(shacl_model::term::Literal::new_simple_literal("x"));
        assert!(!NodeKind::Iri.admits(&lit));
    }
}

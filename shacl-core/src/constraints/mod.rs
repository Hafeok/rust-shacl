//! Core constraint components (§7). Each submodule implements one component group; the
//! [`registry`] maps a `sh:…ConstraintComponent` IRI to its [`crate::validator::Validator`].
//!
//! Build order within §7 (§11.5 step 6→8): value_type (nodeKind first) → cardinality → range →
//! string → pair → logical → shape → list → other. Only value_type is sketched here; the rest are
//! module stubs to be filled from their §7 packets.

pub mod cardinality; // §7.2 — CMP-MINCOUNT (worked in spec), CMP-MAXCOUNT
pub mod helpers;
pub mod value_type; // §7.1 — CMP-NODEKIND, CMP-CLASS, CMP-DATATYPE
                    // pub mod range;   // §7.3
                    // pub mod string;  // §7.4 — CMP-PATTERN (worked in spec)
                    // pub mod pair;    // §7.6
                    // pub mod logical; // §7.7 — needs recursion guard (ADR-002) before enabling
                    // pub mod shape;   // §7.8
                    // pub mod list;    // §7.5
                    // pub mod other;   // §7.9

use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::shape::Constraint;
use shacl_model::term::{NamedNode, NodeKind, Term};

/// SHACL namespace.
const SH: &str = "http://www.w3.org/ns/shacl#";

/// Build a constraint-component IRI `sh:<name>` (e.g. `sh:MinCountConstraintComponent`). Shared by
/// every component's `Validator` to stamp `sh:sourceConstraintComponent` on its results.
#[must_use]
pub(crate) fn comp(name: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{name}"))
}

/// Construct a [`ValidationResult`] from the validation context (§6.7.2): `sh:focusNode`,
/// `sh:resultPath` (property shapes), the offending `value` where applicable (`None` for
/// count-based violations like `sh:minCount`), the `component` IRI, `sh:sourceShape`, and the
/// effective severity. `sh:resultMessage` is filled from `sh:message` by the engine (`REQ-ING-9`).
#[must_use]
pub(crate) fn result_for(
    ctx: &Ctx<'_, impl RdfGraph>,
    value: Option<Term>,
    component: NamedNode,
) -> ValidationResult {
    ValidationResult {
        focus_node: ctx.focus.clone(),
        result_path: ctx.path_sparql.clone(),
        value,
        source_constraint_component: component,
        source_shape: ctx.shape.id().clone(),
        severity: ctx.severity,
        messages: Vec::new(),
    }
}

/// Build the [`Validator`]s for one declared constraint (the §7 dispatch table).
///
/// Returns possibly *several* validators when a single-parameter component repeats (independent
/// conjunctive constraints, `REQ-ING-4`/`REQ-CLASS-4`), exactly one for a well-formed
/// single-valued component, or none when the component IRI is unknown (the constraint is ignored,
/// per the open-world dispatch) or the parameter is ill-formed. Adding a component means adding one
/// arm here plus its `Validator` impl — nothing else in the engine changes.
#[must_use]
pub fn dispatch<G: RdfGraph>(c: &Constraint) -> Vec<Box<dyn Validator<G>>> {
    let comp = c.component.as_str();
    match comp.strip_prefix(SH).unwrap_or(comp) {
        // §7.1.3 — sh:nodeKind. Exactly one valid kind IRI (REQ-NODEKIND-2/3).
        "NodeKindConstraintComponent" => param_iris(c, "nodeKind")
            .into_iter()
            .filter_map(|iri| NodeKind::from_iri(&iri))
            .map(|kind| Box::new(value_type::NodeKindValidator { kind }) as Box<dyn Validator<G>>)
            .collect(),
        // §7.1.1 — sh:class. May repeat → one validator per value (REQ-CLASS-4).
        "ClassConstraintComponent" => param_iris(c, "class")
            .into_iter()
            .map(|class| Box::new(value_type::ClassValidator { class }) as Box<dyn Validator<G>>)
            .collect(),
        // §7.1.2 — sh:datatype. Exactly one (REQ-DATATYPE-3).
        "DatatypeConstraintComponent" => param_iris(c, "datatype")
            .into_iter()
            .map(|datatype| {
                Box::new(value_type::DatatypeValidator { datatype }) as Box<dyn Validator<G>>
            })
            .collect(),
        // §7.2.1 — sh:minCount. Exactly one integer (REQ-MINCOUNT), property shapes only.
        "MinCountConstraintComponent" => param_int(c, "minCount")
            .map(|min| Box::new(cardinality::MinCountValidator { min }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        // §7.2.2 — sh:maxCount. Exactly one integer (REQ-MAXCOUNT), property shapes only.
        "MaxCountConstraintComponent" => param_int(c, "maxCount")
            .map(|max| Box::new(cardinality::MaxCountValidator { max }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

/// The IRI values bound to parameter `sh:<local>` on a constraint, in declaration order.
fn param_iris(c: &Constraint, local: &str) -> Vec<NamedNode> {
    let pred = format!("{SH}{local}");
    c.params
        .iter()
        .filter(|(p, _)| p.as_str() == pred)
        .filter_map(|(_, v)| match v {
            Term::NamedNode(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

/// The first integer value bound to parameter `sh:<local>` (e.g. `sh:minCount`), parsed from its
/// literal lexical form. Single-valued integer parameters are exactly-one per shape (`REQ-ING-5`);
/// a missing or non-integer value yields `None`, so the component is silently skipped.
fn param_int(c: &Constraint, local: &str) -> Option<i64> {
    let pred = format!("{SH}{local}");
    c.params
        .iter()
        .find(|(p, _)| p.as_str() == pred)
        .and_then(|(_, v)| match v {
            Term::Literal(lit) => lit.value().parse::<i64>().ok(),
            _ => None,
        })
}

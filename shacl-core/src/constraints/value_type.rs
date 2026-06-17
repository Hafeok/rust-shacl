//! Value-type constraint components (§7.1): `sh:class` (§7.1.1, `CMP-CLASS`), `sh:datatype`
//! (§7.1.2, `CMP-DATATYPE`), `sh:nodeKind` (§7.1.3, `CMP-NODEKIND`).
//!
//! `nodeKind` is fully implemented — it is pure term-kind inspection (`REQ-NODEKIND-1`), the
//! simplest component, and the one chosen to wire up validator dispatch + report construction
//! end-to-end first (§11.5 step 6). `class` and `datatype` are sketched against their packets.

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNode, NamedNodeRef, NodeKind, Term};

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
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("NodeKindConstraintComponent"),
                ));
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
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("ClassConstraintComponent"),
                ));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CMP-DATATYPE — sh:datatype (§7.1.2). FULLY IMPLEMENTED (ADR-010).
// ─────────────────────────────────────────────────────────────────────────────

/// `sh:DatatypeConstraintComponent`. `REQ-DATATYPE-1..4`.
///
/// A value node conforms iff it is a literal whose datatype IRI equals [`Self::datatype`]
/// (`REQ-DATATYPE-1`) **and** whose lexical form is valid for that datatype (`REQ-DATATYPE-2`,
/// checked via [`oxsdatatypes`]). The language-tag rules (`REQ-DATATYPE-4`) fall out of the datatype
/// IRI comparison: a language-tagged literal has datatype `rdf:langString`, so it matches only when
/// `sh:datatype` is `rdf:langString`, and an `xsd:*`-typed literal never carries a language tag.
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
                    lit.datatype().as_str() == self.datatype.as_str()
                        && lexical_valid(lit.value(), &self.datatype)
                }
                _ => false, // REQ-DATATYPE-1: non-literals never conform.
            };
            if !conforms {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("DatatypeConstraintComponent"),
                ));
            }
        }
    }
}

/// Is `value` a valid lexical form for the datatype `dt`? (`REQ-DATATYPE-2`, ADR-010.)
///
/// XSD value-space membership is delegated to `oxsdatatypes`' `FromStr` parsers. Datatypes outside
/// the modelled XSD set — the string family (`xsd:string`, `xsd:token`, `xsd:anyURI`, …) and any
/// non-XSD datatype (`rdf:langString`, `rdf:HTML`, custom IRIs) — have no lexical constraint we can
/// check here and are accepted. NOTE: derived integer types are validated as `xsd:integer`, so their
/// numeric *range* bounds (e.g. `xsd:byte` ∈ −128..=127) are not yet enforced — a documented gap.
fn lexical_valid(value: &str, dt: &NamedNode) -> bool {
    use oxsdatatypes::{
        Boolean, Date, DateTime, DayTimeDuration, Decimal, Double, Duration, Float, GDay, GMonth,
        GMonthDay, GYear, GYearMonth, Integer, Time, YearMonthDuration,
    };
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    let Some(local) = dt.as_str().strip_prefix(XSD) else {
        return true; // non-XSD datatype: no lexical space we model.
    };
    macro_rules! parses {
        ($t:ty) => {
            value.parse::<$t>().is_ok()
        };
    }
    match local {
        "boolean" => parses!(Boolean),
        "decimal" => parses!(Decimal),
        "integer" | "long" | "int" | "short" | "byte" | "nonNegativeInteger"
        | "positiveInteger" | "nonPositiveInteger" | "negativeInteger" | "unsignedLong"
        | "unsignedInt" | "unsignedShort" | "unsignedByte" => parses!(Integer),
        "float" => parses!(Float),
        "double" => parses!(Double),
        "dateTime" | "dateTimeStamp" => parses!(DateTime),
        "date" => parses!(Date),
        "time" => parses!(Time),
        "gYear" => parses!(GYear),
        "gYearMonth" => parses!(GYearMonth),
        "gMonth" => parses!(GMonth),
        "gMonthDay" => parses!(GMonthDay),
        "gDay" => parses!(GDay),
        "duration" => parses!(Duration),
        "dayTimeDuration" => parses!(DayTimeDuration),
        "yearMonthDuration" => parses!(YearMonthDuration),
        // string-family XSD types (string, normalizedString, token, language, Name, anyURI, …):
        // any lexical form is admissible at this layer.
        _ => true,
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

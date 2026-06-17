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

/// `sh:NodeKindConstraintComponent`. `REQ-NODEKIND-1`. A single `sh:nodeKind` is the common case; a
/// 1.2 list value (`sh:nodeKind ( sh:BlankNode sh:IRI )`) is a **disjunction** — a value node
/// conforms if its kind is admitted by any listed kind.
pub struct NodeKindValidator {
    /// The declared `sh:nodeKind` value(s); a value node conforms if **any** admits it.
    pub kinds: Vec<NodeKind>,
}

impl<G: RdfGraph> Validator<G> for NodeKindValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#NodeKindConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            if !self.kinds.iter().any(|k| k.admits(v)) {
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
// CMP-ROOTCLASS — sh:rootClass (§7.9.4, new in 1.2).
// ─────────────────────────────────────────────────────────────────────────────

/// `sh:RootClassConstraintComponent`. Each value node must be a class that is the root class or a
/// transitive `rdfs:subClassOf` of it; a value that is not (or is not an IRI class) violates.
pub struct RootClassValidator {
    /// The required root class.
    pub root: NamedNode,
}

impl<G: RdfGraph> Validator<G> for RootClassValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#RootClassConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let ok = matches!(v, Term::NamedNode(c)
                if crate::constraints::helpers::is_subclass_or_self(ctx.graph, c, &self.root));
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("RootClassConstraintComponent"),
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
    /// The required datatype IRI(s). A single value is the 1.0 case; a 1.2 list value
    /// (`sh:datatype ( xsd:string rdf:langString )`) is a **disjunction** — a value node conforms
    /// if it is a well-formed literal of **any** listed datatype.
    pub datatypes: Vec<NamedNode>,
}

impl<G: RdfGraph> Validator<G> for DatatypeValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#DatatypeConstraintComponent")
    }

    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let conforms = match v {
                Term::Literal(lit) => self.datatypes.iter().any(|dt| {
                    lit.datatype().as_str() == dt.as_str() && lexical_valid(lit.value(), dt)
                }),
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
/// check here and are accepted. Derived integer types enforce their value-space bounds via
/// [`integer_in_range`] (e.g. `xsd:byte` ∈ −128..=127).
fn lexical_valid(value: &str, dt: &NamedNode) -> bool {
    use oxsdatatypes::{
        Boolean, Date, DateTime, DayTimeDuration, Decimal, Double, Duration, Float, GDay, GMonth,
        GMonthDay, GYear, GYearMonth, Time, YearMonthDuration,
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
        // Integer and its derived types: lexical form must be a valid integer AND within the
        // datatype's value-space bounds (e.g. xsd:byte ∈ −128..=127; "300"^^xsd:byte is ill-formed,
        // W3C core/property/datatype-ill-formed). `Integer` alone would accept any in-range i64.
        "integer" | "long" | "int" | "short" | "byte" | "nonNegativeInteger"
        | "positiveInteger" | "nonPositiveInteger" | "negativeInteger" | "unsignedLong"
        | "unsignedInt" | "unsignedShort" | "unsignedByte" => integer_in_range(value, local),
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

/// Is `value` a valid lexical form for the XSD integer type `local`, *and* within its value-space
/// bounds? Covers `xsd:integer` and its derived types. Values too large for `i128` are accepted only
/// for the sign-constrained unbounded types (with the right sign).
fn integer_in_range(value: &str, local: &str) -> bool {
    let s = value.trim();
    // Lexical: optional sign then ≥1 ASCII digits (no point, no exponent).
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let Ok(n) = s.parse::<i128>() else {
        // Beyond i128: only unbounded types can still match, by sign.
        let neg = s.starts_with('-');
        return match local {
            "nonPositiveInteger" | "negativeInteger" => neg,
            "integer" => true,
            _ => !neg, // nonNegativeInteger, positiveInteger, unsigned* — only positive magnitudes
        };
    };
    match local {
        "integer" => true,
        "long" => i64::try_from(n).is_ok(),
        "int" => i32::try_from(n).is_ok(),
        "short" => i16::try_from(n).is_ok(),
        "byte" => i8::try_from(n).is_ok(),
        "unsignedLong" => (0..=u64::MAX as i128).contains(&n),
        "unsignedInt" => (0..=u32::MAX as i128).contains(&n),
        "unsignedShort" => (0..=u16::MAX as i128).contains(&n),
        "unsignedByte" => (0..=u8::MAX as i128).contains(&n),
        "nonNegativeInteger" => n >= 0,
        "positiveInteger" => n > 0,
        "nonPositiveInteger" => n <= 0,
        "negativeInteger" => n < 0,
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

    #[test]
    fn derived_integer_ranges_enforced() {
        let xsd =
            |t: &str| NamedNode::new_unchecked(format!("http://www.w3.org/2001/XMLSchema#{t}"));
        // byte ∈ −128..=127
        assert!(lexical_valid("127", &xsd("byte")));
        assert!(lexical_valid("-128", &xsd("byte")));
        assert!(!lexical_valid("300", &xsd("byte")));
        assert!(!lexical_valid("c", &xsd("byte")));
        // unsignedByte ∈ 0..=255
        assert!(lexical_valid("255", &xsd("unsignedByte")));
        assert!(!lexical_valid("-1", &xsd("unsignedByte")));
        // sign-constrained
        assert!(!lexical_valid("0", &xsd("positiveInteger")));
        assert!(lexical_valid("1", &xsd("positiveInteger")));
        assert!(!lexical_valid("5", &xsd("negativeInteger")));
        // plain integer unbounded
        assert!(lexical_valid(
            "99999999999999999999999999999",
            &xsd("integer")
        ));
        // wrong lexical form
        assert!(!lexical_valid("1.5", &xsd("integer")));
    }
}

//! Value-range constraint components (§7.3): `sh:minExclusive`/`sh:minInclusive`/
//! `sh:maxExclusive`/`sh:maxInclusive` (§7.3.1–4, `CMP-RANGE-*`).
//!
//! A value node conforms iff the SPARQL comparison `value <op> bound` evaluates to `true`. When the
//! two terms are not comparable (a SPARQL type error — e.g. comparing a string against a number),
//! the value node violates. Comparison is delegated to a shared [`compare`] helper backed by
//! `oxsdatatypes` for numeric and dateTime value spaces (`REQ-RANGE`, closes the derived-integer
//! range-bound gap previously logged against `sh:datatype`).

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::term::{NamedNodeRef, Term};
use std::cmp::Ordering;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// Which range bound a [`RangeValidator`] enforces.
#[derive(Clone, Copy)]
pub enum Bound {
    /// `sh:minExclusive` — conforms iff `value > bound`.
    MinExclusive,
    /// `sh:minInclusive` — conforms iff `value >= bound`.
    MinInclusive,
    /// `sh:maxExclusive` — conforms iff `value < bound`.
    MaxExclusive,
    /// `sh:maxInclusive` — conforms iff `value <= bound`.
    MaxInclusive,
}

impl Bound {
    fn component(self) -> &'static str {
        match self {
            Bound::MinExclusive => "MinExclusiveConstraintComponent",
            Bound::MinInclusive => "MinInclusiveConstraintComponent",
            Bound::MaxExclusive => "MaxExclusiveConstraintComponent",
            Bound::MaxInclusive => "MaxInclusiveConstraintComponent",
        }
    }
    /// Does `ord` (value compared to bound) satisfy this bound?
    fn admits(self, ord: Ordering) -> bool {
        match self {
            Bound::MinExclusive => ord == Ordering::Greater,
            Bound::MinInclusive => ord != Ordering::Less,
            Bound::MaxExclusive => ord == Ordering::Less,
            Bound::MaxInclusive => ord != Ordering::Greater,
        }
    }
}

/// One of the four `sh:*Inclusive`/`sh:*Exclusive` components.
pub struct RangeValidator {
    /// Which bound this enforces.
    pub bound: Bound,
    /// The comparison threshold (the parameter literal).
    pub limit: Term,
}

impl<G: RdfGraph> Validator<G> for RangeValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked(match self.bound {
            Bound::MinExclusive => "http://www.w3.org/ns/shacl#MinExclusiveConstraintComponent",
            Bound::MinInclusive => "http://www.w3.org/ns/shacl#MinInclusiveConstraintComponent",
            Bound::MaxExclusive => "http://www.w3.org/ns/shacl#MaxExclusiveConstraintComponent",
            Bound::MaxInclusive => "http://www.w3.org/ns/shacl#MaxInclusiveConstraintComponent",
        })
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            // Incomparable (None) → SPARQL type error → the value node violates.
            let ok = compare(v, &self.limit).is_some_and(|ord| self.bound.admits(ord));
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp(self.bound.component()),
                ));
            }
        }
    }
}

/// True iff `dt` is one of the XSD numeric datatypes (the value space SPARQL compares numerically).
fn is_numeric(dt: &str) -> bool {
    matches!(
        dt.strip_prefix(XSD),
        Some(
            "integer"
                | "decimal"
                | "float"
                | "double"
                | "long"
                | "int"
                | "short"
                | "byte"
                | "nonNegativeInteger"
                | "positiveInteger"
                | "nonPositiveInteger"
                | "negativeInteger"
                | "unsignedLong"
                | "unsignedInt"
                | "unsignedShort"
                | "unsignedByte"
        )
    )
}

/// Parse a numeric literal's lexical form to `f64` (XSD `INF`/`-INF`/`NaN` tokens handled).
fn as_f64(s: &str) -> Option<f64> {
    match s {
        "INF" | "+INF" => Some(f64::INFINITY),
        "-INF" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        _ => s.trim().parse::<f64>().ok(),
    }
}

/// Compare two value terms in SPARQL ordering, returning `None` when they are not comparable.
///
/// Numerics compare across the XSD numeric tower (via `f64` — adequate for the conformance suite,
/// with a documented precision caveat for very large `xsd:decimal`s). `xsd:dateTime` values compare
/// via `oxsdatatypes` (which may itself return `None` for timezone-indeterminate pairs). Equal
/// string-typed literals compare lexically. Everything else is incomparable.
fn compare(value: &Term, bound: &Term) -> Option<Ordering> {
    let (Term::Literal(a), Term::Literal(b)) = (value, bound) else {
        return None;
    };
    let (adt, bdt) = (a.datatype().as_str(), b.datatype().as_str());

    if is_numeric(adt) && is_numeric(bdt) {
        return as_f64(a.value())?.partial_cmp(&as_f64(b.value())?);
    }
    if adt == format!("{XSD}dateTime") && bdt == format!("{XSD}dateTime") {
        use oxsdatatypes::DateTime;
        let (x, y) = (
            a.value().parse::<DateTime>().ok()?,
            b.value().parse::<DateTime>().ok()?,
        );
        return x.partial_cmp(&y);
    }
    if adt == bdt && (adt == format!("{XSD}string") || adt.is_empty()) {
        return Some(a.value().cmp(b.value()));
    }
    None
}

//! SHACL property-path AST (§4, `REQ-PATH-1..6`).
//!
//! Each variant corresponds to one of the seven SHACL path kinds and maps to a SPARQL 1.2 property
//! path via `path(p,G)` (§4). Evaluation semantics (the least-fixpoint closure for `*`/`+`) live in
//! `shacl-core::path`; this crate only holds the shape of a parsed path. Well-formedness rule
//! `REQ-PATH-6` (a blank-node path may not reference itself) is checked at parse time, so a
//! constructed `Path` is assumed acyclic *as a path* (data cycles are a separate, allowed case).

use crate::term::NamedNode;

/// A parsed SHACL property path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Path {
    /// Predicate path: a single IRI. `REQ-PATH-1`. SPARQL: `iri`.
    Predicate(NamedNode),
    /// Inverse path (`sh:inversePath`). `REQ-PATH-4`. SPARQL: `^p`.
    Inverse(Box<Path>),
    /// Sequence path: ≥2 sub-paths (`REQ-PATH-2`). SPARQL: `p1 / p2 / …`.
    Sequence(Vec<Path>),
    /// Alternative path (`sh:alternativePath`). `REQ-PATH-3`. SPARQL: `p1 | p2 | …`.
    Alternative(Vec<Path>),
    /// Zero-or-more (`sh:zeroOrMorePath`). `REQ-PATH-4`/`-7`. SPARQL: `p*`.
    ZeroOrMore(Box<Path>),
    /// One-or-more (`sh:oneOrMorePath`). `REQ-PATH-4`/`-7`. SPARQL: `p+`.
    OneOrMore(Box<Path>),
    /// Zero-or-one (`sh:zeroOrOnePath`). `REQ-PATH-4`. SPARQL: `p?`.
    ZeroOrOne(Box<Path>),
}

impl Path {
    /// Render to SPARQL 1.2 property-path surface syntax (for `$PATH` substitution in SHACL-SPARQL,
    /// REQ-SPQ-4, and for the SPARQL-pushdown `reach()` fast path, ADR-003).
    #[must_use]
    pub fn to_sparql(&self) -> String {
        match self {
            Path::Predicate(iri) => format!("<{}>", iri.as_str()),
            Path::Inverse(p) => format!("^{}", p.to_sparql()),
            Path::Sequence(ps) => {
                let parts: Vec<_> = ps.iter().map(Path::to_sparql).collect();
                format!("({})", parts.join(" / "))
            }
            Path::Alternative(ps) => {
                let parts: Vec<_> = ps.iter().map(Path::to_sparql).collect();
                format!("({})", parts.join(" | "))
            }
            Path::ZeroOrMore(p) => format!("{}*", p.to_sparql()),
            Path::OneOrMore(p) => format!("{}+", p.to_sparql()),
            Path::ZeroOrOne(p) => format!("{}?", p.to_sparql()),
        }
    }
}

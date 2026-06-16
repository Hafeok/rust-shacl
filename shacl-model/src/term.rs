//! RDF term layer (ADR-004). Re-exports `oxrdf` (`rdf-12`) so the rest of the workspace shares one
//! term representation and stays insulated from the AT-RISK `rdf:TripleTerm`/`rdf:tt*` vocabulary
//! (we touch triple terms only through the typed API, never by IRI string-match).

pub use oxrdf::{
    BlankNode, BlankNodeRef, Literal, LiteralRef, NamedNode, NamedNodeRef, Term, TermRef, Triple,
    TripleRef,
};

/// A node usable in subject position (IRI or blank node).
pub use oxrdf::NamedOrBlankNode;

/// SHACL node kinds (§7.1.3, `sh:NodeKind`). Pure term-shape classification; see
/// `CMP-NODEKIND` / `REQ-NODEKIND-1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// `sh:IRI`
    Iri,
    /// `sh:BlankNode`
    BlankNode,
    /// `sh:Literal`
    Literal,
    /// `sh:BlankNodeOrIRI`
    BlankNodeOrIri,
    /// `sh:BlankNodeOrLiteral`
    BlankNodeOrLiteral,
    /// `sh:IRIOrLiteral`
    IriOrLiteral,
}

impl NodeKind {
    /// True iff `term`'s kind is admitted by this node kind. `REQ-NODEKIND-1`.
    #[must_use]
    pub fn admits(self, term: &Term) -> bool {
        let (is_iri, is_blank, is_lit) = match term {
            Term::NamedNode(_) => (true, false, false),
            Term::BlankNode(_) => (false, true, false),
            Term::Literal(_) => (false, false, true),
            // RDF 1.2 triple terms are not value nodes that any sh:nodeKind admits in Core.
            #[allow(unreachable_patterns)]
            _ => (false, false, false),
        };
        match self {
            NodeKind::Iri => is_iri,
            NodeKind::BlankNode => is_blank,
            NodeKind::Literal => is_lit,
            NodeKind::BlankNodeOrIri => is_iri || is_blank,
            NodeKind::BlankNodeOrLiteral => is_blank || is_lit,
            NodeKind::IriOrLiteral => is_iri || is_lit,
        }
    }
}

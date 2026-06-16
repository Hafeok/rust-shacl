//! The backend seam (ADR-003, §11.2). Two traits decouple the engine from any store.
//!
//! - [`RdfGraph`]: pattern access via [`RdfGraph::triples`]. Sufficient for all Core constraints
//!   (§7), targets (§3.1.3), and path evaluation (§4). `REQ-ARCH-2`: paths run over this alone.
//! - [`SparqlGraph`]: adds SPARQL `select`/`ask` with pre-bound variables. Required only by
//!   `shacl-sparql` (§8) and optionally by `sh:targetWhere` pushdown.
//!
//! The split is load-bearing for conformance levels (§1.2): L1 needs only [`RdfGraph`].

use indexmap::IndexSet;
use shacl_model::path::Path;
use shacl_model::term::{NamedNode, Term};

/// A set of RDF terms (dedup + deterministic order for reproducible reports). Path-evaluation
/// results are sets (`REQ-PATH-8`).
///
/// Backed by [`IndexSet`] rather than `BTreeSet` because oxrdf's [`Term`] is `Hash + Eq` but not
/// `Ord`; insertion order gives the determinism the spec asks for, and result comparison is
/// graph-isomorphic (`REQ-TS-2`) so ordering is never semantically load-bearing.
pub type NodeSet = IndexSet<Term>;

/// Pattern access over an RDF graph. `None` in any position is a wildcard.
///
/// This single primitive backs the entire Core engine. Implementors live in `shacl-oxigraph`
/// (an in-memory graph for tests, and an `oxigraph::Store` adapter).
pub trait RdfGraph {
    /// Iterator type returned by [`RdfGraph::triples`].
    type Iter<'a>: Iterator<Item = Triple>
    where
        Self: 'a;

    /// All triples matching the (subject, predicate, object) pattern; `None` = wildcard.
    fn triples(
        &self,
        subject: Option<&Term>,
        predicate: Option<&NamedNode>,
        object: Option<&Term>,
    ) -> Self::Iter<'_>;

    /// Object set of `(subject, predicate, *)`. Convenience over [`RdfGraph::triples`].
    fn objects(&self, subject: &Term, predicate: &NamedNode) -> NodeSet {
        self.triples(Some(subject), Some(predicate), None)
            .map(|t| t.object)
            .collect()
    }

}

/// Optional backend capability: push a whole path closure down as one native query (ADR-003,
/// `REQ-ARCH-3`). Kept as a **separate** trait so the stable [`RdfGraph`] abstraction does not
/// depend on any concrete evaluation policy (Stable Dependencies / Dependency Inversion): the
/// generic path evaluator ([`crate::path::reach`]) decides whether to use this fast path, rather
/// than the trait baking one in.
///
/// Backends that can do property-path pushdown (e.g. SPARQL stores) implement this; pure
/// triple-pattern backends (e.g. the in-memory test graph) do not, and the evaluator falls back to
/// the least-fixpoint closure over [`RdfGraph::triples`].
pub trait PathReach: RdfGraph {
    /// Value nodes reachable from `start` along `path`, computed natively.
    fn reach_native(&self, start: &Term, path: &Path) -> NodeSet;
}

/// A simple owned triple yielded by [`RdfGraph::triples`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triple {
    /// Subject term.
    pub subject: Term,
    /// Predicate IRI.
    pub predicate: NamedNode,
    /// Object term.
    pub object: Term,
}

/// Pre-bound variable bindings for SHACL-SPARQL (§8.4). Opaque here; defined where it is built.
#[derive(Debug, Default, Clone)]
pub struct Bindings {
    /// (variable name, value) pairs to pre-bind. Pre-binding is Values-Insertion over the algebra
    /// (`REQ-SPQ-16`, ADR-008), not surface-string editing — the backend interprets these.
    pub pairs: Vec<(String, Term)>,
}

/// SPARQL solution set (list of variable→term maps). Concrete shape filled in by the backend.
pub type Solutions = Vec<Vec<(String, Term)>>;

/// Errors a SPARQL backend may raise.
#[derive(Debug)]
pub enum EngineError {
    /// The query string did not parse as valid SPARQL 1.2 (`REQ-SPQ-1` → failure).
    Parse(String),
    /// A pre-binding restriction was violated (`REQ-SPQ-15` → failure).
    PreBindingViolation(String),
    /// Backend-specific execution error.
    Backend(String),
}

/// Adds SPARQL evaluation to [`RdfGraph`]. Implemented only by SPARQL-capable backends.
pub trait SparqlGraph: RdfGraph {
    /// Evaluate a SELECT query with the given pre-bound variables (§8.4).
    fn select(&self, query: &str, prebound: &Bindings) -> Result<Solutions, EngineError>;
    /// Evaluate an ASK query with the given pre-bound variables.
    fn ask(&self, query: &str, prebound: &Bindings) -> Result<bool, EngineError>;
}

// Re-export Triple's term type aliases for downstream ergonomics.
pub use shacl_model::term::Term as TermAlias;

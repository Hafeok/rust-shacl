//! In-memory `RdfGraph` for tests (build step 3, §11.5). Lets `shacl-core` be exercised against a
//! real backend without pulling oxigraph's store. A thin Vec<Triple> with linear pattern scan —
//! correctness oracle, not performance.

use shacl_core::graph::{RdfGraph, Triple};
use shacl_model::term::{NamedNode, Term};

/// Trivial in-memory triple store.
#[derive(Debug, Default, Clone)]
pub struct MemGraph {
    triples: Vec<Triple>,
}

impl MemGraph {
    /// New empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Insert a triple.
    pub fn insert(&mut self, subject: Term, predicate: NamedNode, object: Term) {
        self.triples.push(Triple {
            subject,
            predicate,
            object,
        });
    }
}

impl RdfGraph for MemGraph {
    type Iter<'a> = std::vec::IntoIter<Triple>;

    fn triples(
        &self,
        subject: Option<&Term>,
        predicate: Option<&NamedNode>,
        object: Option<&Term>,
    ) -> Self::Iter<'_> {
        let matched: Vec<Triple> = self
            .triples
            .iter()
            .filter(|t| {
                subject.is_none_or(|s| &t.subject == s)
                    && predicate.is_none_or(|p| &t.predicate == p)
                    && object.is_none_or(|o| &t.object == o)
            })
            .cloned()
            .collect();
        matched.into_iter()
    }
}

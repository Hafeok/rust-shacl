//! `oxigraph::Store`-backed graph: the L2 backend implementing both [`RdfGraph`] (pattern access)
//! and [`SparqlGraph`] (SPARQL `SELECT`/`ASK`) for SHACL-SPARQL (§8). This is the only place the
//! engine touches a SPARQL engine; `shacl-core` stays SPARQL-free (`REQ-ARCH-1`).
//!
//! Pre-binding (§8.4, ADR-008) is implemented as a `VALUES` join: the `$var` sigils SHACL uses for
//! pre-bound variables are normalised to `?var` and bound by a `VALUES` clause injected into the
//! query's group graph pattern. This is the surface-level stand-in for algebra Values-Insertion the
//! ADR prescribes — correct for the well-formed queries SHACL-SPARQL requires (it preserves
//! projection of `$this`), and a documented divergence point for adversarial pre-binding (REQ-SPQ-15
//! restrictions — MINUS/SERVICE/rebinding — are not yet enforced).

use oxigraph::io::RdfFormat;
use oxigraph::model::{GraphNameRef, NamedOrBlankNodeRef, TermRef};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use shacl_core::graph::{Bindings, EngineError, RdfGraph, SparqlGraph, Triple as CoreTriple};
use shacl_core::Solutions;
use shacl_model::term::{NamedNode, Term};

/// An `oxigraph::Store` wrapped as a SHACL backend.
pub struct OxiStore {
    store: Store,
}

impl OxiStore {
    /// Wrap an existing `oxigraph::Store` (e.g. a host application's own store).
    #[must_use]
    pub fn new(store: Store) -> Self {
        OxiStore { store }
    }

    /// Load a Turtle 1.2 document into a fresh in-memory store.
    pub fn from_turtle(turtle: &str) -> Result<Self, String> {
        let store = Store::new().map_err(|e| e.to_string())?;
        store
            .load_from_slice(RdfFormat::Turtle, turtle.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(OxiStore { store })
    }

    /// The wrapped store (for callers needing native access).
    #[must_use]
    pub fn store(&self) -> &Store {
        &self.store
    }
}

impl RdfGraph for OxiStore {
    type Iter<'a> = std::vec::IntoIter<CoreTriple>;

    fn triples(
        &self,
        subject: Option<&Term>,
        predicate: Option<&NamedNode>,
        object: Option<&Term>,
    ) -> Self::Iter<'_> {
        // A literal in subject position matches nothing.
        let subj_ref: Option<NamedOrBlankNodeRef<'_>> = match subject {
            Some(Term::NamedNode(n)) => Some(n.as_ref().into()),
            Some(Term::BlankNode(b)) => Some(b.as_ref().into()),
            Some(_) => return Vec::new().into_iter(),
            None => None,
        };
        let pred_ref = predicate.map(NamedNode::as_ref);
        let obj_ref: Option<TermRef<'_>> = object.map(Term::as_ref);

        let mut out = Vec::new();
        for quad in self.store.quads_for_pattern(
            subj_ref,
            pred_ref,
            obj_ref,
            Some(GraphNameRef::DefaultGraph),
        ) {
            let Ok(q) = quad else { continue };
            out.push(CoreTriple {
                subject: q.subject.into(),
                predicate: q.predicate,
                object: q.object,
            });
        }
        out.into_iter()
    }
}

impl SparqlGraph for OxiStore {
    fn select(&self, query: &str, prebound: &Bindings) -> Result<Solutions, EngineError> {
        let bound = apply_prebinding(query, prebound);
        match self.evaluate(&bound).map_err(EngineError::Parse)? {
            QueryResults::Solutions(iter) => {
                let mut solutions = Vec::new();
                for sol in iter {
                    let sol = sol.map_err(|e| EngineError::Backend(e.to_string()))?;
                    solutions.push(
                        sol.iter()
                            .map(|(var, term)| (var.as_str().to_string(), term.clone()))
                            .collect(),
                    );
                }
                Ok(solutions)
            }
            _ => Err(EngineError::Backend("expected SELECT results".into())),
        }
    }

    fn ask(&self, query: &str, prebound: &Bindings) -> Result<bool, EngineError> {
        let bound = apply_prebinding(query, prebound);
        match self.evaluate(&bound).map_err(EngineError::Parse)? {
            QueryResults::Boolean(b) => Ok(b),
            _ => Err(EngineError::Backend("expected ASK result".into())),
        }
    }
}

impl OxiStore {
    /// Parse and execute a SPARQL query against the store via the (non-deprecated) evaluator.
    fn evaluate(&self, query: &str) -> Result<QueryResults<'static>, String> {
        SparqlEvaluator::new()
            .parse_query(query)
            .map_err(|e| e.to_string())?
            .on_store(&self.store)
            .execute()
            .map_err(|e| e.to_string())
    }
}

/// Apply pre-binding (§8.4, ADR-008). SHACL pre-bound variables carry the `$` sigil; we normalise
/// each `$var` to a regular `?var` and bind them by injecting a `VALUES` clause into the query's
/// group graph pattern. A `VALUES` join with the singleton solution is the surface-level stand-in
/// for algebra Values-Insertion — it preserves projection of `$this` (textual term substitution does
/// not) and binds the variable everywhere it occurs.
fn apply_prebinding(query: &str, prebound: &Bindings) -> String {
    if prebound.pairs.is_empty() {
        return query.to_string();
    }
    // Normalise $var → ?var for each pre-bound name so VALUES can bind them.
    let mut q = query.to_string();
    for (name, _) in &prebound.pairs {
        q = replace_var(&q, &format!("${name}"), &format!("?{name}"));
    }
    // Build `VALUES (?a ?b) { (termA termB) }` and inject after the first group-pattern brace.
    let vars: Vec<String> = prebound
        .pairs
        .iter()
        .map(|(n, _)| format!("?{n}"))
        .collect();
    let vals: Vec<String> = prebound.pairs.iter().map(|(_, t)| t.to_string()).collect();
    let clause = format!(" VALUES ({}) {{ ({}) }} ", vars.join(" "), vals.join(" "));
    if let Some(pos) = q.find('{') {
        q.insert_str(pos + 1, &clause);
    }
    q
}

/// Replace occurrences of the variable token `needle` (e.g. `$this`) not immediately followed by an
/// identifier char, so `$this` does not match inside `$thisX`.
fn replace_var(haystack: &str, needle: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(haystack.len());
    let mut rest = haystack;
    while let Some(pos) = rest.find(needle) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + needle.len()..];
        let boundary = after
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if boundary {
            out.push_str(replacement);
        } else {
            out.push_str(needle);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

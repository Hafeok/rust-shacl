//! Property-path evaluation over [`RdfGraph`] (§4.1, `REQ-PATH-1..8`).
//!
//! Implements the compositional relation semantics `⟦p⟧ ⊆ V×V` from §4.1 in terms of
//! [`RdfGraph::triples`] only (`REQ-ARCH-2` — no SPARQL). The `*`/`+` cases delegate to
//! [`crate::closure`], the property-tested least-fixpoint core.

use crate::closure::{reachable_plus, reachable_star};
use crate::graph::{NodeSet, RdfGraph};
use shacl_model::path::Path;
use shacl_model::term::{NamedNode, Term};

/// Path-evaluation **policy** entry point. The engine calls this for every property shape's value
/// nodes. By default it is the least-fixpoint closure over [`RdfGraph::triples`] ([`eval`],
/// `REQ-PATH-7`) — correct for any backend.
///
/// The optional native fast path (`PathReach`, ADR-003) is *not* selected here, because doing so
/// generically would force `RdfGraph` to know about pushdown and re-introduce the stable→unstable
/// dependency we removed. Instead, a backend with pushdown calls its own `reach_native` directly
/// from the (less stable) `shacl-oxigraph`/`shacl-sparql` layer where the concrete type is known.
/// This keeps the stable core depending only on the `triples` primitive (SDP/DIP).
pub fn reach<G: RdfGraph + ?Sized>(graph: &G, start: &Term, path: &Path) -> NodeSet {
    eval(graph, start, path)
}

/// Core compositional path evaluation: `⟦path⟧(start) ⊆ V` over [`RdfGraph::triples`] (§4.1).
fn eval<G: RdfGraph + ?Sized>(graph: &G, start: &Term, path: &Path) -> NodeSet {
    match path {
        // ⟦iri⟧ — objects of (start, iri, *). REQ-PATH-1.
        Path::Predicate(iri) => objects(graph, start, iri),

        // ⟦inverse(p)⟧ — subjects s such that (s,start) ∈ ⟦p⟧. REQ-PATH-4.
        // For a predicate inverse this is subjects of (*, iri, start); general inverse needs the
        // relation reversed. We handle the common predicate case directly and recurse otherwise.
        Path::Inverse(inner) => match inner.as_ref() {
            Path::Predicate(iri) => subjects(graph, iri, start),
            other => invert_general(graph, start, other),
        },

        // ⟦seq(p1..pn)⟧ — relational composition. REQ-PATH-2.
        Path::Sequence(parts) => {
            let mut current: NodeSet = NodeSet::from([start.clone()]);
            for part in parts {
                let mut next = NodeSet::new();
                for node in &current {
                    next.extend(eval(graph, node, part));
                }
                current = next;
            }
            current
        }

        // ⟦alt(p1..pn)⟧ — union. REQ-PATH-3.
        Path::Alternative(parts) => {
            let mut out = NodeSet::new();
            for part in parts {
                out.extend(eval(graph, start, part));
            }
            out
        }

        // ⟦zeroOrMore(p)⟧ = reflexive-transitive closure of the one-step relation. REQ-PATH-7.
        Path::ZeroOrMore(inner) => reachable_star(start.clone(), |n: &Term| {
            eval(graph, n, inner).into_iter().collect::<Vec<_>>()
        }),

        // ⟦oneOrMore(p)⟧ = transitive closure. REQ-PATH-7.
        Path::OneOrMore(inner) => reachable_plus(start.clone(), |n: &Term| {
            eval(graph, n, inner).into_iter().collect::<Vec<_>>()
        }),

        // ⟦zeroOrOne(p)⟧ = Δ ∪ ⟦p⟧. REQ-PATH-4.
        Path::ZeroOrOne(inner) => {
            let mut out = eval(graph, start, inner);
            out.insert(start.clone());
            out
        }
    }
}

fn objects<G: RdfGraph + ?Sized>(graph: &G, subject: &Term, predicate: &NamedNode) -> NodeSet {
    graph
        .triples(Some(subject), Some(predicate), None)
        .map(|t| t.object)
        .collect()
}

fn subjects<G: RdfGraph + ?Sized>(graph: &G, predicate: &NamedNode, object: &Term) -> NodeSet {
    graph
        .triples(None, Some(predicate), Some(object))
        .map(|t| t.subject)
        .collect()
}

/// Inverse of a general (non-predicate) path: { s | start ∈ ⟦p⟧(s) }.
///
/// NOTE (`REQ-PATH-4`, partial): a fully general inverse over arbitrary sub-paths needs the inverse
/// relation, which for closures is not simply expressible by forward evaluation. v1 supports
/// inverse-of-predicate directly (the overwhelmingly common case) and inverse-of-sequence by
/// reversing+inverting components; deeper nesting (inverse of `*`/`+`) is deferred and currently
/// returns empty with a debug assertion so the gap is loud, not silent. TODO before L1 sign-off.
fn invert_general<G: RdfGraph + ?Sized>(graph: &G, start: &Term, inner: &Path) -> NodeSet {
    match inner {
        // ^(p1/p2/.../pn) = ^pn / ... / ^p1
        Path::Sequence(parts) => {
            let reversed: Vec<Path> = parts
                .iter()
                .rev()
                .map(|p| Path::Inverse(Box::new(p.clone())))
                .collect();
            eval(graph, start, &Path::Sequence(reversed))
        }
        // ^(p1|p2) = ^p1 | ^p2
        Path::Alternative(parts) => {
            let alts: Vec<Path> = parts
                .iter()
                .map(|p| Path::Inverse(Box::new(p.clone())))
                .collect();
            eval(graph, start, &Path::Alternative(alts))
        }
        // ^^p = p
        Path::Inverse(p) => eval(graph, start, p),
        _ => {
            debug_assert!(
                false,
                "inverse of {inner:?} not yet supported (REQ-PATH-4 partial)"
            );
            NodeSet::new()
        }
    }
}

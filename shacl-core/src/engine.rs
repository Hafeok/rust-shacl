//! The validation engine (§6). Ties the pieces together: resolve a shape's **targets** into focus
//! nodes (§3.1.3, `REQ-TGT-*`), compute each focus node's **value nodes** (§5), **dispatch** every
//! declared constraint to its component validator (§7), and accumulate the [`ValidationReport`]
//! (§6.7). The validators and the value-node computation are backend-agnostic ([`RdfGraph`]), so the
//! whole engine is Level-1 (`REQ-ARCH-1`: no SPARQL).
//!
//! Shape-referencing components (`sh:node`/`sh:property`/`sh:not`/`sh:and`/`sh:or`/`sh:xone`/
//! `sh:qualifiedValueShape`/`sh:memberShape`, §7.5/7.7/7.8) resolve referenced shapes through the
//! [`Registry`] carried on [`Ctx`] and recurse via [`conforms`] / [`validate_focus_collect`].
//! Termination is guaranteed by the shapes-graph recursion guard (ADR-002,
//! [`crate::recursion::shape_ref_cycle`]); a depth backstop ([`MAX_DEPTH`]) defends against any
//! cycle that slips past it.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::constraints::{dispatch, helpers::is_shacl_instance};
use crate::graph::RdfGraph;
use crate::report::{ValidationReport, ValidationResult};
use crate::validator::Ctx;
use crate::values::value_nodes;
use shacl_model::shape::{Shape, ShapeId};
use shacl_model::target::Target;
use shacl_model::term::{NamedNode, Term};

/// Maps a [`ShapeId`] to the shape it identifies, for resolving shape references at validation time.
pub type Registry<'a> = HashMap<ShapeId, &'a Shape>;

/// Recursion backstop (see module docs); deep enough never to bite an acyclic shapes graph.
const MAX_DEPTH: usize = 1024;

fn rdf_type() -> NamedNode {
    NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

/// Build a [`Registry`] indexing every shape by its [`ShapeId`].
#[must_use]
pub fn build_registry(shapes: &[Shape]) -> Registry<'_> {
    shapes.iter().map(|s| (s.id().clone(), s)).collect()
}

/// Validate the `data` graph against all `shapes`, producing a [`ValidationReport`] (§6).
///
/// For each active shape: resolve its targets to focus nodes, and validate each focus node. A
/// deactivated shape is skipped entirely (`REQ-ING-10`). Shape references resolve through a registry
/// built from `shapes`; callers that need reject-on-recursion (ADR-002) should first consult
/// [`crate::recursion::shape_ref_cycle`].
#[must_use]
pub fn validate<G: RdfGraph>(data: &G, shapes: &[Shape]) -> ValidationReport {
    let registry = build_registry(shapes);
    let mut report = ValidationReport::default();
    for shape in shapes {
        if shape.deactivated() {
            continue;
        }
        for focus in focus_nodes(data, &registry, shape) {
            report
                .results
                .extend(validate_focus_collect(data, &registry, shape, &focus, 0));
        }
    }
    report
}

/// Validate one shape against one already-selected focus node (`REQ-TGT-7`: targets are bypassed),
/// appending results to `report`. Public so direct-focus test entries (`sht:focus` + `sht:shape`,
/// `REQ-TS-3`) can call it. Shape references resolve against a registry built from `shape` alone; to
/// resolve references to *other* shapes, validate through [`validate`] (full registry) instead.
pub fn validate_focus<G: RdfGraph>(
    data: &G,
    shape: &Shape,
    focus: &Term,
    report: &mut ValidationReport,
) {
    let registry = build_registry(std::slice::from_ref(shape));
    report
        .results
        .extend(validate_focus_collect(data, &registry, shape, focus, 0));
}

/// Validate `shape` against `focus`, returning the produced results (§6, §7). The core recursion
/// point: shape-referencing components call back into this through the registry.
#[must_use]
pub fn validate_focus_collect<G: RdfGraph>(
    data: &G,
    registry: &Registry<'_>,
    shape: &Shape,
    focus: &Term,
    depth: usize,
) -> Vec<ValidationResult> {
    let mut out = Vec::new();
    if depth > MAX_DEPTH {
        return out; // recursion backstop; the SCC guard is the real protection.
    }
    let vns = value_nodes(data, shape, focus);
    let path_sparql = match shape {
        Shape::Property(p) => Some(p.path.to_sparql()),
        Shape::Node(_) => None,
    };
    for constraint in shape.constraints() {
        if constraint.deactivated {
            continue;
        }
        let severity = constraint.severity.unwrap_or_else(|| shape.severity());
        let ctx = Ctx {
            graph: data,
            focus,
            shape,
            constraint,
            severity,
            path_sparql: path_sparql.clone(),
            registry,
            depth,
        };
        for validator in dispatch::<G>(constraint) {
            validator.validate(&vns, &ctx, &mut out);
        }
    }
    out
}

/// Does `focus` **conform** to `shape`? (§7.8 conformance: validation produces *no* results,
/// regardless of severity.) The predicate used by `sh:not`/`sh:and`/`sh:or`/`sh:xone`/`sh:node`/
/// `sh:qualifiedValueShape`/`sh:memberShape`.
#[must_use]
pub fn conforms<G: RdfGraph>(
    data: &G,
    registry: &Registry<'_>,
    shape: &Shape,
    focus: &Term,
    depth: usize,
) -> bool {
    validate_focus_collect(data, registry, shape, focus, depth.saturating_add(1)).is_empty()
}

/// Resolve a [`ShapeId`] against the registry.
#[must_use]
pub fn lookup<'a>(registry: &Registry<'a>, id: &ShapeId) -> Option<&'a Shape> {
    registry.get(id).copied()
}

/// Resolve a term used as a shape reference (`sh:node` value, `sh:and` list element, …) to a
/// [`ShapeId`]: IRI → named shape, blank node → inline shape. Literals are not shape references.
#[must_use]
pub fn term_to_shape_id(t: &Term) -> Option<ShapeId> {
    match t {
        Term::NamedNode(n) => Some(ShapeId::Named(n.clone())),
        Term::BlankNode(b) => Some(ShapeId::Blank(b.as_str().to_string())),
        _ => None,
    }
}

/// Resolve a shape's target declarations to the set of focus nodes (§3.1.3, `REQ-TGT-1..6`),
/// deduplicated and order-stable.
///
/// `sh:targetWhere` (`REQ-TGT-5`, ADR-007): naive iteration — every node in the data graph that
/// *conforms* to the inner shape becomes a focus node. Explicit `sh:shape` data-graph links
/// (`REQ-TGT-6`) are resolved unconditionally for every named shape.
#[must_use]
pub fn focus_nodes<G: RdfGraph>(data: &G, registry: &Registry<'_>, shape: &Shape) -> Vec<Term> {
    let mut out: Vec<Term> = Vec::new();
    let mut seen: HashSet<Term> = HashSet::new();
    let push = |t: Term, out: &mut Vec<Term>, seen: &mut HashSet<Term>| {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    };

    // REQ-TGT-6: any data triple `(n, sh:shape, thisShape)` makes `n` a focus node.
    if let ShapeId::Named(iri) = shape.id() {
        let sh_shape = NamedNode::new_unchecked("http://www.w3.org/ns/shacl#shape");
        let obj = Term::NamedNode(iri.clone());
        for t in data.triples(None, Some(&sh_shape), Some(&obj)) {
            push(t.subject, &mut out, &mut seen);
        }
    }

    for target in shape.targets() {
        match target {
            // REQ-TGT-5: nodes conforming to the inner shape (naive iteration, ADR-007).
            Target::Where(id) => {
                if let Some(wshape) = lookup(registry, id) {
                    for node in candidate_nodes(data) {
                        if conforms(data, registry, wshape, &node, 0) {
                            push(node, &mut out, &mut seen);
                        }
                    }
                }
            }
            // REQ-TGT-1: explicit node(s).
            Target::Node(t) => push(t.clone(), &mut out, &mut seen),

            // REQ-TGT-2/3: SHACL instances of the class (incl. subclass walk). Enumerate the
            // subjects of rdf:type triples and keep those that are SHACL instances of the class.
            Target::Class(c) | Target::ImplicitClass(c) => {
                for s in typed_subjects(data) {
                    if is_shacl_instance(data, &s, c) {
                        push(s, &mut out, &mut seen);
                    }
                }
            }

            // REQ-TGT-4: subjects / objects of triples with the given predicate.
            Target::SubjectsOf(p) => {
                for t in data.triples(None, Some(p), None) {
                    push(t.subject, &mut out, &mut seen);
                }
            }
            Target::ObjectsOf(p) => {
                for t in data.triples(None, Some(p), None) {
                    push(t.object, &mut out, &mut seen);
                }
            }

            // ExplicitShape is resolved above (REQ-TGT-6), independent of declared targets.
            Target::ExplicitShape => {}
        }
    }
    out
}

/// All distinct terms appearing as a subject or object in the data graph — the candidate focus nodes
/// for `sh:targetWhere`'s naive iteration.
fn candidate_nodes<G: RdfGraph>(data: &G) -> Vec<Term> {
    let mut seen: HashSet<Term> = HashSet::new();
    let mut out = Vec::new();
    for t in data.triples(None, None, None) {
        if seen.insert(t.subject.clone()) {
            out.push(t.subject);
        }
        if seen.insert(t.object.clone()) {
            out.push(t.object);
        }
    }
    out
}

/// All distinct subjects that carry at least one `rdf:type` (candidate class instances).
fn typed_subjects<G: RdfGraph>(data: &G) -> Vec<Term> {
    let type_pred = rdf_type();
    let mut seen: HashSet<Term> = HashSet::new();
    let mut out = Vec::new();
    for t in data.triples(None, Some(&type_pred), None) {
        if seen.insert(t.subject.clone()) {
            out.push(t.subject);
        }
    }
    out
}

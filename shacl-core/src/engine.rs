//! The validation engine (§6). Ties the pieces together: resolve a shape's **targets** into focus
//! nodes (§3.1.3, `REQ-TGT-*`), compute each focus node's **value nodes** (§5), **dispatch** every
//! declared constraint to its component validator (§7), and accumulate the [`ValidationReport`]
//! (§6.7). The validators and the value-node computation are backend-agnostic ([`RdfGraph`]), so the
//! whole engine is Level-1 (`REQ-ARCH-1`: no SPARQL).

use std::collections::HashSet;

use crate::constraints::{dispatch, helpers::is_shacl_instance};
use crate::graph::RdfGraph;
use crate::report::ValidationReport;
use crate::validator::Ctx;
use crate::values::value_nodes;
use shacl_model::shape::Shape;
use shacl_model::target::Target;
use shacl_model::term::{NamedNode, Term};

fn rdf_type() -> NamedNode {
    NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}

/// Validate the `data` graph against all `shapes`, producing a [`ValidationReport`] (§6).
///
/// For each active shape: resolve its targets to focus nodes, and validate each focus node. A
/// deactivated shape is skipped entirely (`REQ-ING-10`).
#[must_use]
pub fn validate<G: RdfGraph>(data: &G, shapes: &[Shape]) -> ValidationReport {
    let mut report = ValidationReport::default();
    for shape in shapes {
        if shape.deactivated() {
            continue;
        }
        for focus in focus_nodes(data, shape) {
            validate_focus(data, shape, &focus, &mut report);
        }
    }
    report
}

/// Validate one shape against one already-selected focus node (`REQ-TGT-7`: targets are bypassed).
/// Public so direct-focus test entries (`sht:focus` + `sht:shape`, `REQ-TS-3`) can call it.
pub fn validate_focus<G: RdfGraph>(
    data: &G,
    shape: &Shape,
    focus: &Term,
    report: &mut ValidationReport,
) {
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
        };
        for validator in dispatch::<G>(constraint) {
            validator.validate(&vns, &ctx, &mut report.results);
        }
    }
}

/// Resolve a shape's target declarations to the set of focus nodes (§3.1.3, `REQ-TGT-1..6`),
/// deduplicated and order-stable.
///
/// `sh:targetWhere` (`REQ-TGT-5`, ADR-007) and explicit `sh:shape` data-graph targets (`REQ-TGT-6`)
/// need a shape registry / data-graph shape links and are wired in a later step; they contribute no
/// focus nodes yet.
#[must_use]
pub fn focus_nodes<G: RdfGraph>(data: &G, shape: &Shape) -> Vec<Term> {
    let mut out: Vec<Term> = Vec::new();
    let mut seen: HashSet<Term> = HashSet::new();
    let push = |t: Term, out: &mut Vec<Term>, seen: &mut HashSet<Term>| {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    };

    for target in shape.targets() {
        match target {
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

            // Not yet wired (see fn docs).
            Target::Where(_) | Target::ExplicitShape => {}
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

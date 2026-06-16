//! Shared helpers used across components (§11.4). Built before the components that need them.

use crate::closure::reachable_star;
use crate::graph::RdfGraph;
use shacl_model::term::{NamedNode, Term};

/// `rdf:type` IRI.
fn rdf_type() -> NamedNode {
    NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
}
/// `rdfs:subClassOf` IRI.
fn rdfs_subclassof() -> NamedNode {
    NamedNode::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subClassOf")
}

/// Is `node` a SHACL instance of `class` in the data graph? (`REQ-CLASS-2`, also used by
/// `REQ-TGT-2`, implicit targets, qualified shapes.)
///
/// Definition (§1.1): the SHACL types of `node` are its `rdf:type` values plus all SHACL
/// superclasses of those (transitive `rdfs:subClassOf`). `node` is a SHACL instance of `class`
/// iff `class` is among those types. The superclass walk is the property-tested closure
/// ([`reachable_star`]); it terminates on cyclic subclass graphs (`REQ-PATH-7`).
///
/// NOTE: by default the `rdfs:subClassOf` walk reads the **data** graph (`REQ-CLASS-3`); reading
/// it from the shapes graph instead (§6.3) is a configuration deferred past this sketch.
pub fn is_shacl_instance<G: RdfGraph + ?Sized>(graph: &G, node: &Term, class: &NamedNode) -> bool {
    let type_pred = rdf_type();
    let sub_pred = rdfs_subclassof();
    let class_term = Term::NamedNode(class.clone());

    // Direct types of `node`.
    let direct_types = graph.objects(node, &type_pred);

    // For each direct type, walk subClassOf* upward; if `class` appears, it's an instance.
    for t in &direct_types {
        let supers = reachable_star(t.clone(), |c: &Term| {
            graph
                .triples(Some(c), Some(&sub_pred), None)
                .map(|tr| tr.object)
                .collect::<Vec<_>>()
        });
        if supers.contains(&class_term) {
            return true;
        }
    }
    false
}

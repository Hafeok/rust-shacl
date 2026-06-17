//! SPARQL-based constraints (§8.1, spec §2): a shape's `sh:sparql` → `sh:select` query is executed
//! with `this` pre-bound to the focus node, and each non-`failure` solution becomes a validation
//! result (`REQ-SPQ-1..6`). Generic over [`SparqlGraph`]; the concrete engine lives in
//! `shacl-oxigraph`.

use shacl_core::graph::{Bindings, SparqlGraph};
use shacl_core::report::ValidationResult;
use shacl_model::shape::{Severity, ShapeId};
use shacl_model::term::{NamedNode, Term};

const SH: &str = "http://www.w3.org/ns/shacl#";

/// A parsed SPARQL-based constraint (the `sh:sparql`/`sh:select` value of a shape).
pub struct SelectConstraint {
    /// The `sh:select` query text (with any `PREFIX` lines already prepended, `REQ-SPQ-13`).
    pub select: String,
    /// The shape declaring the constraint (`sh:sourceShape`).
    pub source_shape: ShapeId,
    /// Effective severity for produced results.
    pub severity: Severity,
    /// Static `sh:message`, used when a solution does not bind `?message` (`REQ-SPQ-5`).
    pub message: Option<String>,
}

/// The result of evaluating a SPARQL-based constraint: either violations, or a *failure* (distinct
/// from a violation, `REQ-SPQ-3`) when a solution binds `failure` = true or the query errors.
pub enum Outcome {
    /// Zero or more validation results (one per non-`failure` solution).
    Results(Vec<ValidationResult>),
    /// A processing failure with a diagnostic message.
    Failure(String),
}

/// Evaluate one SPARQL-based constraint against one focus node (`REQ-SPQ-2..6`).
///
/// `path_sparql` is the property shape's path in SPARQL surface syntax, used to substitute the
/// `$PATH` token (`REQ-SPQ-4`); `None` for node shapes. `this` is pre-bound to `focus` (§8.4).
pub fn validate_select<G: SparqlGraph>(
    graph: &G,
    focus: &Term,
    path_sparql: Option<&str>,
    c: &SelectConstraint,
) -> Outcome {
    // REQ-SPQ-4: substitute $PATH with the path's SPARQL surface syntax (property shapes).
    let query = match path_sparql {
        Some(p) => c.select.replace("$PATH", p),
        None => c.select.clone(),
    };

    let prebound = Bindings {
        pairs: vec![("this".to_string(), focus.clone())],
    };
    let solutions = match graph.select(&query, &prebound) {
        Ok(s) => s,
        Err(e) => return Outcome::Failure(format!("{e:?}")),
    };

    let mut results = Vec::new();
    for sol in &solutions {
        let get = |name: &str| sol.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());

        // REQ-SPQ-3: a solution binding failure=true is a failure, not a violation.
        if let Some(Term::Literal(l)) = get("failure") {
            if l.value() == "true" {
                return Outcome::Failure("SPARQL constraint signalled sh:failure".to_string());
            }
        }

        // REQ-SPQ-5 result-property mapping.
        let focus_node = get("this").unwrap_or_else(|| focus.clone());
        let result_path = match get("path") {
            Some(Term::NamedNode(n)) => Some(format!("<{}>", n.as_str())),
            _ => path_sparql.map(str::to_string),
        };
        let value = get("value");
        let messages = match get("message") {
            Some(Term::Literal(l)) => vec![l.value().to_string()],
            _ => c.message.clone().into_iter().collect(),
        };

        results.push(ValidationResult {
            focus_node,
            result_path,
            value,
            source_constraint_component: NamedNode::new_unchecked(format!(
                "{SH}SPARQLConstraintComponent"
            )),
            source_shape: c.source_shape.clone(),
            severity: c.severity,
            messages,
        });
    }
    Outcome::Results(results)
}

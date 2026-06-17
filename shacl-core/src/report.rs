//! Validation report model (§6.7, `REQ-RPT-2/3`). Serialization back to RDF over a backend is a
//! later step (build step 5, §11.5); this defines the in-memory result the engine produces.

use shacl_model::shape::Severity;
use shacl_model::shape::ShapeId;
use shacl_model::term::{Literal, NamedNode, Term};

const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// A single validation result (`sh:ValidationResult`, §6.7.2).
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// `sh:focusNode` (§6.7.2.1).
    pub focus_node: Term,
    /// `sh:resultPath` (§6.7.2.2) — present for property-shape results.
    pub result_path: Option<String>,
    /// `sh:value` (§6.7.2.3) — the offending value node, where applicable (absent for e.g.
    /// `sh:minCount`, whose violation is absence — `REQ-MINCOUNT`).
    pub value: Option<Term>,
    /// `sh:sourceConstraintComponent` (§6.7.2.5).
    pub source_constraint_component: NamedNode,
    /// `sh:sourceShape` (§6.7.2.4).
    pub source_shape: ShapeId,
    /// `sh:resultSeverity` (§6.7.2.8).
    pub severity: Severity,
    /// `sh:resultMessage` (§6.7.2.7) — copied from `sh:message` if present (`REQ-ING-9`).
    pub messages: Vec<String>,
}

/// The overall report (`sh:ValidationReport`, §6.7.1).
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// All results, across shapes and focus nodes.
    pub results: Vec<ValidationResult>,
}

impl ValidationReport {
    /// `sh:conforms` (§6.7.1.1, `REQ-RPT-2`): true iff the report contains no result of severity
    /// `sh:Info`, `sh:Warning`, or `sh:Violation`. A `sh:Warning`/`sh:Info` result still makes the
    /// report non-conforming (W3C `core/misc/severity-001`), but the diagnostic-only 1.2 severities
    /// `sh:Trace` and `sh:Debug` do not (W3C `core/misc/severity-004`/`-005`).
    #[must_use]
    pub fn conforms(&self) -> bool {
        !self.results.iter().any(|r| {
            matches!(
                r.severity,
                Severity::Info | Severity::Warning | Severity::Violation
            )
        })
    }

    /// Append a result.
    pub fn push(&mut self, r: ValidationResult) {
        self.results.push(r);
    }

    /// Serialize the report to N-Triples (a Turtle subset, `REQ-RPT-2/3`, §6.7.1). The report node
    /// and each result are blank nodes; comparison against the expected graph in the test suite is
    /// graph-isomorphic (`REQ-TS-2`), so the chosen blank-node labels are not significant.
    ///
    /// `sh:resultPath` is emitted only for predicate paths (the path's SPARQL form is a single
    /// `<iri>`); complex paths (sequence/alternative/`*`/`+`/inverse) require an RDF blank-node path
    /// structure and are a documented gap — they are skipped rather than mis-serialized.
    #[must_use]
    pub fn to_ntriples(&self) -> String {
        let mut s = String::new();
        let report = "_:report";
        triple(
            &mut s,
            report,
            RDF_TYPE,
            &iri_nt(&format!("{SH}ValidationReport")),
        );
        let conforms = Literal::new_typed_literal(
            if self.conforms() { "true" } else { "false" },
            NamedNode::new_unchecked(XSD_BOOLEAN),
        );
        triple(&mut s, report, &sh_iri("conforms"), &conforms.to_string());

        for (i, r) in self.results.iter().enumerate() {
            let rn = format!("_:result{i}");
            triple(&mut s, report, &sh_iri("result"), &rn);
            triple(
                &mut s,
                &rn,
                RDF_TYPE,
                &iri_nt(&format!("{SH}ValidationResult")),
            );
            triple(&mut s, &rn, &sh_iri("focusNode"), &r.focus_node.to_string());
            if let Some(path) = r.result_path.as_deref().and_then(predicate_path_iri) {
                triple(&mut s, &rn, &sh_iri("resultPath"), &iri_nt(path));
            }
            if let Some(v) = &r.value {
                triple(&mut s, &rn, &sh_iri("value"), &v.to_string());
            }
            triple(
                &mut s,
                &rn,
                &sh_iri("resultSeverity"),
                &iri_nt(&format!("{SH}{}", severity_local(r.severity))),
            );
            triple(
                &mut s,
                &rn,
                &sh_iri("sourceConstraintComponent"),
                &iri_nt(r.source_constraint_component.as_str()),
            );
            triple(
                &mut s,
                &rn,
                &sh_iri("sourceShape"),
                &shape_id_nt(&r.source_shape),
            );
            for m in &r.messages {
                triple(
                    &mut s,
                    &rn,
                    &sh_iri("resultMessage"),
                    &Literal::new_simple_literal(m).to_string(),
                );
            }
        }
        s
    }
}

/// `sh:<local>` as an N-Triples IRI ref.
fn sh_iri(local: &str) -> String {
    iri_nt(&format!("{SH}{local}"))
}

/// Wrap an IRI string in angle brackets (N-Triples IRI ref).
fn iri_nt(iri: &str) -> String {
    format!("<{iri}>")
}

/// Emit one `s p o .` line.
fn triple(out: &mut String, s: &str, p: &str, o: &str) {
    let p = if p.starts_with('<') {
        p.to_string()
    } else {
        iri_nt(p)
    };
    out.push_str(&format!("{s} {p} {o} .\n"));
}

/// The local name of a [`Severity`]'s `sh:` IRI.
fn severity_local(sev: Severity) -> &'static str {
    match sev {
        Severity::Trace => "Trace",
        Severity::Debug => "Debug",
        Severity::Info => "Info",
        Severity::Warning => "Warning",
        Severity::Violation => "Violation",
    }
}

/// N-Triples form of a [`ShapeId`]: `<iri>` for a named shape, `_:label` for a blank one.
fn shape_id_nt(id: &ShapeId) -> String {
    match id {
        ShapeId::Named(n) => iri_nt(n.as_str()),
        ShapeId::Blank(b) => format!("_:{b}"),
    }
}

/// If `sparql_path` is a bare predicate path (`<iri>`), return the inner IRI; else `None`.
fn predicate_path_iri(sparql_path: &str) -> Option<&str> {
    let inner = sparql_path.strip_prefix('<')?.strip_suffix('>')?;
    // A bare predicate path is `<iri>`: the IRI itself cannot contain `<`, `>`, or whitespace, so
    // their presence means the outer string was a compound path (sequence/inverse/`*`/…), whose
    // `to_sparql` form embeds nested `<…>` or operators. `/`, `*`, `?` etc. are legal IRI chars and
    // must NOT be rejected here.
    if inner.contains(['<', '>', ' ']) {
        None
    } else {
        Some(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_conforming_report() {
        let nt = ValidationReport::default().to_ntriples();
        assert!(nt.contains("<http://www.w3.org/ns/shacl#ValidationReport>"));
        assert!(nt.contains(
            "<http://www.w3.org/ns/shacl#conforms> \
             \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        ));
    }

    #[test]
    fn serializes_result_fields() {
        let mut report = ValidationReport::default();
        report.push(ValidationResult {
            focus_node: Term::NamedNode(NamedNode::new_unchecked("http://example.com/a")),
            result_path: Some("<http://example.com/p>".to_string()),
            value: Some(Term::NamedNode(NamedNode::new_unchecked(
                "http://example.com/bad",
            ))),
            source_constraint_component: NamedNode::new_unchecked(format!(
                "{SH}DatatypeConstraintComponent"
            )),
            source_shape: ShapeId::Named(NamedNode::new_unchecked("http://example.com/Shape")),
            severity: Severity::Violation,
            messages: vec!["nope".to_string()],
        });
        let nt = report.to_ntriples();
        assert!(nt.contains("<http://www.w3.org/ns/shacl#conforms> \"false\""));
        assert!(nt.contains("<http://www.w3.org/ns/shacl#focusNode> <http://example.com/a>"));
        assert!(nt.contains("<http://www.w3.org/ns/shacl#resultPath> <http://example.com/p>"));
        assert!(nt.contains("<http://www.w3.org/ns/shacl#value> <http://example.com/bad>"));
        assert!(nt.contains(
            "<http://www.w3.org/ns/shacl#resultSeverity> <http://www.w3.org/ns/shacl#Violation>"
        ));
        assert!(nt.contains("<http://www.w3.org/ns/shacl#resultMessage> \"nope\""));
    }

    #[test]
    fn complex_path_is_skipped_not_misserialized() {
        let mut report = ValidationReport::default();
        report.push(ValidationResult {
            focus_node: Term::NamedNode(NamedNode::new_unchecked("http://example.com/a")),
            result_path: Some("(<http://example.com/p> / <http://example.com/q>)".to_string()),
            value: None,
            source_constraint_component: NamedNode::new_unchecked(format!(
                "{SH}MinCountConstraintComponent"
            )),
            source_shape: ShapeId::Blank("s0".to_string()),
            severity: Severity::Warning,
            messages: vec![],
        });
        let nt = report.to_ntriples();
        assert!(
            !nt.contains("resultPath"),
            "compound path must not be emitted: {nt}"
        );
        assert!(nt.contains("_:s0"), "blank shape id serialized");
    }
}

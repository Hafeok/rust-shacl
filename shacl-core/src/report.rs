//! Validation report model (§6.7, `REQ-RPT-2/3`). Serialization back to RDF over a backend is a
//! later step (build step 5, §11.5); this defines the in-memory result the engine produces.

use shacl_model::shape::Severity;
use shacl_model::shape::ShapeId;
use shacl_model::term::{NamedNode, Term};

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
    /// `sh:conforms` (§6.7.1.1): true iff no result has severity `Violation` (`REQ-RPT-2`).
    #[must_use]
    pub fn conforms(&self) -> bool {
        !self
            .results
            .iter()
            .any(|r| matches!(r.severity, Severity::Violation))
    }

    /// Append a result.
    pub fn push(&mut self, r: ValidationResult) {
        self.results.push(r);
    }
}

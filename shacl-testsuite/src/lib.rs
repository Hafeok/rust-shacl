//! W3C SHACL 1.2 test-suite runner (§10.1, `REQ-TS-*`). Each core test `.ttl` is self-contained:
//! the data graph and shapes graph are the document itself (`sht:dataGraph <>`/`sht:shapesGraph <>`),
//! and the expected `sh:ValidationReport` is embedded under `mf:result`.
//!
//! The runner parses the file, validates, and compares the produced report to the expected one. The
//! comparison is the relaxed graph-isomorphism the suite needs (`REQ-TS-2`): `sh:conforms` must
//! match, and the multiset of result `(focusNode, sourceConstraintComponent, value, resultPath)`
//! tuples must match up to blank-node renaming (blank nodes compare equal to any blank node; IRIs
//! and literals compare exactly).

use shacl_core::{validate, RdfGraph};
use shacl_model::term::{NamedNode, Term};
use shacl_oxigraph::ingest::{parse_data_with_base, parse_shapes_with_base};
use shacl_oxigraph::mem::MemGraph;

/// Base IRI used to resolve the relative IRIs (`<>`, `<test-001>`) in self-contained test files.
const TEST_BASE: &str = "http://example.org/test";

const SH: &str = "http://www.w3.org/ns/shacl#";
const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";

fn sh(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{local}"))
}

/// One expected validation result, reduced to the comparable fields.
#[derive(Debug, Clone)]
struct ExpectedResult {
    focus: Option<Term>,
    component: Option<Term>,
    value: Option<Term>,
    path: Option<String>,
}

/// The outcome of running one test.
#[derive(Debug)]
pub enum Verdict {
    /// The produced report matched the expected one.
    Pass,
    /// Mismatch (or a processing error), with a diagnostic.
    Fail(String),
}

impl Verdict {
    /// Did the test pass?
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }
}

/// Run one self-contained core test document, returning its [`Verdict`].
#[must_use]
pub fn run_test_file(ttl: &str) -> Verdict {
    let shapes = match parse_shapes_with_base(ttl, TEST_BASE) {
        Ok(s) => s,
        Err(e) => return Verdict::Fail(format!("shapes parse error: {e}")),
    };
    let data = match parse_data_with_base(ttl, TEST_BASE) {
        Ok(d) => d,
        Err(e) => return Verdict::Fail(format!("data parse error: {e}")),
    };

    let report = validate(&data, &shapes);
    let (expected_conforms, expected) = match parse_expected(&data) {
        Some(x) => x,
        None => return Verdict::Fail("no mf:result / sh:ValidationReport in test".to_string()),
    };

    if report.conforms() != expected_conforms {
        return Verdict::Fail(format!(
            "conforms mismatch: got {}, expected {expected_conforms}",
            report.conforms()
        ));
    }

    // Relaxed multiset match of result tuples.
    let mut actual: Vec<ExpectedResult> = report
        .results
        .iter()
        .map(|r| ExpectedResult {
            focus: Some(r.focus_node.clone()),
            component: Some(Term::NamedNode(r.source_constraint_component.clone())),
            value: r.value.clone(),
            path: r.result_path.clone(),
        })
        .collect();

    if actual.len() != expected.len() {
        return Verdict::Fail(format!(
            "result count: got {}, expected {}",
            actual.len(),
            expected.len()
        ));
    }
    for exp in &expected {
        match actual.iter().position(|a| result_match(exp, a)) {
            Some(i) => {
                actual.swap_remove(i);
            }
            None => return Verdict::Fail(format!("no actual result matched expected {exp:?}")),
        }
    }
    Verdict::Pass
}

/// Relaxed equality of two terms: blank nodes match any blank node; everything else is exact.
fn term_match(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::BlankNode(_), Term::BlankNode(_)) => true,
        _ => a == b,
    }
}

fn opt_term_match(a: &Option<Term>, b: &Option<Term>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => term_match(x, y),
        _ => false,
    }
}

fn result_match(exp: &ExpectedResult, act: &ExpectedResult) -> bool {
    // Component must match. focus/value match up to blank renaming. Path matched only when the
    // expectation carries one (predicate paths; complex blank-node paths are not compared).
    opt_term_match(&exp.component, &act.component)
        && exp
            .focus
            .as_ref()
            .is_none_or(|f| act.focus.as_ref().is_some_and(|a| term_match(f, a)))
        && opt_term_match(&exp.value, &act.value)
        && exp
            .path
            .as_ref()
            .is_none_or(|p| act.path.as_deref() == Some(p))
}

/// Parse the expected `sh:ValidationReport` (`sh:conforms` + result tuples) from the test graph.
fn parse_expected(g: &MemGraph) -> Option<(bool, Vec<ExpectedResult>)> {
    // The report is the object of mf:result.
    let report_node = g
        .triples(
            None,
            Some(&NamedNode::new_unchecked(format!("{MF}result"))),
            None,
        )
        .map(|t| t.object)
        .next()?;

    let conforms = g
        .objects(&report_node, &sh("conforms"))
        .iter()
        .any(|t| matches!(t, Term::Literal(l) if l.value() == "true"));

    let mut results = Vec::new();
    for r in g.objects(&report_node, &sh("result")) {
        results.push(ExpectedResult {
            focus: g.objects(&r, &sh("focusNode")).into_iter().next(),
            component: g
                .objects(&r, &sh("sourceConstraintComponent"))
                .into_iter()
                .next(),
            value: g.objects(&r, &sh("value")).into_iter().next(),
            path: match g.objects(&r, &sh("resultPath")).into_iter().next() {
                Some(Term::NamedNode(n)) => Some(format!("<{}>", n.as_str())),
                _ => None,
            },
        });
    }
    Some((conforms, results))
}

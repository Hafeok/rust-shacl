//! W3C SHACL 1.2 test-suite runner (§10.1, `REQ-TS-*`). A core test `.ttl` carries a `mf:action`
//! pointing at a data graph and a shapes graph: usually the document itself (`sht:dataGraph <>`/
//! `sht:shapesGraph <>`), but some tests reference sibling files (`<foo-data.ttl>`/`<foo-shapes.ttl>`).
//! The expected `sh:ValidationReport` is embedded under `mf:result`.
//!
//! The runner resolves those graph references ([`run_test_file_at`] reads siblings from disk;
//! [`run_test_file`] handles the self-contained case), validates, and compares the produced report
//! to the expected one. The comparison is the relaxed graph-isomorphism the suite needs
//! (`REQ-TS-2`): `sh:conforms` must match, and the multiset of result `(focusNode,
//! sourceConstraintComponent, value, resultPath)` tuples must match up to blank-node renaming (blank
//! nodes compare equal to any blank node; IRIs and literals compare exactly).

use shacl_core::{validate, RdfGraph};
use shacl_model::term::{NamedNode, Term};
use shacl_oxigraph::ingest::{parse_data_with_base, parse_shapes_with_base};
use shacl_oxigraph::mem::MemGraph;
use std::path::Path;

/// Base IRI used to resolve the relative IRIs (`<>`, `<test-001>`, `<foo-data.ttl>`) in test files.
/// `<>` resolves to exactly this string; a sibling reference `<foo.ttl>` resolves to
/// `http://example.org/foo.ttl` (the last base segment, `test`, is replaced).
const TEST_BASE: &str = "http://example.org/test";

const SH: &str = "http://www.w3.org/ns/shacl#";
const MF: &str = "http://www.w3.org/2001/sw/DataAccess/tests/test-manifest#";
const SHT: &str = "http://www.w3.org/ns/shacl-test#";

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

/// Run a self-contained core test document (data graph = shapes graph = the document), returning its
/// [`Verdict`]. Use [`run_test_file_at`] for tests that reference sibling data/shapes files.
#[must_use]
pub fn run_test_file(ttl: &str) -> Verdict {
    run_with(ttl, ttl, ttl)
}

/// Run a core test from its path, resolving `sht:dataGraph`/`sht:shapesGraph` references in
/// `mf:action`: `<>` means the document itself, while a sibling reference (`<foo-data.ttl>`) is read
/// from the test file's directory (`R1`). The expected report is always read from the test document.
#[must_use]
pub fn run_test_file_at(path: &Path) -> Verdict {
    let manifest = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return Verdict::Fail(format!("read error: {e}")),
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    // Parse the manifest to discover the action's graph references.
    let mg = match parse_data_with_base(&manifest, TEST_BASE) {
        Ok(g) => g,
        Err(e) => return Verdict::Fail(format!("manifest parse error: {e}")),
    };
    let action = mg
        .triples(
            None,
            Some(&NamedNode::new_unchecked(format!("{MF}action"))),
            None,
        )
        .map(|t| t.object)
        .next();

    let resolve = |pred: &str| -> String {
        let Some(action) = &action else {
            return manifest.clone();
        };
        match mg
            .objects(action, &NamedNode::new_unchecked(format!("{SHT}{pred}")))
            .into_iter()
            .next()
        {
            // `<>` resolves to TEST_BASE → the document itself.
            Some(Term::NamedNode(n)) if n.as_str() != TEST_BASE => {
                let file = n.as_str().rsplit('/').next().unwrap_or("");
                std::fs::read_to_string(dir.join(file)).unwrap_or_else(|_| manifest.clone())
            }
            _ => manifest.clone(),
        }
    };

    let data_ttl = resolve("dataGraph");
    let shapes_ttl = resolve("shapesGraph");
    run_with(&manifest, &data_ttl, &shapes_ttl)
}

/// Validate `data_ttl` against `shapes_ttl` and compare to the expected report embedded in
/// `manifest_ttl`. The three may be the same string (self-contained tests).
fn run_with(manifest_ttl: &str, data_ttl: &str, shapes_ttl: &str) -> Verdict {
    let shapes = match parse_shapes_with_base(shapes_ttl, TEST_BASE) {
        Ok(s) => s,
        Err(e) => return Verdict::Fail(format!("shapes parse error: {e}")),
    };
    let data = match parse_data_with_base(data_ttl, TEST_BASE) {
        Ok(d) => d,
        Err(e) => return Verdict::Fail(format!("data parse error: {e}")),
    };
    let manifest = match parse_data_with_base(manifest_ttl, TEST_BASE) {
        Ok(m) => m,
        Err(e) => return Verdict::Fail(format!("manifest parse error: {e}")),
    };

    let report = validate(&data, &shapes);
    let (expected_conforms, expected) = match parse_expected(&manifest) {
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

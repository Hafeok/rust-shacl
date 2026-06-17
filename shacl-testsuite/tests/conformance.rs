//! Offline conformance gate (§10.1, `REQ-TS-*`). Runs the bundled subset of W3C SHACL 1.2 core
//! tests (`tests/fixtures/`, vendored from `w3c/data-shapes`, W3C Test Suite licence) so CI exercises
//! the runner without a network checkout. The full suite is run via the `shacl-testsuite` binary
//! against a local clone.

use shacl_testsuite::run_test_file;
use std::fs;
use std::path::Path;

#[test]
fn bundled_w3c_core_tests_pass() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut total = 0;
    let mut failures = Vec::new();
    for entry in fs::read_dir(&dir).expect("fixtures dir exists") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "ttl") {
            continue;
        }
        total += 1;
        let ttl = fs::read_to_string(&path).unwrap();
        let verdict = run_test_file(&ttl);
        if !verdict.is_pass() {
            failures.push(format!("{}: {verdict:?}", path.display()));
        }
    }
    assert!(total >= 10, "expected the bundled fixtures, found {total}");
    assert!(
        failures.is_empty(),
        "bundled W3C tests failed:\n{}",
        failures.join("\n")
    );
}

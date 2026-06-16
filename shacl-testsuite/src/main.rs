//! W3C SHACL 1.2 test-suite runner (§10.1, `REQ-TS-*`). Loads per-directory `manifest.ttl`,
//! executes `sht:Validate` / focus / well-formedness entries, compares against expected reports
//! (graph-isomorphic on results, `REQ-TS-2`), and emits an implementation report (`REQ-TS-5`).
//!
//! Stub: argument parsing + manifest walking to be implemented in build step 10 (§11.5), after
//! cloning the suite from w3c/data-shapes gh-pages.

fn main() {
    eprintln!("shacl-testsuite: not yet implemented (build step 10, spec §10.1)");
    std::process::exit(2);
}

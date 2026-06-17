//! W3C SHACL 1.2 test-suite runner CLI (§10.1, `REQ-TS-*`).
//!
//! Usage: `shacl-testsuite <dir>` — recursively runs every self-contained core test `*.ttl` under
//! `<dir>` (e.g. a checkout of `w3c/data-shapes` `shacl12-test-suite/tests/core`), printing a
//! per-test pass/fail line and a summary. Exit code is non-zero if any test fails.

use shacl_testsuite::{run_test_file, Verdict};
use std::path::{Path, PathBuf};

fn main() {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: shacl-testsuite <dir-of-core-tests>");
        std::process::exit(2);
    };

    let mut files = Vec::new();
    collect_ttl(Path::new(&dir), &mut files);
    files.sort();

    let (mut pass, mut fail) = (0u32, 0u32);
    for f in &files {
        let Ok(ttl) = std::fs::read_to_string(f) else {
            continue;
        };
        // Only run files that embed a sht:Validate entry (skip pure manifest indexes / data files).
        if !ttl.contains("sht:Validate") && !ttl.contains("shacl-test#Validate") {
            continue;
        }
        match run_test_file(&ttl) {
            Verdict::Pass => {
                pass += 1;
                println!("PASS {}", f.display());
            }
            Verdict::Fail(why) => {
                fail += 1;
                println!("FAIL {} — {why}", f.display());
            }
        }
    }
    println!("\n{pass} passed, {fail} failed, {} total", pass + fail);
    if fail > 0 {
        std::process::exit(1);
    }
}

fn collect_ttl(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_ttl(&p, out);
        } else if p.extension().is_some_and(|x| x == "ttl") {
            out.push(p);
        }
    }
}

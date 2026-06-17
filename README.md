# shacl-rs

**A native Rust [SHACL 1.2](https://www.w3.org/TR/shacl12-core/) validator.**

[![CI](https://github.com/Hafeok/rust-shacl/actions/workflows/ci.yml/badge.svg)](https://github.com/Hafeok/rust-shacl/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![SHACL 1.2 core](https://img.shields.io/badge/W3C%20SHACL%201.2%20core-138%2F141-brightgreen.svg)](#conformance)

Validate an RDF graph against a SHACL shapes graph — entirely in Rust, no Python, no `pyshacl`.
`shacl-rs` implements the full SHACL 1.2 **Core** constraint set plus **SHACL-SPARQL** (`sh:sparql`),
parses Turtle 1.2 shapes and data, and runs over either an in-memory graph or an
[`oxigraph`](https://github.com/oxigraph/oxigraph) store.

```rust
use shacl_oxigraph::validate_turtle;

let shapes = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.com/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:PersonShape a sh:NodeShape ;
      sh:targetClass ex:Person ;
      sh:property [ sh:path ex:age ; sh:datatype xsd:integer ;
                    sh:message "age must be an integer" ] .
"#;
let data = r#"
    @prefix ex: <http://example.com/> .
    ex:alice a ex:Person ; ex:age "twenty" .
"#;

let report = validate_turtle(shapes, data)?;
assert!(!report.conforms());
for r in &report.results {
    println!("{:?}: {:?}", r.focus_node, r.messages); // → "age must be an integer"
}
# Ok::<(), String>(())
```

## Why

- **Native & embeddable.** A library, not a CLI shell-out — drop it into a Rust application and
  validate in-process.
- **Single source of truth.** Keep your constraints as `.ttl` SHACL shapes and validate them
  directly; no hand-mirrored checkers to keep in sync.
- **`sh:message` → `sh:resultMessage`.** Validation reports read as human-readable conformance
  reports, carrying the message each shape declares.
- **Core stays SPARQL-free.** `shacl-core` has **no** dependency on `oxigraph` or any SPARQL engine
  (enforced; `REQ-ARCH-1`); the SPARQL backend is an opt-in layer.

## Conformance

Validated against the [W3C SHACL 1.2 test suite](https://github.com/w3c/data-shapes): **138 / 141 of
the core suite pass** (~98%). The remaining three are out of scope for a Core validator (SHACL-SHACL
`shsh:` metashapes, a configurable `sh:conformanceDisallows` policy, and one internally-inconsistent
fixture). See [`CHANGELOG.md`](CHANGELOG.md) for the feature list and known limitations.

Supported, in brief: all §7 components (value-type, cardinality, range, string, property-pair,
logical, shape-based, list, `sh:closed`/`sh:hasValue`/`sh:in`/…), all seven property-path kinds, every
target form, recursion detection, RDF-1.2 reifier annotations, and SHACL-SPARQL §8.1.

## Install

```toml
[dependencies]
# released tag (recommended):
shacl-oxigraph = { git = "https://github.com/Hafeok/rust-shacl", tag = "v0.1.0" }
# or, for Core-only use without oxigraph:
# shacl-core = { git = "https://github.com/Hafeok/rust-shacl", tag = "v0.1.0" }
```

MSRV: **Rust 1.87** (required by `oxigraph` 0.5.x / `oxrdf` 0.3.x).

## Usage

**One call** — parse shapes + data from Turtle and validate (Core §7 *and* SPARQL §8.1):

```rust
let report = shacl_oxigraph::validate_turtle(shapes_ttl, data_ttl)?;
```

**Already have an `oxigraph::Store`?** Wrap it and reuse parsed shapes:

```rust
use shacl_oxigraph::{ingest::parse_shapes, store::OxiStore, validate_store};

let shapes = parse_shapes(shapes_ttl)?;          // parse once, cache
let store  = OxiStore::new(existing_store);
let report = validate_store(&store, &shapes);
```

**Core-only, against your own backend** — implement the `RdfGraph` trait and call the generic engine
(`shacl_core::validate`), with no oxigraph dependency at all.

Embedding in a host application (mapping the report to your own diagnostic type, what to keep/delete)
is covered in [`docs/integration.md`](docs/integration.md).

## Workspace layout

| Crate | Role | Depends on `oxigraph`? |
|---|---|---|
| `shacl-model` | RDF term model (re-exports `oxrdf`) + the shape / path / target AST | no |
| `shacl-core` | The Level-1 validation engine over the `RdfGraph` trait | **no** (enforced) |
| `shacl-sparql` | SHACL-SPARQL (§8): `sh:sparql` over the `SparqlGraph` trait | no |
| `shacl-oxigraph` | The only crate depending on `oxigraph`: `OxiStore`, Turtle ingestion, high-level API | yes |
| `shacl-testsuite` | W3C SHACL test-suite runner + offline conformance gate | yes |

The two seam traits live in `shacl-core/src/graph.rs`:

```rust
trait RdfGraph              { fn triples(s?, p?, o?) -> impl Iterator<Item = Triple>; }
trait SparqlGraph: RdfGraph { fn select(...); fn ask(...); }
```

Every constraint component implements `Validator`; the W3C §7 component packets map 1:1 onto these
impls. The full numbered specification is in
[`shacl-rs-functional-spec.md`](shacl-rs-functional-spec.md).

## Building & testing

```bash
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets
```

The library crates are free of `unwrap`/`expect`/`panic!`, so they pass downstream
`deny(clippy::unwrap_used)` policies.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

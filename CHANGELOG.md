# Changelog

All notable changes to `shacl-rs` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). All four crates (`shacl-model`,
`shacl-core`, `shacl-sparql`, `shacl-oxigraph`) are versioned together.

## [0.1.0] — 2026-06-17

First release: a native Rust [SHACL 1.2](https://www.w3.org/TR/shacl12-core/) validator.

### Added

**SHACL Core (§7)** — every constraint-component family, validated against the W3C SHACL 1.2 test
suite (**138/141 core tests passing**):

- Value type: `sh:class` (incl. list disjunction), `sh:datatype` (XSD lexical + derived-integer
  ranges; list disjunction), `sh:nodeKind` (incl. list).
- Cardinality: `sh:minCount` / `sh:maxCount`.
- Value range: `sh:minExclusive` / `sh:minInclusive` / `sh:maxExclusive` / `sh:maxInclusive`.
- String: `sh:minLength` / `sh:maxLength`, `sh:pattern` (+ `sh:flags`, via `fancy-regex`),
  `sh:singleLine`, `sh:languageIn`, `sh:uniqueLang` (with RDF-1.2 base direction).
- Property pair: `sh:equals` / `sh:disjoint` / `sh:subsetOf` / `sh:lessThan` /
  `sh:lessThanOrEquals` (predicate or sequence-path valued).
- Logical: `sh:not` / `sh:and` / `sh:or` / `sh:xone`.
- Shape-based: `sh:node`, `sh:property`, `sh:qualifiedValueShape` (+ `…Disjoint`), `sh:someValue`,
  `sh:nodeByExpression` (IRI expression), `sh:reifierShape` (+ `sh:reificationRequired`).
- List (RDF-1.2): `sh:memberShape`, `sh:minListLength` / `sh:maxListLength`, `sh:uniqueMembers`.
- Other: `sh:closed` (+ `sh:ByTypes`), `sh:hasValue`, `sh:in`, `sh:rootClass`, `sh:uniqueValuesFor`
  (single or property-tuple).

**Targets** — `sh:targetNode` / `sh:targetClass` (+ implicit class & `sh:ShapeClass`) /
`sh:targetSubjectsOf` / `sh:targetObjectsOf` / `sh:targetWhere` / explicit `sh:shape` data links.

**Property paths (§4)** — all seven kinds (predicate, inverse, sequence, alternative, zero-or-more,
one-or-more, zero-or-one) over a least-fixpoint closure; recursion guard (Tarjan-style cycle
detection, reject-on-recursion per ADR-002).

**SHACL-SPARQL (§8.1)** — `sh:sparql` / `sh:SPARQLConstraint` with `$this` pre-binding via VALUES
injection; failure-vs-violation distinction.

**Ingestion** — Turtle 1.2 parsing (`oxttl`, rdf-12) into the shape model, including RDF-1.2 reifier
annotations (`{| sh:deactivated true |}`) and `sh:message`.

**Reporting** — `ValidationReport` with `conforms()` (Trace/Debug severities are diagnostic-only),
N-Triples serialization, and `sh:message` → `sh:resultMessage` copying.

**Backends** — in-memory `MemGraph` and an `oxigraph::Store` adapter (`OxiStore`) implementing
`RdfGraph` + `SparqlGraph`.

**Host-application surface** — `shacl_oxigraph::validate_turtle(shapes_ttl, data_ttl)` and
`validate_store(&OxiStore, &shapes)` run Core (§7) and SPARQL (§8.1) in one pass and merge into one
report. `OxiStore::new(store)` wraps an existing `oxigraph::Store`. See [`docs/integration.md`].

**Conformance tooling** — `shacl-testsuite` runner for the W3C SHACL test manifests (relaxed
graph-isomorphic comparison) plus an offline fixture gate.

### Architecture

- `shacl-core` has **no** dependency on `oxigraph` or any SPARQL engine (`REQ-ARCH-1`); everything is
  expressed against the `RdfGraph` trait.
- Library code is free of `unwrap`/`expect`/`panic!`/indexing panics.
- MSRV: Rust 1.87 (required by `oxigraph` 0.5.x / `oxrdf` 0.3.x).

### Known limitations

- `sh:prefixes` collection (REQ-SPQ-13) is not implemented — `sh:select` queries must use full IRIs
  or carry their own `PREFIX` lines.
- SPARQL-based constraint *components* (§8.2) and pre-binding restriction checks (REQ-SPQ-15) are not
  implemented.
- Complex-path `sh:resultPath` is not serialized to RDF (the in-memory SPARQL-string form is always
  available).
- SHACL-SHACL (`shsh:`) shapes-graph well-formedness and the configurable `sh:conformanceDisallows`
  policy are not supported (the 3 W3C core tests these cover are the only failures).

[0.1.0]: https://github.com/Hafeok/rust-shacl/releases/tag/v0.1.0
[`docs/integration.md`]: docs/integration.md

# Embedding shacl-rs in a host application

This guide shows how to use `shacl-rs` as a library — written against `product-cli`'s case, which
already maintains SHACL shapes (`schema/shapes/*.ttl`) and currently hand-mirrors them in Rust /
shells out to `pyshacl`. `shacl-rs` replaces both with one native engine over the SHACL shapes as the
single source of truth.

## Dependency

```toml
[dependencies]
shacl-oxigraph = { path = "../rust-shacl/shacl-oxigraph" }   # or a git rev
# pulls in shacl-core, shacl-model, shacl-sparql transitively
```

`shacl-core` has **no** oxigraph dependency; `shacl-oxigraph` is the only crate that does (and the
host already depends on oxigraph, ADR-008).

## The one call you need

```rust
use shacl_oxigraph::validate_turtle;

// shapes_ttl  = your schema/shapes/shapes.shacl.ttl (or how.shacl.ttl) as a string
// data_ttl    = the Turtle projection of the graph under test (you already build this in pf/turtle.rs)
let report = shacl_oxigraph::validate_turtle(shapes_ttl, data_ttl)?;   // Result<ValidationReport, String>

if !report.conforms() {
    for r in &report.results {
        // r.messages carries the shape's sh:message ("§3.1 An entity must …")
    }
}
```

This runs **Core constraints (§7)** *and* **`sh:sparql` constraints (§8.1)** in one pass over a single
`OxiStore`, and merges them into one report — including the load-bearing cross-reference rules that
`rules_what.rs` / `rules_how.rs` currently hand-mirror as SPARQL.

### If you already have an `oxigraph::Store`

Avoid re-parsing — wrap your store (clone is cheap, the storage is shared) and pass parsed shapes:

```rust
use shacl_oxigraph::{store::OxiStore, validate_store};
use shacl_oxigraph::ingest::parse_shapes;

let shapes = parse_shapes(shapes_ttl)?;          // parse the .ttl once, cache it
let store  = OxiStore::new(my_oxigraph_store.clone());
let report = validate_store(&store, &shapes);
```

## Mapping the report to `pf::validate::Violation`

`ValidationReport.results: Vec<ValidationResult>` maps directly onto product-cli's `Violation`:

```rust
use shacl_core::ValidationResult;
use shacl_model::shape::{Severity, ShapeId};

fn to_violation(r: &ValidationResult) -> pf::validate::Violation {
    pf::validate::Violation {
        focus:    term_local_name(&r.focus_node),                 // SHACL focus node
        path:     r.result_path.as_deref().map(iri_local_name)    // "<iri>" → local name
                   .unwrap_or_default(),
        message:  r.messages.first().cloned().unwrap_or_default(), // sh:message
        severity: match r.severity {                              // all framework rules are blocking
            Severity::Violation => "violation",
            Severity::Warning   => "warning",
            Severity::Info      => "info",
            _                   => "info",
        }.to_string(),
    }
}
```

`r.source_shape: ShapeId` (`Named(iri)` / `Blank(id)`) and `r.source_constraint_component: NamedNode`
are also available if you want richer reporting.

## What you can delete after adopting

- `validate.py` + the `pyshacl` / `rdflib` Python dependency.
- The hand-mirrored Rust conformance checkers that exist only to avoid the Python path:
  `pf/validate.rs` presence/cardinality checks, and the `rules_what.rs` / `rules_how.rs` /
  `sparql_rules.rs` SPARQL-rule mirrors — they are now expressed once, in the `.ttl` shapes.

The SHACL shapes (`shapes.shacl.ttl`, `how.shacl.ttl`) become the single source of truth, validated
natively in Rust, with `git diff` showing exactly what a shape change affects.

## Caveats (none block product-cli's current shapes)

- `sh:select` queries must use full IRIs or carry their own `PREFIX` lines — `sh:prefixes` collection
  (REQ-SPQ-13) isn't implemented yet. product-cli's `how.shacl.ttl` already uses full IRIs.
- SPARQL-based *components* (§8.2, custom `sh:ConstraintComponent` with `sh:validator`) and
  pre-binding restriction checks (REQ-SPQ-15) are not implemented.
- Complex-path (`sequence`/`alternative`/`*`) `sh:resultPath` is not yet serialized to RDF (the
  in-memory `result_path` SPARQL string is always available).

# shacl-rs

A SHACL 1.2 validation engine in Rust, decoupled from any single triplestore.

Targets **SHACL 1.2 Core** + **SHACL 1.2 SPARQL Extensions** (W3C Working Drafts). The full
behavioral specification — numbered requirements traced to the W3C spec and the W3C test suite —
is in [`shacl-rs-functional-spec.md`](./shacl-rs-functional-spec.md). That document is the source
of truth; this README is just orientation.

## Why another SHACL library

The engine is generic over a graph **backend trait**, not bound to a specific store. `oxigraph`
is *a* backend, behind its own crate — the core validation logic compiles without it (enforced in
CI, `REQ-ARCH-1`). Property paths and recursion are implemented as least-fixpoint closures with a
property-tested oracle (the "provable core", spec §4.1, §9).

## Workspace layout (spec §11.1)

| Crate | Role | Key spec refs | Depends on `oxigraph`? |
|-------|------|---------------|------------------------|
| `shacl-model` | Shape + path AST, RDF 1.2 term re-export | §3, §4, `REQ-ING/TERM` | no |
| `shacl-core` | Validation engine, generic over `RdfGraph` | §4–§7, `REQ-PATH/TGT/RPT/CMP` | **no** (enforced) |
| `shacl-sparql` | SHACL-SPARQL, generic over `SparqlGraph` | §8, `REQ-SPQ` | no |
| `shacl-oxigraph` | Backend adapters + in-memory test graph | §11, ADR-009 | yes |
| `shacl-testsuite` | W3C 1.2 manifest runner + diff harness | §10.1, `REQ-TS` | yes |

The two seam traits (`shacl-core/src/graph.rs`):

```rust
trait RdfGraph   { fn triples(s?, p?, o?) -> ...; fn reach(start, path) -> NodeSet; }
trait SparqlGraph: RdfGraph { fn select(...); fn ask(...); }
```

Every constraint component implements `Validator` (`shacl-core/src/validator.rs`); the spec's §7
component packets map 1:1 to these impls.

## Build order (spec §11.5)

Each step lands with green tests before the next:

1. `shacl-model` + Turtle 1.2 shapes-graph parsing (`oxttl`).
2. **`closure` helper + property tests** — the provable fixpoint core. *(implemented)*
3. `MemGraph` in-memory backend. *(implemented)*
4. Path evaluation (§4) over `RdfGraph`. *(implemented; inverse-of-closure partial — see TODO)*
5. Report builders (§6.7).
6. **`CMP-NODEKIND`** — wires validator dispatch + report end-to-end. *(implemented)*
7. `CMP-CLASS`, `CMP-DATATYPE` (uses `is_shacl_instance` + `oxsdatatypes`). *(class sketched; datatype lexical TODO)*
8. Remaining §7 groups: cardinality → range → string → pair → logical → shape → list → other.
9. `shacl-sparql` (§8): prefixes → constraints → components → pre-binding seam.
10. Conformance matrix + CI gate on the W3C 1.2 suite.

## Status

Scaffold + provable core. What's real vs. stubbed:

- **Real**: workspace, traits, `closure` (with proptest invariants), path evaluator,
  `MemGraph`, `NodeKind` validator, `is_shacl_instance`, report/conforms logic.
- **Stubbed**: shapes-graph ingestion (needs parser wiring), most §7 components, all of §8,
  the test-suite runner.
- **Known partial**: inverse of a `*`/`+` path (`REQ-PATH-4`, flagged with `debug_assert!`);
  `sh:datatype` lexical-space check (`REQ-DATATYPE-2`, TODO via `oxsdatatypes`).

> Not yet compiled in this environment (no toolchain present at scaffold time). Signatures and
> module structure are consistent with the spec; first `cargo build`/`cargo test` is step 1 of
> picking this up.

## License

MIT OR Apache-2.0.

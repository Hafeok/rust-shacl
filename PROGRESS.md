# shacl-rs — Implementation Progress

Tracking implementation of `shacl-rs-functional-spec.md`. Build order per spec §11.5; each step
should have green tests before the next. Status legend: ✅ done · 🟡 in progress · ⬜ not started.

## Environment / setup — ✅ COMPLETE (workspace builds, `cargo test --workspace` green, clippy clean)
- ✅ Unpacked scaffold tarball, cleaned Windows `:Zone.Identifier` artifacts (gitignored).
- ✅ Fixed dependency pins: `oxrdf 0.3` / `oxttl 0.2` (independently versioned from `oxigraph 0.5`),
  all with `rdf-12`.
- ✅ Bumped toolchain + MSRV to 1.87 (required by oxigraph 0.5.x / oxrdf 0.3.x).
- ✅ Foundational fix: `Term` is not `Ord` in oxrdf 0.3.3 → `NodeSet` is now `IndexSet<Term>` and
  `closure` is generic over `Hash + Eq`. Cleared all 12 shacl-core compile errors.
- ✅ Restored the dropped `fn eval(...)` signature in `shacl-core/src/path.rs` (scaffold corruption).
- ✅ Disabled oxigraph's default `rocksdb` feature (`default-features = false`) so the build needs no
  libclang / C++ RocksDB toolchain; the in-memory Store + SPARQL eval are all SHACL needs.
- ✅ REQ-ARCH-1 verified: `shacl-core` has no `oxigraph` in its dependency tree.

## Build order (§11.5)
1. ⬜ **shacl-model** + `oxrdf` re-export; Turtle 1.2 shapes-graph parsing via `oxttl` (REQ-ING-*, ADR-009).
   - Model AST present. Ingestion (Turtle → Shape set) not yet written.
2. ✅ **closure helper** + property tests (the provable core, REQ-PATH-7/9). Migrated to `IndexSet`; 7
   tests green (oracle/idempotence/termination + plus/star edge cases).
3. ⬜ **shacl-oxigraph** in-memory `RdfGraph` (`MemGraph` present) + `oxigraph::Store` adapter.
4. ⬜ **value nodes / paths** (§4, §5) over RdfGraph; pass `path/*` suite. `path::reach` present.
5. ⬜ **report builders** (§6.7) — in-memory model present; RDF serialization pending.
6. ⬜ **CMP-NODEKIND** (done as validator) — wire validator dispatch + engine end-to-end.
7. ⬜ **CMP-CLASS, CMP-DATATYPE** (sketch/stub) — finish `is_shacl_instance` + oxsdatatypes lexical.
8. ⬜ Remaining §7 components: cardinality → range → string → pair → logical → shape → list → other.
9. ⬜ **shacl-sparql** (§8): prefixes → constraints → components → prebinding seam (ADR-008). All stubs.
10. ⬜ **conformance matrix** + W3C 1.2 testsuite runner (§10). `shacl-testsuite` is a stub.

## Cross-cutting pieces still missing
- ⬜ The validation **engine**: shape → targets (§6, REQ-TGT-*) → value nodes (§5) → dispatch
  constraints → assemble `ValidationReport`. (Nothing ties the validator trait to a run yet.)
- ⬜ Target resolution (REQ-TGT-1..7), incl. `sh:targetWhere` naive iteration (ADR-007).
- ⬜ Shapes-graph ingestion (parse Turtle → `Shape`s, REQ-ING-1..10), ill-formedness detection.
- ⬜ Recursion / cycle detection (Tarjan SCC, §9.1, ADR-002) before logical/shape components.

## Notes / decisions taken during implementation
- `NodeSet = IndexSet<Term>` (not `BTreeSet`) because oxrdf `Term: !Ord`. Determinism comes from
  insertion order; report comparison is graph-isomorphic anyway (REQ-TS-2).
</content>
</invoke>

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
3. ✅ **shacl-oxigraph** in-memory `RdfGraph` (`MemGraph`). ⬜ `oxigraph::Store` adapter still to add.
4. ✅ **value nodes / paths** (§4, §5) over RdfGraph. `value_nodes()` added; `reach` exercised by 10
   integration tests in `shacl-oxigraph/tests/path_eval.rs` (all seven path kinds + cyclic-data).
5. 🟡 **report builders** (§6.7) — in-memory model + `conforms()` work; RDF serialization pending.
6. ✅ **Engine end-to-end** (`engine.rs`): targets → value nodes → `dispatch` → report. CMP-NODEKIND
   wired; 8 engine tests in `shacl-oxigraph/tests/engine.rs`.
7. ✅ **CMP-CLASS, CMP-DATATYPE** — `is_shacl_instance` (subclass walk) drives class; datatype now
   does full lexical validation via `oxsdatatypes` (REQ-DATATYPE-2). Both green.
8. 🟡 Remaining §7 components: cardinality → range → string → pair → logical → shape → list → other.
   In progress — Phase 8a (cardinality) underway. See **Build plan** below for the phase breakdown.
9. ⬜ **shacl-sparql** (§8): prefixes → constraints → components → prebinding seam (ADR-008). All stubs.
10. ⬜ **conformance matrix** + W3C 1.2 testsuite runner (§10). `shacl-testsuite` is a stub.

## Build plan (phased) — remaining work, dependency-ordered

Each phase is gated on green tests before the next. Components = one `Validator` impl + one
`dispatch` arm + table-driven `MemGraph` tests. Pure-term components first; graph-walking next;
recursion-bearing ones gated behind the SCC guard (9b).

### Phase 8 — remaining §7 components
- **8a. Cardinality (§7.2)** — ✅ `CMP-MINCOUNT` + `CMP-MAXCOUNT`. Results carry no `sh:value`
  (violation is the count). Hoisted shared `comp`/`result_for` helpers into `constraints/mod.rs`;
  added `param_int`. 6 tests in `shacl-oxigraph/tests/cardinality.rs` (boundary + distinct-count).
- **8b. String, set membership, range (no recursion)** — ✅ `CMP-LENGTH-*`, `CMP-PATTERN`
  (fancy-regex, ADR-005), `CMP-SINGLELINE`, `CMP-LANGUAGEIN`, `CMP-UNIQUELANG` (§7.4); `CMP-HASVALUE`
  / `CMP-IN` (§7.9); `CMP-RANGE-*` (§7.3) with a shared numeric/dateTime comparator. List-valued
  params (`sh:in`/`sh:languageIn`) adopt the repeated-`(pred, element)` representation that
  ingestion will flatten into. Added `param_term`/`param_terms`/`param_bool`. 11 tests in
  `shacl-oxigraph/tests/string_range_membership.rs`.
- **8c. Property-pair (§7.6)** — `sh:equals/disjoint/subsetOf/lessThan/lessThanOrEquals`; second
  path eval against the focus (reuse `reach`).
- **8d. List (§7.5, new in 1.2)** — `rdf:List` walker; `sh:minListLength/maxListLength/uniqueMembers`
  no-recursion; `sh:memberShape` recurses → gate behind 9b.
- **8e. `sh:closed`/`sh:rootClass`/`sh:uniqueValuesFor` (§7.9)** — node-level property-set checks.

### Phase 9 — cross-cutting infra (interleave, not strictly after 8)
- **9a. Report RDF serialization** (finishes step 5) — `ValidationReport → Turtle` (REQ-RPT-2/3).
  Do early: the testsuite runner diffs serialized output.
- **9b. Recursion / cycle guard** (ADR-002, §9.1) — Tarjan SCC over the shape-ref graph. **Hard gate**
  before 8d's `sh:memberShape` and all of 9c.
- **9c. Shape-logic + shape-ref (§7.7–7.8)** — `sh:not/and/or/xone`, `sh:node/property/someValue/
  qualifiedValueShape`. Needs a shape registry + conformance-checking entry point. After 9b.

### Phase 10 — ingestion (unblocks real fixtures)
Turtle → `Shape` (`oxttl` rdf-12, REQ-ING-*); ill-formedness detection (REQ-ING-3/4/5);
`sh:message` → `sh:resultMessage` (REQ-ING-9); then `sh:targetWhere` (REQ-TGT-5) + explicit
`sh:shape` data targets (REQ-TGT-6).

### Phase 11 — SHACL-SPARQL (§8, L2)
`oxigraph::Store` `SparqlGraph` adapter → prefixes → SPARQL constraints (`sh:sparql`) → SPARQL
components → pre-binding seam (ADR-008).

### Phase 12 — conformance testsuite (§10)
W3C 1.2 manifests → `shacl-testsuite` runner (graph-isomorphic diff, REQ-TS-2) → matrix + CI gate.

**Critical path:** 9a + Phase 10 are the unlocks (real `.ttl` fixtures vs hand-built `MemGraph`).
9b blocks `sh:memberShape` and 9c. 8a–8c can proceed now with no new infra.

## Cross-cutting pieces
- ✅ The validation **engine** (`engine::validate`): shape → targets → value nodes → dispatch → report.
- 🟡 Target resolution: `sh:targetNode/targetClass/implicitClass/targetSubjectsOf/targetObjectsOf`
  done (REQ-TGT-1/2/3/4). ⬜ `sh:targetWhere` (REQ-TGT-5, naive iter ADR-007) and explicit `sh:shape`
  data-graph targets (REQ-TGT-6) need a shape registry — deferred.
- ⬜ Shapes-graph ingestion (parse Turtle → `Shape`s, REQ-ING-1..10), ill-formedness detection.
- ⬜ Recursion / cycle detection (Tarjan SCC, §9.1, ADR-002) before logical/shape components.
- ⬜ `sh:message` → `sh:resultMessage` copying (REQ-ING-9); results currently carry empty messages.

## Known gaps logged during implementation
- Derived integer datatypes (xsd:byte/int/short/unsigned*) are lexically validated as xsd:integer;
  numeric range bounds not yet enforced (`lexical_valid` in `constraints/value_type.rs`).

## Notes / decisions taken during implementation
- `NodeSet = IndexSet<Term>` (not `BTreeSet`) because oxrdf `Term: !Ord`. Determinism comes from
  insertion order; report comparison is graph-isomorphic anyway (REQ-TS-2).
</content>
</invoke>

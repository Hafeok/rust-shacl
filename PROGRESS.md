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
- **8c. Property-pair (§7.6)** — ✅ `sh:equals` (symmetric diff), `sh:disjoint`, `sh:subsetOf`,
  `sh:lessThan`/`sh:lessThanOrEquals` (reuse `range::compare`). Paired values = objects of
  `(focus, predicate, *)`. 5 tests in `shacl-oxigraph/tests/pair.rs`.
- **8d. List (§7.5, new in 1.2)** — ✅ `rdf:List` walker (cycle-safe); `sh:minListLength`,
  `sh:maxListLength`, `sh:uniqueMembers`. `sh:memberShape` recurses → deferred to 9c (with the guard).
  3 tests in `shacl-oxigraph/tests/list.rs`.
- **8e. `sh:closed`/`sh:rootClass`/`sh:uniqueValuesFor` (§7.9)** — `sh:closed` needs the sibling
  property-shape predicate set → folded into **9c** (shape registry). `sh:rootClass` /
  `sh:uniqueValuesFor` are under-specified in the 1.2 draft → deferred, tracked under known gaps.

### Phase 9 — cross-cutting infra (interleave, not strictly after 8)
- **9a. Report RDF serialization** (finishes step 5) — ✅ `ValidationReport::to_ntriples()`
  (REQ-RPT-2/3): report + result blank nodes, `sh:conforms`, all §6.7.2 result fields.
  `sh:resultPath` emitted for predicate paths; compound paths skipped (documented gap, need RDF
  blank-node path structure). 3 unit tests in `report.rs`.
- **9b. Recursion / cycle guard** (ADR-002, §9.1) — ✅ `recursion::shape_ref_cycle` (white/grey/black
  DFS over the shape-ref graph; self-ref = cycle; dangling refs ignored). 4 unit tests. Runtime also
  has a `MAX_DEPTH` backstop in the engine.
- **9c. Shape-logic + shape-ref (§7.7–7.8)** — ✅ Added a shape `Registry` (`ShapeId → &Shape`) on
  `Ctx`, recursive `conforms`/`validate_focus_collect` in the engine, and the components:
  `sh:not/and/or/xone` (§7.7), `sh:node` (summarises) / `sh:property` (bubbles) /
  `sh:qualifiedValueShape` min+max (§7.8), `sh:memberShape` (§7.5.1), and `sh:closed` +
  `sh:ignoredProperties` (§7.9.1, 8e). 8 tests in `shacl-oxigraph/tests/shape_logic.rs`.
  Deferred: `sh:someValue`, `sh:reifierShape`/`sh:reificationRequired`,
  `sh:qualifiedValueShapesDisjoint`, `sh:rootClass`, `sh:uniqueValuesFor` (under-specified / RDF-1.2
  reification — see known gaps).

### Phase 10 — ingestion (unblocks real fixtures) — ✅ core done
✅ `shacl-oxigraph::ingest`: `parse_shapes`/`parse_data` (Turtle 1.2 via `oxttl` → `MemGraph` →
shapes). Shape detection (sh:path / target / param / NodeShape-PropertyShape type), constraint
grouping by component (with secondary params: flags, ignoredProperties, qualifiedValueShape),
list-param flattening, all-seven-kinds `sh:path` parsing, targets, severity, deactivation. 6
end-to-end tests in `shacl-oxigraph/tests/ingest.rs`.
⬜ Remaining: `sh:message` → `sh:resultMessage` (REQ-ING-9, needs a messages field on Constraint);
explicit ill-formedness *diagnostics* (REQ-ING-3/4/5 — currently ill-formed params are silently
skipped, not flagged); `sh:targetWhere` (REQ-TGT-5) + explicit `sh:shape` data targets (REQ-TGT-6).

### Phase 11 — SHACL-SPARQL (§8, L2) — ✅ core done
✅ `shacl-oxigraph::store::OxiStore`: `oxigraph::Store` implementing `RdfGraph` (pattern access via
`quads_for_pattern`) + `SparqlGraph` (SELECT/ASK via `SparqlEvaluator`). Pre-binding (§8.4, ADR-008)
as a `VALUES`-injection of the `$`-sigil variables (preserves `$this` projection).
✅ `shacl-sparql::constraint::validate_select` (§8.1, REQ-SPQ-1..6): `this` pre-bound, one result per
non-`failure` solution, REQ-SPQ-5 property mapping, failure-vs-violation distinction. `prefixes`
helper for `PREFIX` prepending. 5 tests in `shacl-oxigraph/tests/sparql.rs`.
⬜ Deferred: SPARQL-based **components** (§8.2, `sh:validator`/`sh:nodeValidator`), full prefix
collection (REQ-SPQ-13 property path), pre-binding restriction enforcement (REQ-SPQ-15), and wiring
SPARQL constraints into the engine dispatch (the L1 engine is `RdfGraph`-only by `REQ-ARCH-1`).

### Phase 12 — conformance testsuite (§10) — ✅ runner + gate done
✅ `shacl-testsuite`: a runner (`run_test_file`) for the self-contained W3C 1.2 core tests
(data = shapes = the doc; expected report under `mf:result`), with relaxed graph-isomorphic result
comparison (`sh:conforms` + result tuples up to blank-node renaming, REQ-TS-2); a CLI
(`shacl-testsuite <dir>`) for an external suite checkout; and an offline CI gate over 12 vendored
passing fixtures (`tests/fixtures/`).
**Result against the real W3C SHACL 1.2 core suite: 101 / 141 passing (~72%).** Two bugs found and
fixed by the suite: (1) `sh:conforms` is "no results at all", not "no Violation-severity results"
(severity is result metadata); (2) empty list params (`sh:in ()`, `sh:xone ()`) are declared
constraints with defined semantics, not no-ops.

**Remaining 40 failures** are genuine SHACL-1.2 enhancements / deferred features, not core bugs:
path-valued pair constraints (`sh:equals`/`disjoint`/`lessThan` taking a path, not just a predicate),
datatype/`nodeKind` list values, RDF-1.2 reifier annotations (`{| sh:deactivated true |}`),
`sh:reifierShape`/`sh:someValue`/`sh:rootClass`/`sh:uniqueValuesFor`/`sh:nodeByExpression`,
`sh:targetWhere`/explicit-shape targets, complex-path result serialization, and `shsh:`
well-formedness checks.

**Critical path (historical):** 9a + Phase 10 were the unlocks (real `.ttl` fixtures vs hand-built
`MemGraph`). 9b gated `sh:memberShape` and 9c.

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
  numeric range bounds are enforced separately by `sh:minInclusive`/etc. (range comparator, 8b).
- SHACL-1.2 enhancements not yet implemented (see Phase 12 failures): path-valued property-pair
  constraints; list-valued `sh:datatype`/`sh:nodeKind`; RDF-1.2 reifier annotations on constraints;
  `sh:reifierShape`/`sh:reificationRequired`/`sh:someValue`/`sh:rootClass`/`sh:uniqueValuesFor`/
  `sh:nodeByExpression`; `sh:targetWhere`/explicit-`sh:shape` targets; `sh:message` →
  `sh:resultMessage` copying; complex-path `sh:resultPath` RDF serialization; `shsh:` shapes-graph
  well-formedness; SPARQL-based constraint *components* (§8.2) and pre-binding restriction checks.

## Notes / decisions taken during implementation
- `NodeSet = IndexSet<Term>` (not `BTreeSet`) because oxrdf `Term: !Ord`. Determinism comes from
  insertion order; report comparison is graph-isomorphic anyway (REQ-TS-2).
</content>
</invoke>

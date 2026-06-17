# shacl-rs — Implementation Progress

Tracking implementation of `shacl-rs-functional-spec.md`. Build order per spec §11.5; each step
should have green tests before the next. Status legend: ✅ done · 🟡 in progress · ⬜ not started.

> **Conformance remediation plan** (closing the last 40 W3C 1.2 core failures, 101→141) lives at the
> bottom of this file under **"Conformance remediation plan"**.

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

---

## Conformance remediation plan (last 40 W3C 1.2 core failures: 101 → 141)

Each of the 40 failures was triaged against the live `w3c/data-shapes` `shacl12-test-suite/tests/core`
suite. They group into **runner gaps** (the harness loads the wrong graph), **cheap engine fixes**,
**medium 1.2 enhancements**, and **larger 1.2 features**. Work the tiers top-down — each tier is
independently shippable with its own test-count win.

### Tier 1 — runner + cheap fixes — ✅ DONE (101 → 109)
All four landed: **R1** multi-file loading (+4), **R2** Trace/Debug severities (+2), **R3**
`sh:singleLine` whitespace (+1), **R4** derived-integer range in `sh:datatype` (+1). Fixtures added
for severity-004/005 + singleLine-001; `derived_integer_ranges_enforced` unit test in `value_type`.

#### Original notes (kept for reference)

- **R1. Multi-file test loading** — *runner*, ~5 clean wins (+ partials). Several tests use
  `sht:dataGraph <foo-data.ttl>` / `sht:shapesGraph <foo-shapes.ttl>` instead of `<>`; the runner
  currently loads the manifest doc as both. Fix `shacl-testsuite`: read `mf:action`'s
  `sht:dataGraph`/`sht:shapesGraph`, resolve relative refs against the test file's dir, and load the
  referenced files (falling back to the doc itself for `<>`). Unblocks `xone-duplicate`,
  `path/path-unused-001`, `path/path-complex-002`, `node/qualified-001`, `validation-reports/shared`
  (and is a prerequisite for `datatype-ill-formed`, `targets/shape-001`).
- **R2. Severity `sh:Trace`/`sh:Debug` don't break conformance** — *1-line*, 2 tests
  (`misc/severity-004/005`). `sh:conforms` is "no result with severity ≥ `sh:Info`"; `Trace`/`Debug`
  are diagnostic-only. Refine `ValidationReport::conforms()` from `results.is_empty()` to
  `!results.iter().any(|r| sev ∈ {Info, Warning, Violation})`. (Keeps `severity-001` passing.)
- **R3. `sh:singleLine` whitespace set** — *small*, 1 test (`property/singleLine-001`). Broaden the
  rejected chars beyond `\n`/`\r` to all Unicode line breaks: `\n \r \f      `.
- **R4. Derived-integer range in `sh:datatype`** — *small*, contributes to `datatype-ill-formed`
  (with R1). `"300"^^xsd:byte` is ill-formed (out of range). Replace the "validate derived ints as
  `xsd:integer`" shortcut in `value_type::lexical_valid` with per-type `oxsdatatypes` parsers
  (`Byte`, `Short`, `Int`, `UnsignedByte`, …) so range bounds are enforced.

### Tier 2 — medium 1.2 enhancements — ✅ DONE (109 → 121)
**M1** list-valued datatype/nodeKind disjunction (+3; `sh:class` list left as a gap), **M2**
path-valued pair constraints + per-pair `lessThan` results (+6), **M3** `sh:uniqueLang` direction +
`param_bool` "true"-only (+2), **M4** implicit `sh:ShapeClass` targets (+1).

#### Original notes (kept for reference)

- **M1. List-valued value-type params** — 4 tests (`node/datatype-003`, `property/datatype-004`,
  `node/nodeKind-002`, `property/class-002`). In 1.2 `sh:class`/`sh:datatype`/`sh:nodeKind` may take
  an `rdf:List`: `sh:class` = conjunction (instance of **all**), `sh:datatype`/`sh:nodeKind` =
  disjunction (**any** of). Mark these three primary params list-valued in `ingest::LIST_PARAMS`,
  and add disjunctive `Datatype`/`NodeKind` validators (one validator over a set) while keeping
  `sh:class` repeats as independent conjuncts.
- **M2. Path-valued property-pair constraints** — 6 tests (`property/equals-002`, `disjoint-002`,
  `subsetOf-002`, `lessThan-002/003`, `lessThanOrEquals-002`). In 1.2 `sh:equals`/`disjoint`/
  `subsetOf`/`lessThan`/`lessThanOrEquals` take a **path** (often a sequence list) rather than only a
  predicate IRI. Change the pair validators to hold a `Path` and compute the "other" value set via
  `reach(graph, focus, &path)`; teach ingestion to parse the pair param as a path (reuse
  `parse_path`). Predicate-only cases keep working (a bare IRI is a `Path::Predicate`).
- **M3. `sh:uniqueLang` with base direction** — 1 test (`property/uniqueLang-003`). RDF-1.2
  directional literals (`"x"@ar--ltr`) make the uniqueness key **(language, direction)**, not language
  alone. Include the literal's direction in `UniqueLangValidator`'s key (oxrdf `Literal` direction API).
- **M4. Implicit `sh:ShapeClass` targets** — 1–2 tests (`targets/targetClassImplicit-002`, partial
  `node/in-002`). Treat a shape typed `sh:ShapeClass` (not only `rdfs:Class`) as an implicit class
  target, and ensure subclass instances are picked up via the existing `is_shacl_instance` walk.

### Tier 3 — larger 1.2 features (~12 tests)

- **L1. `sh:closed sh:ByTypes`** — 2 tests (`node/closed-003/004`). New 1.2 closure variant: the
  permitted predicate set is computed per `rdf:type` of the focus (each type's own + inherited
  property shapes) rather than globally. Extend `other::ClosedValidator` to accept the `sh:ByTypes`
  IRI value and resolve allowed predicates per the focus node's types.
- **L2. `sh:qualifiedValueShapesDisjoint`** — 2 tests (`property/qualifiedValueShapesDisjoint-001`,
  `qualifiedMinCountDisjoint-001`). When `true`, a value node only counts toward the qualified count
  if it conforms to **no sibling** `sh:qualifiedValueShape`. Thread sibling qualified shapes into
  `QualifiedValidator` and subtract them.
- **L3. `sh:uniqueValuesFor`** — 3 tests (`node/uniqueValuesFor-001/002/003`). Property-level
  cross-focus uniqueness: a value reached via the path must be unique across the focus nodes sharing
  the given property. Needs a value→focus index built over the target set.
- **L4. `sh:someValue` + `sh:rootClass`** — 2 tests (`property/someValue-001`, `property/rootClass-001`).
  `sh:someValue`: at least one value node conforms to the referenced shape (else one result).
  `sh:rootClass`: each value node must be an `rdfs:subClassOf*` of the root class. Both are small
  validators in the `shape`/`value_type` modules once their exact 1.2 semantics are pinned.
- **L5. `sh:targetWhere` + explicit `sh:shape` targets** — 2 tests (`targets/targetWhere-001`,
  `targets/shape-001`). `sh:targetWhere` (REQ-TGT-5, ADR-007): naive iteration — a node is a focus if
  it conforms to the inner shape. Explicit `sh:shape` (REQ-TGT-6): read `(node, sh:shape, thisShape)`
  links from the **data** graph. Both need the shape registry already on `Ctx`.

### Tier 4 — RDF-1.2 reification & node expressions (~7 tests)

- **X1. Reifier annotations + `sh:reifierShape`/`sh:reificationRequired`** — 3 tests
  (`misc/deactivated-003`, `property/reifierShape-001/002`). Parse RDF-1.2 reifier annotation syntax
  (`{| sh:deactivated true |}`) in ingestion into per-constraint reifier metadata, and add the
  `sh:reifierShape` family (validate the reifying triple-term against a shape). Largest parser change.
- **X2. `sh:nodeByExpression` / SHACL node expressions** — 1 test (`node/nodeByExpression-001`).
  Core node-expression evaluation (at minimum the IRI-expression form). A self-contained sub-feature;
  schedule last.
- **X3. `shsh:` shapes-graph well-formedness** — 2 tests (`node/in-003`,
  `validation-reports/conformance-disallows-001`). These validate the *shapes graph itself* against
  the SHACL-SHACL metashapes; needs the `shsh:` vocabulary prefix + metashape validation pass.
- **X4. Stragglers** — `node/in-002` (empty `sh:in` + implicit-class targeting interaction) — revisit
  after M4; may resolve or need a targeted fix.

### Sequencing & expected yield
1. **Tier 1** (R1–R4): ~10 tests, ~half a day. R1 (runner) first — it also exposes the true failure
   reason for the separate-file tests behind it.
2. **Tier 2** (M1–M4): ~12 tests. M1/M2 are the biggest single wins.
3. **Tier 3** (L1–L5): ~12 tests, one feature at a time.
4. **Tier 4** (X1–X4): ~7 tests; X1 (reification) is the heaviest and lowest-yield-per-effort — do last.

After each workstream: re-run `cargo run -p shacl-testsuite -- <suite>/tests/core`, add any newly-green
tests to the vendored `shacl-testsuite/tests/fixtures/` offline gate, and `cargo fmt`/`clippy`/commit.

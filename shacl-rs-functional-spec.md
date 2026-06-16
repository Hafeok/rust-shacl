# shacl-rs — Functional Specification

| | |
|---|---|
| **Status** | Draft v0.8 |
| **Scope (v1)** | SHACL 1.2 Core + SHACL 1.2 SPARQL Extensions |
| **Normative target** | W3C **SHACL 1.2 Core**, Working Draft 10 April 2026 (`https://www.w3.org/TR/shacl12-core/`) + **SHACL 1.2 SPARQL Extensions** (`https://www.w3.org/TR/shacl12-sparql/`) |
| **Conformance oracle** | W3C **SHACL 1.2 test suite** (`github.com/w3c/data-shapes/tree/gh-pages/shacl12-test-suite`) |
| **Primary readers** | (1) LLM implementation agent · (2) human contributors · (3) conformance auditors |

> **Stability caveat.** SHACL 1.2 Core is a *Working Draft*, not a Recommendation. Section
> numbers and component sets may shift before REC. Every \`W3C §\` cell is a snapshot of the
> 10 April 2026 draft; the conformance-matrix tooling (§10) must re-verify anchors on each spec
> refresh. SHACL-C examples in the spec are explicitly informative/unstable and are out of scope.
> See ADR-001.

---

## 0. About this document

Single source of truth for \`shacl-rs\`, serving three readers through one mechanism: the numbered
**requirement** (\`REQ-*\`). Each carries an RFC-2119 keyword, a normative statement, an upstream
W3C link, and downstream test-suite links. Humans read the prose; an implementation agent treats
each §7 component subsection as a work packet; an auditor reads the conformance matrix (§10). A
requirement with no test reference is, by construction, a visible gap.

### 0.1 RFC-2119 keywords
MUST, MUST NOT, SHOULD, SHOULD NOT, MAY per BCP-14, only when capitalised. Shown in the \`Kw\` column.

### 0.2 Identifier scheme

| Prefix | Meaning | Traced to |
|--------|---------|-----------|
| \`REQ-<AREA>-<n>\` | Atomic functional requirement | W3C § (up), Tests[] (down) |
| \`CMP-<NAME>\` | Constraint component (clusters REQs + parameters + report contract) | W3C §7.x |
| \`ADR-<n>\` | Decision where the spec leaves a choice open | rationale + literature |

\`<AREA>\`: \`TGT\` targets, \`PATH\` paths, \`RPT\` report, \`SPQ\` SHACL-SPARQL, \`REC\` recursion,
\`ING\` ingestion, \`TERM\` term-model/RDF-1.2, plus component name for component REQs.
IDs are **append-only and immutable**; retire as \`WITHDRAWN\`, never reuse or renumber.

### 0.3 Extending this document
New component → clone the §7 template, allocate \`CMP-<NAME>\`, fill parameters + report contract +
\`REQ-<NAME>-n\` rows + test links. New decision → add \`ADR-<n>\` in Appendix A and reference it from
affected REQs. Never change a requirement's behavior in place without a new ID.

---

## 1. Conformance

### 1.1 Definition
A focus node **conforms** to a shape iff validating it produces no results and reports no failure
(§6.6). Conformance checking underlies \`sh:not\`/\`sh:and\`/\`sh:or\`/\`sh:xone\`/\`sh:node\`/\`sh:someValue\`,
which invoke it internally rather than emitting results.

### 1.2 Conformance levels

| Level | Requirement set | Backend need |
|-------|-----------------|--------------|
| **L1 — Core** | all non-WITHDRAWN MUST REQs in §3–§7 | \`RdfGraph\` only |
| **L2 — Core + SPARQL** | L1 + §8 | \`SparqlGraph\` |
| **L3 — Recursion-defined** | L2 + §9 | \`SparqlGraph\` |

The processor declares its level. Per the W3C note, passing the suite implies conformance only to
tested aspects.

### 1.3 The test suite as oracle
For spec-defined behavior the SHACL 1.2 manifest entry *is* the definition of correct; listed in
each REQ's \`Tests[]\`. Under-specified behavior (recursion, some SPARQL binding edges) is pinned by
an ADR plus differential tests against a reference processor recorded as \`Tests[diff:…]\`.

---

## 2. Architecture (normative seam)

Two traits decouple the engine from any store; the split is load-bearing for conformance levels.

- **\`RdfGraph\`** — \`triples(s?, p?, o?)\`. Sufficient for all Core constraints (§7), targets (§3.1.3
  except where-targets caveat), and path evaluation (§4).
- **\`SparqlGraph: RdfGraph\`** — adds \`select\`/\`ask\` with pre-bound variables. Required by §8, and
  by \`sh:targetWhere\` if implemented via conformance over arbitrary shapes (see REQ-TGT note).

> **REQ-ARCH-1** — MUST — The Core engine (§3–§7) compiles and runs with no SPARQL-engine crate in
> the dependency tree. — *verified by: build with \`oxigraph\` absent.*
> **REQ-ARCH-2** — MUST — Path evaluation (§4) runs over \`RdfGraph\` only and MUST NOT require
> compiling to SPARQL. — W3C §4 — Tests: path/*.
> **REQ-ARCH-3** — SHOULD — \`RdfGraph\` provides an overridable \`reach(start, path)\` whose default
> is expressed via \`triples()\`, so SPARQL/remote backends can push path closure down as one query.
> — ADR-003.

RDF 1.2 note: 1.2 Core uses triple terms / reifiers for per-constraint severity, message, and
deactivation (§3.1.4–§3.1.6). The term model MUST represent triple terms and reifier nodes. See
ADR-004, REQ-TERM-*.

---

## 3. Shapes graph ingestion & shape model

### 3.1 Shape identification  (W3C §3.1)
> **REQ-ING-1** — MUST — A node is a shape if it is a SHACL instance of \`sh:NodeShape\`/
> \`sh:PropertyShape\`, OR is subject of a triple whose predicate is a target predicate
> (\`sh:targetClass|targetNode|targetObjectsOf|targetSubjectsOf\`), OR subject of a triple whose
> predicate is a parameter, OR is a value (or list member) of a shape-expecting parameter. — §3.1
> **REQ-ING-2** — MUST — A property shape is a shape that is subject of a \`sh:path\` triple; a node
> shape is a shape that is not. The two sets are disjoint. — §3.2, §3.3
> **REQ-ING-3** — MUST — A shape has at most one \`sh:path\` value; a node shape MUST NOT have
> \`sh:path\`. — §3.3
> **REQ-ING-4** — MUST — Single-parameter components (e.g. \`sh:class\`) may repeat; each value is an
> independent conjunctive constraint. — §3.1.1
> **REQ-ING-5** — MUST — Multi-parameter components (e.g. \`sh:pattern\`+\`sh:flags\`) with >1 value for
> any of their parameters → shape is **ill-formed**. — §3.1.1
> **REQ-ING-6** — MUST — A shapes graph containing any ill-formed node is ill-formed; handling per
> §6.5.2. — §1.3, §6.5.2

### 3.2 Severity, message, deactivation  (W3C §3.1.4–§3.1.6) — **RDF 1.2 reification**
> **REQ-ING-7** — MUST — Default severity is \`sh:Violation\`. A shape may carry one \`sh:severity\`
> (an IRI); SHACL severities are Trace/Debug/Info/Warning/Violation. — §3.1.4
> **REQ-ING-8** — MUST — \`sh:severity\` MAY also appear on a **reifier** of a (shape, parameter, …)
> triple, giving per-constraint severity; at most one such value per constraint's triple set. — §3.1.4
> **REQ-ING-9** — MUST — If a shape has any \`sh:message\`, all its results copy exactly those messages
> into \`sh:resultMessage\`. \`sh:message\` may likewise appear on a reifier. Datatypes: xsd:string,
> rdf:langString, rdf:dirLangString, rdf:HTML. — §3.1.5
> **REQ-ING-10** — MUST — \`sh:deactivated\` value is a node expression that must yield only \`true\` or
> \`false\`; in Core the only valid values are the literals \`true\`/\`false\`. A deactivated shape is
> ignored. Per-constraint deactivation is via a reifier. — §3.1.6
> **REQ-TERM-1** — MUST — The term model represents RDF 1.2 triple terms and reifiers sufficiently
> to attach \`sh:severity\`/\`sh:message\`/\`sh:deactivated\` to individual constraint triples. — §3.1.4–6, ADR-004

*(stub — \`sh:ShapeClass\` handling, \`sh:values\`/\`sh:defaultValue\` (not required by Core; needed for
SPARQL ext), \`sh:order\`/\`sh:group\`/\`sh:name\`/\`sh:description\` non-validating props §8.)*

---

## 4. Property paths  \`[REQ-PATH-*]\`  (W3C §4)

1.2 defines paths by a mapping \`path(p,G)\` to **SPARQL 1.2** property paths. Supported subset:
Predicate, Inverse, Sequence, Alternative, ZeroOrMore, OneOrMore, ZeroOrOne.

> **REQ-PATH-1** — MUST — Predicate path (an IRI) maps to SPARQL \`PredicatePath\`; value nodes are
> objects of \`(focus, iri, o)\`. — §4.1
> **REQ-PATH-2** — MUST — Sequence path: a SHACL list of ≥2 well-formed paths → SPARQL \`SequencePath\`.
> — §4.2
> **REQ-PATH-3** — MUST — Alternative path: blank node with one \`sh:alternativePath\` list value →
> \`AlternativePath\`. — §4.3
> **REQ-PATH-4** — MUST — Inverse (\`sh:inversePath\`), ZeroOrMore (\`sh:zeroOrMorePath\`), OneOrMore
> (\`sh:oneOrMorePath\`), ZeroOrOne (\`sh:zeroOrOnePath\`) map to their SPARQL counterparts. — §4.4–4.7
> **REQ-PATH-5** — MUST — \`*\`/\`+\` closures return **distinct** reachable nodes and terminate on
> cyclic *data*. — §4 (SPARQL pp semantics)
> **REQ-PATH-6** — MUST — A blank-node path whose mapping directly or transitively references itself
> is **ill-formed** (cycle in the *path*, distinct from cyclic data). — §4

### 4.1 Formal semantics of path evaluation (the provable core)

Path evaluation is the part of SHACL that *is* a math problem with a checkable answer. Each SHACL
path \`p\` denotes a binary relation \`⟦p⟧ ⊆ V × V\` over the nodes \`V\` of the data graph \`G\`. The value
nodes of focus \`f\` under \`p\` are \`{ o | (f,o) ∈ ⟦p⟧ }\`. The relation is defined compositionally:

\`\`\`
⟦iri⟧            = { (s,o) | (s,iri,o) ∈ G }                       (predicate, REQ-PATH-1)
⟦inverse(p)⟧     = { (o,s) | (s,o) ∈ ⟦p⟧ }                         (REQ-PATH-4)
⟦seq(p1..pn)⟧    = ⟦p1⟧ ∘ … ∘ ⟦pn⟧        (relation composition)   (REQ-PATH-2)
⟦alt(p1..pn)⟧    = ⟦p1⟧ ∪ … ∪ ⟦pn⟧                                 (REQ-PATH-3)
⟦zeroOrOne(p)⟧   = Δ ∪ ⟦p⟧                 (Δ = identity on V)      (REQ-PATH-4)
⟦oneOrMore(p)⟧   = ⟦p⟧⁺  = least R ⊇ ⟦p⟧ with R ∘ ⟦p⟧ ⊆ R          (transitive closure)
⟦zeroOrMore(p)⟧  = ⟦p⟧*  = Δ ∪ ⟦p⟧⁺                                (reflexive-transitive closure)
\`\`\`

> **REQ-PATH-7** — MUST — \`+\`/\`*\` are evaluated as the **least fixpoint** of \`R ↦ ⟦p⟧ ∪ (R ∘ ⟦p⟧)\`.
> Because \`V\` is finite, \`V×V\` is a finite complete lattice and the operator is monotone, so the
> Knaster–Tarski least fixpoint exists and is reached in ≤ |V| iterations (worklist/BFS frontier).
> Termination on cyclic data is therefore guaranteed, not best-effort. — §4

> **REQ-PATH-8** — MUST — Value-node *sets* are deduplicated (a node reachable by two routes appears
> once), but a fixed-length \`seq\`/\`alt\` does **not** deduplicate intermediate multiplicity in a way
> that changes the final set. The observable result of any path is a **set** of value nodes. — §4

**Reference (oracle) algorithm.** A deliberately naive evaluator used only in tests: materialise
\`⟦p⟧\` by structural recursion, computing \`⁺\` by repeated relational composition until no change
(\`R_{k+1} = R_k ∪ (R_k ∘ ⟦p⟧)\`; stop when \`R_{k+1} = R_k\`). \`O(|V|³)\`-ish, obviously correct, never
shipped. The production \`reach()\` (ADR-003) is the optimized frontier/BFS version.

> **REQ-PATH-9** — MUST (test) — Property tests assert, on randomly generated graphs and paths:
> (a) **idempotence** \`⟦p*⟧ = ⟦(p*)*⟧\`; (b) **closure fixed point** \`⟦p⁺⟧ ∘ ⟦p⟧ ⊆ ⟦p⁺⟧\`;
> (c) **production = oracle**: optimized \`reach()\` equals the naive evaluator for all generated
> inputs; (d) **termination** within |V| frontier rounds. These are the proof obligations from the
> design discussion, discharged empirically via \`proptest\`. — methodology, not a W3C §

This same least-fixpoint machinery (\`closure\` helper, §11.4) backs SHACL-subclass walking
(REQ-CLASS-2) and recursion cycle detection (ADR-002); it is written and property-tested **once**,
before any component (build step 2, §11.5).

---

## 5. Value nodes & validation  \`[REQ-RPT-*]\`  (W3C §6)

> **REQ-RPT-1** — MUST — Value nodes of a node shape = { focus node }. Value nodes of a property
> shape = nodes reachable from focus via \`sh:path\`. — §6.8
> **REQ-RPT-2** — MUST — Report is a \`sh:ValidationReport\` with \`sh:conforms\` = (no result of
> severity \`sh:Violation\`). — §6.7.1
> **REQ-RPT-3** — MUST — Each \`sh:ValidationResult\` carries \`sh:focusNode\`, \`sh:resultSeverity\`,
> \`sh:sourceConstraintComponent\`, \`sh:sourceShape\`; plus \`sh:resultPath\`/\`sh:value\` where
> applicable; \`sh:resultMessage\` per REQ-ING-9; optional \`sh:detail\`. — §6.7.2
> **REQ-RPT-4** — MUST — Ill-formed shapes graph handling per §6.5.2; failures per §6.5.1 signalled
> distinctly from violations. — §6.5
> **REQ-RPT-5** — MAY — Populate \`sh:shapesGraphWellFormed\`. — §6.7.1.4

*(stub — \`sh:conformanceDisallows\` §6.7.1.2 (new in 1.2 — investigate semantics); detail nesting
for logical/shape components; report serialization back over the backend.)*

---

## 6. Targets  \`[REQ-TGT-*]\`  (W3C §3.1.3)

> **REQ-TGT-1** — MUST — \`sh:targetNode\`: each value is a node expression; its output nodes are
> targets. (Core: constant IRIs/literals.) — §3.1.3.1
> **REQ-TGT-2** — MUST — \`sh:targetClass\`: SHACL instances of the class (incl. subclass walk via
> SHACL-subclass). — §3.1.3.2
> **REQ-TGT-3** — MUST — Implicit class target: a shape that is also a SHACL instance of \`rdfs:Class\`
> (or of \`sh:ShapeClass\`) targets its SHACL instances. A non-IRI shape that is also an rdfs:Class is
> ill-formed. — §3.1.3.3
> **REQ-TGT-4** — MUST — \`sh:targetSubjectsOf\` / \`sh:targetObjectsOf\`: subjects/objects of triples
> with the given predicate. — §3.1.3.4–5
> **REQ-TGT-5** — MUST — \`sh:targetWhere\`: nodes that **conform** to the given shape are targets.
> *Implementation note:* worst case iterates all data-graph nodes; spec warns performance varies.
> Conformance-based → may require \`SparqlGraph\` depending on the inner shape. — §3.1.3.6
> **REQ-TGT-6** — MUST — \`sh:shape\` (explicit shape target): a triple **in the data graph** linking
> node→shape makes that node a target. Note this reads the *data* graph, unlike other targets. — §3.1.3.7
> **REQ-TGT-7** — MUST — Targets are ignored when a focus node is supplied directly (e.g. via
> \`sh:node\`). — §3.1.3

---

## 7. Core constraint components  \`[CMP-*]\`  (W3C §7)

**Template** (clone per component):
> \`\`\`
> ### CMP-<NAME> — sh:<param>
> **Applies to:** node | property | both    **W3C:** §7.x.y
> **Parameters:** <param> (<datatype>, cardinality); <optional params>
> | REQ | Kw | Statement | W3C § | Tests |
> **Report result:** sourceConstraintComponent=…; focusNode/value/resultPath population
> **Pre-conditions:** value-node computation (§5/§4) assumed
> \`\`\`

---

### CMP-MINCOUNT — \`sh:minCount\`   *(worked example — clone this)*
**Applies to:** property shapes only   **W3C:** §7.2.1
**Parameters:** \`sh:minCount\` (xsd:integer, exactly one)

| REQ | Kw | Statement | W3C § | Tests |
|-----|----|-----------|-------|-------|
| REQ-MINCOUNT-1 | MUST | If count of value nodes < \`sh:minCount\`, produce exactly one result. | §7.2.1 | core/property/minCount-* |
| REQ-MINCOUNT-2 | MUST | Count is over distinct value nodes per §5/§4. | §7.2.1 | core/property/minCount-* |
| REQ-MINCOUNT-3 | MUST | \`sh:minCount\` 0 never produces a result. | §7.2.1 | core/property/minCount-* |

**Report result:** \`sh:sourceConstraintComponent\` = \`sh:MinCountConstraintComponent\`; \`sh:focusNode\`
= focus; \`sh:resultPath\` = shape's path; **no** \`sh:value\` (violation is absence).
**Pre-conditions:** value nodes per §5; property-shape only (node-shape use ill-formed, REQ-ING-3).

---

### CMP-PATTERN — \`sh:pattern\`   *(worked example — multi-parameter)*
**Applies to:** both   **W3C:** §7.4.3
**Parameters:** \`sh:pattern\` (xsd:string, exactly one); \`sh:flags\` (xsd:string, optional)

| REQ | Kw | Statement | W3C § | Tests |
|-----|----|-----------|-------|-------|
| REQ-PATTERN-1 | MUST | For each value node, if lexical form does not match the regex, produce one result for that value node. | §7.4.3 | core/property/pattern-* |
| REQ-PATTERN-2 | MUST | If \`sh:flags\` present, apply per SPARQL \`REGEX\` flag semantics. | §7.4.3 | core/property/pattern-* |
| REQ-PATTERN-3 | MUST | Match uses lexical form; a value node with no lexical form (e.g. blank node) produces a result. | §7.4.3 | core/node/pattern-* |
| REQ-PATTERN-4 | MUST NOT | >1 value for \`sh:pattern\` or \`sh:flags\` → ill-formed (REQ-ING-5). | §7.4.3 | — |

**Report result:** \`sh:PatternConstraintComponent\`; \`sh:value\` = failing value node; \`sh:resultPath\`
if property shape.
**Pre-conditions:** regex MUST follow SPARQL/XPath \`REGEX\` semantics — Rust \`regex\` diverges, see ADR-005.

---

### CMP-CLASS — \`sh:class\`
**Applies to:** both node and property shapes   **W3C:** §7.1.1
**Parameters:** \`sh:class\` (rdfs:Resource — an IRI; may repeat → conjunction per REQ-ING-4)

| REQ | Kw | Statement | W3C § | Tests |
|-----|----|-----------|-------|-------|
| REQ-CLASS-1 | MUST | For each value node \`v\` and each value \`C\` of \`sh:class\`: if \`v\` is **not** a SHACL instance of \`C\` in the data graph, produce one result for \`v\`. | §7.1.1 | core/node/class-*, core/property/class-* |
| REQ-CLASS-2 | MUST | "SHACL instance of \`C\`" follows the SHACL-type definition: \`v\` has \`rdf:type\` \`T\` where \`T\` = \`C\` or \`T\` is a SHACL-subclass of \`C\` (transitive \`rdfs:subClassOf\` walk). | §1.1 (SHACL instance/subclass/type) | core/*/class-* |
| REQ-CLASS-3 | MUST | The \`rdfs:subClassOf\` walk reads the data graph by default; if configured (§6.3) it MAY also read \`rdfs:subClassOf\` from the shapes graph. | §6.3 | — |
| REQ-CLASS-4 | MUST | Repeated \`sh:class\` values are independent conjunctive constraints; a value node violating two classes yields two results. | §3.1.1 | core/property/class-002 (multi) |

**Report result:** \`sh:sourceConstraintComponent\` = \`sh:ClassConstraintComponent\`; \`sh:value\` =
the failing value node; \`sh:resultPath\` = shape's path iff property shape; \`sh:focusNode\` = focus.
**Pre-conditions:** value nodes per §5. Requires the SHACL-instance helper (shared with REQ-TGT-2,
implicit targets, \`sh:qualifiedValueShape\`) — implement once as \`is_shacl_instance(node, class)\`
over \`RdfGraph\` with memoised transitive \`rdfs:subClassOf\` closure (least-fixpoint, terminates on
cyclic subclass graphs — same closure machinery as REQ-PATH-5).
**Reference SPARQL validator** (informative, SPARQL-ext App. C.4): \`ASK { \$value rdf:type/rdfs:subClassOf* \$class }\`
returns true for conforming nodes — useful as the differential-test oracle for the native impl.

---

### CMP-DATATYPE — \`sh:datatype\`
**Applies to:** both node and property shapes   **W3C:** §7.1.2
**Parameters:** \`sh:datatype\` (an IRI naming a datatype; **exactly one** per shape — multi-value is
ill-formed, unlike \`sh:class\`)

| REQ | Kw | Statement | W3C § | Tests |
|-----|----|-----------|-------|-------|
| REQ-DATATYPE-1 | MUST | For each value node \`v\`: if \`v\` is not a literal, or its datatype IRI ≠ \`sh:datatype\`, produce one result. | §7.1.2 | core/node/datatype-*, core/property/datatype-* |
| REQ-DATATYPE-2 | MUST | The literal MUST additionally be **well-formed** with respect to that datatype's lexical space; a literal whose lexical form is illegal for its datatype (e.g. \`"abc"^^xsd:integer\`) produces a result even when the datatype IRI matches. | §7.1.2 | core/*/datatype-* (ill-formed lexical) |
| REQ-DATATYPE-3 | MUST NOT | More than one \`sh:datatype\` value → shape ill-formed (REQ-ING-5). | §7.1.2 | — |
| REQ-DATATYPE-4 | MUST | For \`rdf:langString\`, a value node is conforming only if it is a language-tagged literal; for \`xsd:*\` types the language tag must be absent. | §7.1.2 | core/*/datatype-* |

**Report result:** \`sh:DatatypeConstraintComponent\`; \`sh:value\` = failing value node; path iff property shape.
**Pre-conditions:** value nodes per §5. **Lexical-space validation (REQ-DATATYPE-2) is the subtle
part** — "datatype IRI matches" is necessary but not sufficient. Needs an \`xsd\`-aware lexical
validator. \`oxsdatatypes\` (an oxigraph crate) implements XSD lexical/value spaces and SHOULD be the
backing impl rather than hand-rolled regex per type. See ADR-010.
**Reference oracle:** pySHACL/TopBraid datatype handling for the lexical-validity edge cases (these
are under-exercised in the suite and a known interop-divergence area).

---

### CMP-NODEKIND — \`sh:nodeKind\`
**Applies to:** both node and property shapes   **W3C:** §7.1.3
**Parameters:** \`sh:nodeKind\` (one of the six \`sh:NodeKind\` IRIs; exactly one)

The six kinds and the set each admits:
\`sh:IRI\` {IRI}, \`sh:BlankNode\` {blank}, \`sh:Literal\` {literal},
\`sh:BlankNodeOrIRI\` {blank, IRI}, \`sh:BlankNodeOrLiteral\` {blank, literal},
\`sh:IRIOrLiteral\` {IRI, literal}.

| REQ | Kw | Statement | W3C § | Tests |
|-----|----|-----------|-------|-------|
| REQ-NODEKIND-1 | MUST | For each value node \`v\`: if \`v\`'s kind is not in the admitted set of the declared \`sh:nodeKind\`, produce one result. | §7.1.3 | core/node/nodeKind-*, core/property/nodeKind-* |
| REQ-NODEKIND-2 | MUST | The value MUST be exactly one of the six \`sh:NodeKind\` IRIs; any other value → ill-formed. | §7.1.3 | — |
| REQ-NODEKIND-3 | MUST NOT | More than one \`sh:nodeKind\` value → ill-formed. | §7.1.3 | — |

**Report result:** \`sh:NodeKindConstraintComponent\`; \`sh:value\` = failing value node; path iff property shape.
**Pre-conditions:** value nodes per §5. Pure term-kind inspection — no graph access, no SPARQL, no
closure. This is the **simplest component**: a 6-way match on the \`Term\` discriminant. Good first
code to write and a clean fixture for wiring up the validator-dispatch + report-construction plumbing
that every other component reuses.

---

### Remaining components (stubs — same template; §7.1 fully expanded above). Section numbers from 1.2 Core WD 2026-04-10.

| Group | Component | CMP id | W3C § | Applies | Notes |
|-------|-----------|--------|-------|---------|-------|
| Cardinality §7.2 | sh:maxCount | CMP-MAXCOUNT | §7.2.2 | property | |
| Range §7.3 | sh:minExclusive/minInclusive/maxExclusive/maxInclusive | CMP-RANGE-* | §7.3.1–4 | both | |
| String §7.4 | sh:minLength/maxLength | CMP-LENGTH-* | §7.4.1–2 | both | |
| | sh:singleLine | CMP-SINGLELINE | §7.4.4 | both | **new in 1.2** |
| | sh:languageIn | CMP-LANGUAGEIN | §7.4.5 | both | |
| | sh:uniqueLang | CMP-UNIQUELANG | §7.4.6 | property | |
| List §7.5 | sh:memberShape | CMP-MEMBERSHAPE | §7.5.1 | both | **new in 1.2** |
| | sh:minListLength/maxListLength | CMP-LISTLEN-* | §7.5.2–3 | both | **new in 1.2** |
| | sh:uniqueMembers | CMP-UNIQUEMEMBERS | §7.5.4 | both | **new in 1.2** |
| Property pair §7.6 | sh:equals/disjoint | CMP-PAIR-* | §7.6.1–2 | property | |
| | sh:subsetOf | CMP-SUBSETOF | §7.6.3 | property | **new in 1.2** |
| | sh:lessThan/lessThanOrEquals | CMP-PAIR-* | §7.6.4–5 | property | |
| Logical §7.7 | sh:not/and/or/xone | CMP-LOGIC-* | §7.7.1–4 | both | use conformance checking |
| Shape §7.8 | sh:node | CMP-NODE | §7.8.1 | both | |
| | sh:property | CMP-PROPERTY | §7.8.2 | both | |
| | sh:someValue | CMP-SOMEVALUE | §7.8.3 | both | **new in 1.2** |
| | sh:qualifiedValueShape (+min/maxCount, disjoint) | CMP-QUAL | §7.8.4 | property | |
| | sh:reifierShape, sh:reificationRequired | CMP-REIFIER | §7.8.5 | property | **new in 1.2 / RDF 1.2** |
| Other §7.9 | sh:closed, sh:ignoredProperties | CMP-CLOSED | §7.9.1 | node | \`sh:ByTypes\` value variant new in 1.2 |
| | sh:hasValue | CMP-HASVALUE | §7.9.2 | both | |
| | sh:in | CMP-IN | §7.9.3 | both | |
| | sh:rootClass | CMP-ROOTCLASS | §7.9.4 | both | **new in 1.2** |
| | sh:uniqueValuesFor | CMP-UNIQUEVALUESFOR | §7.9.5 | property | **new in 1.2** |

---

## 8. SHACL-SPARQL  \`[REQ-SPQ-*]\`  (W3C **SHACL 1.2 SPARQL Extensions**, WD 2026-01-30)

Requires \`SparqlGraph\`. Conformance problem, not a math problem: behavior is pinned by the spec text
+ reference-processor diff tests. Two features: **SPARQL-based constraints** (§8.1, spec §2) and
**SPARQL-based constraint components** (§8.2, spec §3). Pre-binding mechanics (§8.4) are shared and
**Feature-at-Risk**.

### 8.1 SPARQL-based constraints  (spec §2)
Component IRI \`sh:SPARQLConstraintComponent\`; parameter \`sh:sparql\`.

> **REQ-SPQ-1** — MUST — A shape's \`sh:sparql\` value (IRI/blank node) has exactly one \`sh:select\`
> (xsd:string); after prefix handling it must parse as a valid SPARQL 1.2 SELECT projecting \`this\`.
> Non-parse → **failure**. — spec §2.2
> **REQ-SPQ-2** — MUST — Execute the SELECT with \`this\` pre-bound to the focus node (§8.4). Produce
> one validation result per solution that does **not** bind \`failure\`=true. — spec §2.3
> **REQ-SPQ-3** — MUST — If exactly one solution binds \`failure\`=true, signal a **failure** (distinct
> from a violation). — spec §2.3
> **REQ-SPQ-4** — MUST — In a property shape, before execution substitute the \`PATH\` variable —
> only where it occurs in **predicate position** of a triple pattern — with the SPARQL surface
> syntax of the shape's \`sh:path\`. \`PATH\` anywhere else → ill-formed. — spec §2.2, §2.3
> **REQ-SPQ-5** — MUST — Map each solution to result properties in this precedence: \`sh:focusNode\`
> ← \`this\`; \`sh:resultPath\` ← \`?path\` if IRI, else shape's path; \`sh:value\` ← \`?value\` else the
> value node; \`sh:resultMessage\` ← \`?message\` else constraint's \`sh:message\` (with \`{?var}\`/\`{\$var}\`
> substitution); \`sh:sourceConstraint\` ← the \`sh:sparql\` value. — spec §2.3.2
> **REQ-SPQ-6** — MUST — No results if the constraint has \`sh:deactivated\` = true. — spec §2.3

### 8.2 SPARQL-based constraint components  (spec §3)
Reusable components: an IRI typed \`sh:ConstraintComponent\`, with \`sh:parameter\` declarations and
SELECT- or ASK-based validators.

> **REQ-SPQ-7** — MUST — Parameter declarations (\`sh:parameter\`) have exactly one IRI-valued
> \`sh:path\`; the **parameter name** is its local name (longest trailing NCName after the first
> colon). Names must be valid SPARQL VARNAMEs, must not be \`this\`/\`path\`/\`PATH\`/\`value\`, and must be
> unique within a component; ≥1 non-optional parameter. Violations → ill-formed. — spec §3.2.1
> **REQ-SPQ-8** — MUST — Validator selection order: node shape → a \`sh:nodeValidator\`; property
> shape → a \`sh:propertyValidator\`; else \`sh:validator\`. No suitable validator → ignore the
> constraint. \`sh:nodeValidator\`/\`sh:propertyValidator\` are SELECT-based; \`sh:validator\` is
> ASK-based. — spec §3.2.3
> **REQ-SPQ-9** — MUST — **ASK validator**: for each value node \`v\`, run ASK with \`value\`=v (and
> \`this\`, params) pre-bound; the ASK returns true for **conforming** nodes, so emit a solution
> (\`this\`,focus)+(\`value\`,v) for every \`v\` where ASK = false. — spec §3.2.3.2, §3.3
> **REQ-SPQ-10** — MUST — **SELECT validator**: substitute \`PATH\` (property shape) per REQ-SPQ-4,
> execute; solutions are the (non-conforming) results directly. — spec §3.2.3.1, §3.3
> **REQ-SPQ-11** — MUST — Pre-bind \`this\` (§8.4) and **each parameter value** as a variable named by
> the parameter name, for every validator execution. — spec §3.3
> **REQ-SPQ-12** — MUST — Result-property mapping is identical to REQ-SPQ-5. — spec §3.3

### 8.3 Prefixes & annotations
> **REQ-SPQ-13** — MUST — Collect prefix mappings via the property path
> \`sh:prefixes/owl:imports*/sh:declare\`; each \`sh:declare\` gives one \`sh:prefix\` (xsd:string) +
> \`sh:namespace\` (xsd:anyURI). Conflicting namespaces for one prefix → ill-formed. Prepend the
> mappings as \`PREFIX\` lines before parsing. — spec §2.2.1
> **REQ-SPQ-14** — SHOULD — Support \`sh:resultAnnotation\` (\`sh:annotationProperty\` +
> \`sh:annotationVarName\`/\`sh:annotationValue\`) to copy extra solution bindings onto results. — spec §4

### 8.4 Pre-binding  ⚠ **Feature at Risk** (spec Appendix A, Issue 647)
Pre-binding is **not** a VALUES injection; it is Values-Insertion rewriting the algebra:
\`eval(Q, μ) ≡ eval((Replace(E, μ), DS, QF))\`. Build it behind an abstraction so it can track
SPARQL 1.2 changes.

> **REQ-SPQ-15** — MUST — A shapes graph whose \`sh:select\`/\`sh:ask\` queries violate any pre-binding
> restriction → **failure**. Restrictions on queries mentioning a *potentially pre-bound* variable
> (\`this\`, \`value\` for ASK, and all component parameter names): no \`MINUS\`; no \`SERVICE\`; no
> \`VALUES\` mentioning such a variable; no \`AS ?var\` rebinding such a variable. — spec App. A
> **REQ-SPQ-16** — MUST — Implement pre-binding as Values-Insertion over the algebra (join each BGP/
> path/GRAPH pattern with the singleton solution), not as surface-string editing. — spec App. A

**Open items:** literal/newline escaping when prepending PREFIX lines (ADR-006); the at-risk status
of pre-binding (ADR-008 — abstract it); \`{?var}\`/\`{\$var}\` message-template substitution rules; SPARQL
\`EXISTS\` cross-engine divergence (flagged by Core). Node Expressions (spec §5: \`sh:SelectExpression\`,
\`sh:SPARQLExprExpression\`, plus \`sh:values\`/\`sh:defaultValue\`) are a **separate 1.2 doc** → Appendix C,
out of scope for v1 except constant \`sh:targetNode\`/\`sh:deactivated\` evaluation.

---

## 9. Recursion semantics  \`[REQ-REC-*]\`  (W3C §6.5.3)  (Level L3, optional)

1.2 still leaves recursion to the implementation (§6.5.3 Handling of Recursive Shapes). Default:
reject. L3 option: adopt one formal semantics (four surveyed in arXiv 2108.13063). Choice = ADR-002.

> **REQ-REC-1** — MUST — With recursion disabled, a recursive shape reference → diagnostic, no
> silent partial validation. — §6.5.3, ADR-002
> **REQ-REC-2** — MUST (L3) — Recursive validation implements ADR-002's semantics; verified against
> hand-built fixpoint cases + diff:reference. — ADR-002

### 9.1 Cycle detection (v1, L1/L2)
The shape-reference graph has an edge \`A → B\` whenever shape \`A\` references \`B\` through a
shape-expecting parameter (\`sh:node\`, \`sh:property\`, \`sh:not\`, \`sh:and\`, \`sh:or\`, \`sh:xone\`,
\`sh:qualifiedValueShape\`, \`sh:targetWhere\`). v1 runs **Tarjan SCC** on this graph at ingestion; any
non-trivial SCC (or a self-loop) means recursion → REQ-REC-1 diagnostic. Same \`closure\`/graph
toolkit as §4.1. This is decidable and total — no semantics needed to *detect* recursion, only to
*evaluate* it.

### 9.2 Formal semantics for L3 (deferred)
When L3 is built, validation is an **assignment** \`σ: (shape × node) → {true, false}\` (arXiv
2108.13063). Non-recursive SHACL has a unique \`σ\`; recursive SHACL does not, hence the four candidate
semantics. ADR-002 selects **supported (Y-stratified)**: the assignment is the least fixpoint of the
"one validation round" operator over the truth lattice, computed by Kleene iteration to a fixed point
— structurally the same monotone-operator-on-finite-lattice argument as §4.1 path closure, lifted
from a node relation to a shape-node assignment. Proof obligation (L3): the iterate stabilises (finite
lattice, monotone round operator) and matches the reference processor on the recursive test cases.
**Not in v1 scope** — REQ-REC-1 rejects these inputs until then.

---

## 10. Conformance matrix & test-suite harness
Generated \`REQ-ID | Kw | Level | W3C § | Tests[] | Status\`; empty \`Tests[]\` = flagged gap. Tooling
re-verifies § from the live WD on each refresh (ADR-001). *(Matrix generated once the requirement set
is complete.)*

### 10.1 W3C 1.2 test-suite contract (the \`shacl-testsuite\` crate)
Source: \`w3c/data-shapes\` repo, \`gh-pages\` branch, \`data-shapes-test-suite/tests/\`. Tests are grouped
\`tests/core/{node,property,complex,path,targets,…}/\` and \`tests/sparql/…\`; each test is an individual
Turtle file (e.g. \`tests/core/node/datatype-001.ttl\`, \`datatype-002.ttl\`) referenced from a
\`manifest.ttl\` per directory.

> **REQ-TS-1** — MUST — The harness loads each directory \`manifest.ttl\` (\`mf:Manifest\` with an ordered
> \`mf:entries\` list) and executes every entry. — test-suite manifest format
> **REQ-TS-2** — MUST — \`sht:Validate\` entries: validate \`mf:action\`'s \`sht:dataGraph\` against
> \`sht:shapesGraph\` (which may be the same file) and compare the produced report to \`mf:result\`'s
> \`sh:ValidationReport\`. Comparison is **graph-isomorphic on results** (focusNode, resultPath, value,
> sourceConstraintComponent, sourceShape, severity), not literal triple equality — blank-node result
> nodes and result ordering must not matter. — manifest format
> **REQ-TS-3** — MUST — Node/shape-focus entries (\`sht:focus\` + \`sht:shape\`, \`mf:result true|false\`)
> exercise direct-focus validation (REQ-TGT-7) — validate one node against one shape, assert conforms.
> — manifest format
> **REQ-TS-4** — MUST — Shapes-graph well-formedness entries assert ill-formed detection (REQ-ING-5/6,
> REQ-DATATYPE-3, REQ-NODEKIND-2/3, etc.). — manifest format
> **REQ-TS-5** — SHOULD — Emit an implementation report (per-entry pass/fail/error) and, on failure,
> the diff between produced and expected results — this *is* the conformance evidence for §1.2 levels.
> **REQ-TS-6** — SHOULD — A second oracle: differential runner against **pySHACL** for the
> under-specified areas (recursion L3, SHACL-SPARQL binding edges, datatype lexical corners) recorded
> as \`Tests[diff:pyshacl]\`. — methodology

**Placeholder honesty:** the \`Tests[]\` cells throughout §3–§9 currently use **pattern style**
(\`core/node/datatype-*\`). A one-time tooling pass (build step 10) must walk the cloned manifests and
replace each with the **actual entry IRIs/filenames**, turning empty/glob cells into real gaps or real
links. Until then, treat \`Tests[]\` as *intended coverage*, not verified coverage.

---

## 11. Implementation plan (crate layout, traits, build order)

This section turns the spec into a buildable skeleton. It is normative for *structure*; the REQ
tables remain normative for *behavior*.

### 11.1 Workspace layout
\`\`\`
shacl-rs/                      (cargo workspace)
  shacl-model/    REQ-ING-*, REQ-TERM-*  shape/path AST, term re-export. dep: oxrdf(rdf-12)
  shacl-core/     REQ-PATH/TGT/RPT/CMP(§7)  engine generic over RdfGraph. dep: shacl-model,
                                            fancy-regex, oxsdatatypes.  NO oxigraph (REQ-ARCH-1)
  shacl-sparql/   REQ-SPQ-*               generic over SparqlGraph. dep: shacl-core
  shacl-oxigraph/ —                       impl traits for oxigraph::Store (feature-gated). dep: oxigraph
  shacl-testsuite/ §10                    W3C 1.2 manifest runner + diff harness (pySHACL oracle)
\`\`\`
The compiler enforces ADR-003/REQ-ARCH-1: \`shacl-core\` not depending on \`oxigraph\` is checked by
\`cargo tree\`. \`oxrdf\` is the only shared term dependency (ADR-004).

### 11.2 The two seam traits (ADR-003)
\`\`\`rust
pub trait RdfGraph {
    type Iter<'a>: Iterator<Item = Triple> where Self: 'a;
    fn triples(&self, s: Option<&Term>, p: Option<&NamedNode>, o: Option<&Term>) -> Self::Iter<'_>;
    /// default via triples(); SPARQL/remote backends override (ADR-003).
    fn reach(&self, start: &Term, path: &Path) -> NodeSet { /* fixpoint over triples() */ }
}
pub trait SparqlGraph: RdfGraph {
    fn select(&self, query: &str, prebound: &Bindings) -> Result<Solutions, EngineError>;
    fn ask(&self, query: &str, prebound: &Bindings) -> Result<bool, EngineError>;
}
\`\`\`

### 11.3 The validator seam (every §7 component implements this)
\`\`\`rust
pub struct Ctx<'a, G: RdfGraph> { graph: &'a G, focus: &'a Term, shape: &'a Shape, /* … */ }
pub trait Validator<G: RdfGraph> {
    /// value nodes already computed per §5; push one Result per violation.
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<G>, out: &mut Vec<ValidationResult>);
    fn component_iri(&self) -> NamedNodeRef;   // sh:…ConstraintComponent
}
\`\`\`
Component packets in §7 map 1:1 to \`Validator\` impls; the packet's "Report result" line dictates how
\`ValidationResult\` is populated, the REQ rows dictate the violation predicate.

### 11.4 Shared helpers (build before components)
- \`closure\`: generic least-fixpoint reachability — backs path \`*\`/\`+\` (REQ-PATH-5), SHACL-subclass
  (REQ-CLASS-2), recursion cycle detection (ADR-002). **Write & property-test first.**
- \`is_shacl_instance(node, class)\`: REQ-CLASS-2, REQ-TGT-2, implicit targets, qualified shapes.
- \`value_nodes(shape, focus)\`: §5 / REQ-RPT-1 — path eval entry point.
- \`report\`: \`ValidationResult\`/\`ValidationReport\` builders → RDF (REQ-RPT-2/3).

### 11.5 Build order (each step = green tests before next)
1. **shacl-model** + \`oxrdf\` re-export; parse Turtle 1.2 shapes graph via \`oxttl\` (REQ-ING-*, ADR-009).
2. **closure helper** + property tests (paths-as-fixpoint, the provable core).
3. **shacl-oxigraph** in-memory \`RdfGraph\` impl (so core has a real backend to test against).
4. **value nodes / paths** (§4, §5) over RdfGraph; pass \`path/*\` suite.
5. **report builders** (§6.7).
6. **CMP-NODEKIND** first (no graph access) — wires validator dispatch + report end-to-end.
7. **CMP-CLASS, CMP-DATATYPE** (exercise is_shacl_instance + oxsdatatypes).
8. Remaining §7 groups: cardinality → range → string → pair → logical → shape → list → other.
   Logical/shape (§7.7–7.8) need conformance-checking recursion guard (ADR-002) — gate after SCC detect.
9. **shacl-sparql** (§8): prefixes → constraints → components → prebinding seam (ADR-008).
10. **conformance matrix** generation (§10) + CI gate on the W3C 1.2 suite.

### 11.6 Definition of "ready to implement" (this milestone)
✓ Normative target pinned (1.2 Core + SPARQL ext), real section numbers.
✓ All ADRs decided (001–010). ✓ Seam + validator traits sketched. ✓ Build order with test gates.
✗ Remaining before code: pin exact crate versions (oxigraph 0.5.x, fancy-regex, oxsdatatypes);
clone the W3C 1.2 manifests and run the §10.1 tooling pass to replace pattern-style \`Tests[]\` placeholders with real entry filenames;
expand §7.2–7.9 component packets (template + §7.1 examples make this mechanical).

## Appendix A — Decisions (ADRs)

- **ADR-001 — Target SHACL 1.2 (WD 2026-04-10), accept draft instability.** Building greenfield
  against the forward spec. Section anchors are snapshots; matrix tooling re-verifies per refresh.
  SHACL-C is informative/unstable → out of scope.
- **ADR-002 — Recursion semantics. DECIDED (revisitable).** v1 ships L1/L2 with **reject-on-recursion**
  (REQ-REC-1): detect a cycle in the shape-reference graph (shapes linked via \`sh:node\`/\`sh:property\`/
  \`sh:not\`/\`sh:and\`/\`sh:or\`/\`sh:xone\`/\`sh:qualifiedValueShape\`) during shapes-graph ingestion and
  emit a diagnostic; do not validate. Rationale: the spec (§6.5.3) leaves recursion undefined and
  explicitly sanctions non-support; rejecting is conformant and avoids committing to a semantics the
  WG may later standardize. L3 (deferred past v1): adopt the **supported/Y-stratified** semantics from
  arXiv 2108.13063 since it composes cleanly with the conformance-checking model already used by
  \`sh:not\`/\`sh:or\`. Cycle detection is Tarjan SCC over the shape-ref graph — same closure toolkit as
  paths/subclass.
- **ADR-003 — Backend seam \`RdfGraph\`/\`SparqlGraph\`** + optional \`reach()\` closure-pushdown override.
- **ADR-004 — Term model: RE-EXPORT \`oxrdf\` with the \`rdf-12\` feature. RESOLVED.** \`oxrdf\`'s
  \`Term\`/\`Triple\` types support RDF 1.2 triple terms behind \`rdf-12\`; SHACL 1.2's \`{| … |}\`
  annotation (§3.1.4–6) desugars to the RDF 1.2 reifier model (a triple with predicate \`rdf:reifies\`
  whose object is a triple term; subject = reifier). Re-exporting satisfies REQ-TERM-1 without a local
  model and insulates us from the AT-RISK \`rdf:TripleTerm\`/\`rdf:tt*\` vocabulary churn (we touch it
  only via the typed API, never by IRI string-match). *Open sub-item → ADR-009: confirm the Turtle
  parser (\`oxttl\`) accepts Turtle 1.2 annotation syntax \`{| |}\`; the model must resolve a reifier
  back to the constraint triple it annotates.*
- **ADR-005 — Regex engine = \`fancy-regex\`. DECIDED.** \`sh:pattern\` (REQ-PATTERN-1/2) requires
  XPath/SPARQL \`REGEX\` semantics; the Rust \`regex\` crate omits backreferences and lookaround and uses
  a different flag set, so it would mis-validate conformant patterns. Use \`fancy-regex\` (superset,
  backtracking) and map SHACL \`sh:flags\` (\`i\`,\`s\`,\`m\`,\`x\`,\`q\`) to its options. Accept the perf
  trade-off (backtracking) since pattern constraints are not hot-path. Differential-test against the
  reference processor on the suite's pattern cases; document any residual XPath-vs-PCRE divergence.
- **ADR-006 — SPARQL query assembly** (prefix splicing, escaping, injection method).
- **ADR-007 — \`sh:targetWhere\` strategy. DECIDED (revisitable).** Implement via **naive iteration**:
  enumerate candidate nodes (subjects+objects of the data graph) and keep those that conform to the
  inner shape (REQ-TGT-5). Keeps where-targets in **L1** (no SPARQL forced) and is correct by
  construction. Mitigations: (a) restrict candidates to nodes appearing in the data graph, not the
  infinite term space; (b) if the inner shape has a \`sh:class\`/\`sh:targetClass\`-like discriminator,
  pre-filter by it; (c) memoise conformance. A SPARQL-pushdown fast path is a future optimization
  behind the same \`reach\`/target API, not a v1 requirement. Spec (§3.1.3.6) itself warns this is
  worst-case O(graph) and implementation-dependent, so naive-but-correct is acceptable for v1.
- **ADR-008 — Isolate pre-binding behind an abstraction (Feature-at-Risk).** SHACL-SPARQL pre-binding
  (spec App. A, Issue 647) is defined as algebra Values-Insertion and may change to track SPARQL 1.2.
  Implement it as a single \`prebind(query, bindings)\` seam so a future redefinition is a one-module
  change (REQ-SPQ-15/16). *Pending.*
- **ADR-009 — Turtle 1.2 annotation parsing. RESOLVED.** \`oxttl\` (oxigraph 0.5.x, \`rdf-12\`
  feature) parses RDF 1.2 reifier/annotation syntax — the changelog explicitly fixes \`{| … |}\`
  handling, and \`rdf-12\` replaced the old \`rdf-star\`. Two design facts: (a) triple terms may **not**
  appear in subject position in RDF 1.2 — the SHACL reifier (not the triple term) is the attach point
  for \`sh:severity\`/\`sh:message\`/\`sh:deactivated\`, which fits; (b) pin a specific 0.5.x and track the
  changelog (the crate is pre-1.0 and moving), consistent with ADR-001. Model links reifier→annotated
  triple via the \`rdf:reifies\` triple whose object is the constraint triple term.
- **ADR-011 — Stable Dependencies Principle is a build invariant.** Dependencies (crate *and*
  module level) point toward stability: I = Ce/(Ca+Ce) must be non-increasing along every edge.
  Crate gradient is 0.00(model)→0.25(core)→0.50(sparql)→0.75(oxigraph)→1.00(testsuite). Critically,
  the \`RdfGraph\`/\`SparqlGraph\` trait module must keep **Ce=0** (depend on no concrete module): the
  path-evaluation *policy* lives in a free function \`path::reach()\`, and native pushdown is a
  separate optional \`PathReach\` trait chosen by the backend layer — so the stable abstraction never
  depends on a volatile algorithm (also satisfies DIP). New components (§7) depend on the traits +
  \`report\` + \`closure\` (all I=0.00), never the reverse. A CI \`cargo tree\` check already enforces the
  headline REQ-ARCH-1 edge; the per-edge I-gradient is checked by a small script in build step 10.
- **ADR-010 — Datatype lexical validation backed by \`oxsdatatypes\`.** REQ-DATATYPE-2 requires
  XSD lexical-space validity (matching datatype IRI is insufficient). Use oxigraph's
  \`oxsdatatypes\` crate for XSD lexical/value spaces rather than per-type regex. Confirms term-layer
  reuse consistent with ADR-004. *Pending — confirm crate covers needed types.*

## Appendix B — Glossary
*(stub — focus node, value node, shapes/data graph, SHACL instance/subclass/type, conforms, failure,
reifier, triple term.)*

## Appendix C — Out of scope for v1
SHACL-C compact syntax (unstable); Node Expressions beyond constants (separate 1.2 spec); Rules;
UI; Profiling; entailment regimes (\`sh:entailment\` → MAY; default = signal failure if unsupported).

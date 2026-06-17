# Contributing to shacl-rs

Thanks for your interest. This is a focused SHACL 1.2 validator; contributions that improve
conformance, fix bugs, or sharpen the API are very welcome.

## Ground rules

The behavioural specification is [`shacl-rs-functional-spec.md`](shacl-rs-functional-spec.md) —
numbered requirements traced to the W3C SHACL 1.2 drafts and test suite. It is the source of truth;
when in doubt, follow the spec and cite the requirement id (e.g. `REQ-DATATYPE-2`).

Two architectural invariants are enforced in CI and must hold:

- **`REQ-ARCH-1`** — `shacl-core` must not depend on `oxigraph` (or any SPARQL engine). Everything is
  expressed against the `RdfGraph` / `SparqlGraph` traits; concrete backends live in `shacl-oxigraph`.
- **Stable Dependencies (ADR-011)** — crate dependencies only point "down" the stability gradient
  (`shacl-model` ← `shacl-core` ← `shacl-sparql` ← `shacl-oxigraph` ← `shacl-testsuite`).

## Before you open a PR

All four gates must pass (CI runs the same):

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test  --workspace
cargo deny  check            # advisories, licenses, bans, sources
```

- **MSRV is Rust 1.87** (pinned in `rust-toolchain.toml`). Don't use newer-than-1.87 std APIs.
- **No `unwrap`/`expect`/`panic!`/indexing panics in library code** — use `?`, `ok_or`,
  `unwrap_or_default`, `match`, or `.get(..)`. The crates are consumed by zero-panic downstreams.
- **Add a test** for any behaviour change. Component work belongs in
  `shacl-oxigraph/tests/` (table-driven over `MemGraph`); conformance fixtures live in
  `shacl-testsuite/tests/fixtures/`.
- **Run the W3C suite** if you touch the engine:
  `cargo run -p shacl-testsuite -- <path-to>/data-shapes/shacl12-test-suite/tests/core`.
  Don't regress the pass count (currently 138/141).

## Adding a constraint component

1. Implement the `Validator` in the right `shacl-core/src/constraints/` module.
2. Add a `dispatch` arm in `constraints/mod.rs`.
3. If it has a new parameter, teach `shacl-oxigraph/src/ingest.rs` to parse it.
4. Add tests; update `CHANGELOG.md` under "Unreleased".

## Commits & PRs

- Keep commits focused; a clear imperative subject line plus a short body explaining *why*.
- Reference the spec requirement or the W3C test the change addresses where relevant.
- Update `CHANGELOG.md` for user-visible changes.

## License of contributions

By contributing, you agree that your contributions are dual-licensed under the
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE) licenses, as described in the README, without any
additional terms.

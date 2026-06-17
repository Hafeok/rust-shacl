# Security Policy

## Supported versions

This project is pre-1.0 and moves on the `main` line. Security fixes land on `main` and in the next
tagged release; only the latest `0.x` release is supported.

| Version | Supported |
|---------|-----------|
| latest `0.x` (and `main`) | ✅ |
| older tags | ❌ |

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report privately via GitHub's
[**Report a vulnerability**](https://github.com/Hafeok/rust-shacl/security/advisories/new) button
(Security → Advisories). This opens a private advisory visible only to the maintainers.

When reporting, please include:

- the affected crate and version (or commit),
- a description of the issue and its impact,
- a minimal reproduction (e.g. a shapes/data graph that triggers it) where possible.

You can expect an acknowledgement within a few days. Fixes are coordinated through a private advisory
and released before public disclosure.

## Scope

`shacl-rs` is a validation library; the most relevant concerns are denial-of-service via untrusted
input (pathological shapes graphs, data graphs, or `sh:sparql` queries) and incorrect validation
verdicts. Path/recursion evaluation is bounded (least-fixpoint closure + a recursion guard with a
depth backstop), so please flag any input that causes non-termination, excessive memory, or a panic.

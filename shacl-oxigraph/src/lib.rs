//! `shacl-oxigraph` — the only crate depending on `oxigraph`. Provides:
//! - [`mem::MemGraph`]: an in-memory [`shacl_core::RdfGraph`] for tests (build step 3, §11.5).
//! - (later) an `oxigraph::Store` adapter implementing `RdfGraph` + `SparqlGraph`.
//! - (later) Turtle 1.2 shapes-graph ingestion via `oxttl` (ADR-009).

pub mod mem;

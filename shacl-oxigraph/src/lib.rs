//! `shacl-oxigraph` — the only crate depending on `oxigraph`. Provides:
//! - [`mem::MemGraph`]: an in-memory [`shacl_core::RdfGraph`] for tests (build step 3, §11.5).
//! - [`ingest`]: Turtle 1.2 shapes-graph + data-graph parsing via `oxttl` (ADR-009, `REQ-ING-*`).
//! - (later) an `oxigraph::Store` adapter implementing `RdfGraph` + `SparqlGraph`.

pub mod ingest;
pub mod mem;

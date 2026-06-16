//! `shacl-core` — the SHACL 1.2 Core validation engine, generic over a graph backend.
//!
//! `REQ-ARCH-1`: this crate has **no** dependency on `oxigraph` or any SPARQL engine. The
//! compiler enforces it (`cargo tree -p shacl-core` shows no `oxigraph`). Everything here is
//! expressed against the [`RdfGraph`] trait (§11.2).
//!
//! Build order (§11.5): [`closure`] (step 2, property-tested) → [`path`] (step 4) → [`report`]
//! (step 5) → [`constraints`] (steps 6–8).

pub mod closure;
pub mod constraints;
pub mod engine;
pub mod graph;
pub mod path;
pub mod report;
pub mod validator;
pub mod values;

pub use engine::{focus_nodes, validate, validate_focus};
pub use graph::{Bindings, EngineError, NodeSet, PathReach, RdfGraph, Solutions, SparqlGraph};
pub use path::reach;
pub use report::{ValidationReport, ValidationResult};
pub use validator::{Ctx, Validator};
pub use values::value_nodes;

//! `shacl-model` — SHACL 1.2 shape and path abstract syntax, plus the RDF term layer.
//!
//! Spec references: shapes graph ingestion (§3, `REQ-ING-*`), property-path AST (§4,
//! `REQ-PATH-1..6`), RDF 1.2 term/reifier model (`REQ-TERM-1`, ADR-004).
//!
//! This crate has no backend dependency. Terms are re-exported from `oxrdf` with the `rdf-12`
//! feature so that triple terms and reifiers (needed for SHACL 1.2 `{| … |}` annotation of
//! severity/message/deactivation, §3.1.4–6) are available without a hand-rolled model.

pub mod path;
pub mod shape;
pub mod target;
pub mod term;

pub use path::Path;
pub use shape::{NodeShape, PropertyShape, Shape, ShapeId};
pub use target::Target;
pub use term::{BlankNode, Literal, NamedNode, NamedNodeRef, Term, Triple};

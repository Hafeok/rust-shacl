//! `shacl-sparql` — SHACL 1.2 SPARQL Extensions (§8, `REQ-SPQ-*`). Generic over
//! [`shacl_core::SparqlGraph`]; contains no concrete SPARQL engine.
//!
//! Build order (§11.5 step 9): prefixes → constraints (§8.1) → constraint components (§8.2) →
//! pre-binding seam (§8.4, ADR-008 — isolate behind one function as it is Feature-at-Risk).

#![allow(missing_docs)] // stubs below; fill from §8 packets.

pub mod prefixes; // REQ-SPQ-13: sh:prefixes/owl:imports*/sh:declare collection
pub mod constraint; // §8.1 sh:sparql / SELECT constraints
pub mod component; // §8.2 SPARQL-based constraint components (SELECT/ASK validators)
pub mod prebind; // §8.4 Values-Insertion pre-binding (Feature-at-Risk, ADR-008)

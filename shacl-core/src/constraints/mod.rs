//! Core constraint components (§7). Each submodule implements one component group; the
//! [`registry`] maps a `sh:…ConstraintComponent` IRI to its [`crate::validator::Validator`].
//!
//! Build order within §7 (§11.5 step 6→8): value_type (nodeKind first) → cardinality → range →
//! string → pair → logical → shape → list → other. Only value_type is sketched here; the rest are
//! module stubs to be filled from their §7 packets.

pub mod value_type;
// pub mod cardinality;   // §7.2 — CMP-MINCOUNT (worked in spec), CMP-MAXCOUNT
// pub mod range;         // §7.3
// pub mod string;        // §7.4 — CMP-PATTERN (worked in spec)
// pub mod pair;          // §7.6
// pub mod logical;       // §7.7 — needs recursion guard (ADR-002) before enabling
// pub mod shape;         // §7.8
// pub mod list;          // §7.5
// pub mod other;         // §7.9

pub mod helpers;

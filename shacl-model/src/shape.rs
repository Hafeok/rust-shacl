//! Shape model (§3, `REQ-ING-*`). A shapes graph parses into a set of [`Shape`]s; node vs property
//! shapes are disjoint (`REQ-ING-2`). Constraints are stored as a component-keyed bag so repeated
//! single-parameter components (e.g. two `sh:class`) become independent conjunctive constraints
//! (`REQ-ING-4`). Parsing/ingestion itself lives in `shacl-oxigraph` (needs a parser); this crate
//! defines the parsed shape representation the engine consumes.

use crate::path::Path;
use crate::term::{NamedNode, Term};

/// Identifier of a shape in the shapes graph (IRI or blank node).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapeId {
    /// An IRI-named shape (recommended; allows external reference/deactivation, §3.1.6).
    Named(NamedNode),
    /// A blank-node (inline) shape.
    Blank(String),
}

/// Severity of a shape or constraint (§3.1.4). Default is `Violation` (`REQ-ING-7`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Severity {
    /// `sh:Trace`
    Trace,
    /// `sh:Debug`
    Debug,
    /// `sh:Info`
    Info,
    /// `sh:Warning`
    Warning,
    /// `sh:Violation` (default)
    #[default]
    Violation,
}

/// A SHACL shape: either a node shape or a property shape (`REQ-ING-2`, disjoint sets).
#[derive(Debug, Clone)]
pub enum Shape {
    /// Constraints apply to the focus node itself (§3.2).
    Node(NodeShape),
    /// Constraints apply to value nodes reached via `sh:path` (§3.3).
    Property(PropertyShape),
}

/// A node shape (§3.2): no `sh:path`.
#[derive(Debug, Clone)]
pub struct NodeShape {
    /// Shape identity.
    pub id: ShapeId,
    /// Target declarations (§3.1.3).
    pub targets: Vec<crate::target::Target>,
    /// Declared constraints (component IRI + parameter values), pre-grouped per `REQ-ING-4`.
    pub constraints: Vec<Constraint>,
    /// Effective severity (§3.1.4).
    pub severity: Severity,
    /// `sh:deactivated` resolved to a constant in Core (`REQ-ING-10`).
    pub deactivated: bool,
}

/// A property shape (§3.3): exactly one `sh:path`.
#[derive(Debug, Clone)]
pub struct PropertyShape {
    /// Shape identity.
    pub id: ShapeId,
    /// The path whose value nodes are validated (§4, §6.8).
    pub path: Path,
    /// Target declarations (a property shape may carry targets too).
    pub targets: Vec<crate::target::Target>,
    /// Declared constraints.
    pub constraints: Vec<Constraint>,
    /// Effective severity.
    pub severity: Severity,
    /// `sh:deactivated`.
    pub deactivated: bool,
}

/// One declared constraint: a component plus its parameter values (§3.1.1). The engine dispatches
/// on `component` to the matching `Validator` (§11.3). Per-constraint severity/message/deactivation
/// (the RDF 1.2 reifier annotations, §3.1.4–6) ride along here.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// The `sh:…ConstraintComponent` IRI.
    pub component: NamedNode,
    /// Parameter (predicate → value) pairs for this constraint instance.
    pub params: Vec<(NamedNode, Term)>,
    /// Per-constraint severity override (reifier annotation), else inherits the shape's.
    pub severity: Option<Severity>,
    /// Per-constraint deactivation (reifier annotation).
    pub deactivated: bool,
}

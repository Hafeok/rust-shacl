//! Core constraint components (§7). Each submodule implements one component group; the
//! [`registry`] maps a `sh:…ConstraintComponent` IRI to its [`crate::validator::Validator`].
//!
//! Build order within §7 (§11.5 step 6→8): value_type (nodeKind first) → cardinality → range →
//! string → pair → logical → shape → list → other. Only value_type is sketched here; the rest are
//! module stubs to be filled from their §7 packets.

pub mod cardinality; // §7.2 — CMP-MINCOUNT (worked in spec), CMP-MAXCOUNT
pub mod helpers;
pub mod list; // §7.5 — CMP-LISTLEN-*, CMP-UNIQUEMEMBERS (sh:memberShape waits for the 9b guard)
pub mod membership; // §7.9 — CMP-HASVALUE, CMP-IN
pub mod other; // §7.9 — CMP-CLOSED (rootClass/uniqueValuesFor are documented gaps)
pub mod pair; // §7.6 — CMP-PAIR-*, CMP-SUBSETOF
pub mod range; // §7.3 — CMP-RANGE-*
pub mod shape; // §7.7/7.8 + §7.5.1 — logical, sh:node/property/qualified, sh:memberShape
pub mod string; // §7.4 — CMP-LENGTH-*, CMP-PATTERN, CMP-SINGLELINE, CMP-LANGUAGEIN, CMP-UNIQUELANG
pub mod value_type; // §7.1 — CMP-NODEKIND, CMP-CLASS, CMP-DATATYPE

use crate::engine::term_to_shape_id;
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use shacl_model::shape::{Constraint, ShapeId};
use shacl_model::term::{NamedNode, NodeKind, Term};

/// SHACL namespace.
const SH: &str = "http://www.w3.org/ns/shacl#";

/// Build a constraint-component IRI `sh:<name>` (e.g. `sh:MinCountConstraintComponent`). Shared by
/// every component's `Validator` to stamp `sh:sourceConstraintComponent` on its results.
#[must_use]
pub(crate) fn comp(name: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{SH}{name}"))
}

/// Construct a [`ValidationResult`] from the validation context (§6.7.2): `sh:focusNode`,
/// `sh:resultPath` (property shapes), the offending `value` where applicable (`None` for
/// count-based violations like `sh:minCount`), the `component` IRI, `sh:sourceShape`, and the
/// effective severity. `sh:resultMessage` is filled from `sh:message` by the engine (`REQ-ING-9`).
#[must_use]
pub(crate) fn result_for(
    ctx: &Ctx<'_, impl RdfGraph>,
    value: Option<Term>,
    component: NamedNode,
) -> ValidationResult {
    ValidationResult {
        focus_node: ctx.focus.clone(),
        result_path: ctx.path_sparql.clone(),
        value,
        source_constraint_component: component,
        source_shape: ctx.shape.id().clone(),
        severity: ctx.severity,
        messages: Vec::new(),
    }
}

/// Build the [`Validator`]s for one declared constraint (the §7 dispatch table).
///
/// Returns possibly *several* validators when a single-parameter component repeats (independent
/// conjunctive constraints, `REQ-ING-4`/`REQ-CLASS-4`), exactly one for a well-formed
/// single-valued component, or none when the component IRI is unknown (the constraint is ignored,
/// per the open-world dispatch) or the parameter is ill-formed. Adding a component means adding one
/// arm here plus its `Validator` impl — nothing else in the engine changes.
#[must_use]
pub fn dispatch<G: RdfGraph>(c: &Constraint) -> Vec<Box<dyn Validator<G>>> {
    let comp = c.component.as_str();
    match comp.strip_prefix(SH).unwrap_or(comp) {
        // §7.1.3 — sh:nodeKind. One IRI, or a 1.2 list (disjunction) → one validator over the set.
        "NodeKindConstraintComponent" => {
            let kinds: Vec<NodeKind> = param_iris(c, "nodeKind")
                .iter()
                .filter_map(NodeKind::from_iri)
                .collect();
            if kinds.is_empty() {
                Vec::new()
            } else {
                vec![Box::new(value_type::NodeKindValidator { kinds }) as Box<dyn Validator<G>>]
            }
        }
        // §7.1.1 — sh:class. Repeats / list members are independent conjuncts (REQ-CLASS-4) → one
        // validator per value.
        "ClassConstraintComponent" => param_iris(c, "class")
            .into_iter()
            .map(|class| Box::new(value_type::ClassValidator { class }) as Box<dyn Validator<G>>)
            .collect(),
        // §7.1.2 — sh:datatype. One IRI, or a 1.2 list (disjunction) → one validator over the set.
        "DatatypeConstraintComponent" => {
            let datatypes = param_iris(c, "datatype");
            if datatypes.is_empty() {
                Vec::new()
            } else {
                vec![Box::new(value_type::DatatypeValidator { datatypes }) as Box<dyn Validator<G>>]
            }
        }
        // §7.2.1 — sh:minCount. Exactly one integer (REQ-MINCOUNT), property shapes only.
        "MinCountConstraintComponent" => param_int(c, "minCount")
            .map(|min| Box::new(cardinality::MinCountValidator { min }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        // §7.2.2 — sh:maxCount. Exactly one integer (REQ-MAXCOUNT), property shapes only.
        "MaxCountConstraintComponent" => param_int(c, "maxCount")
            .map(|max| Box::new(cardinality::MaxCountValidator { max }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),

        // §7.3 — value range. Exactly one threshold literal each.
        "MinExclusiveConstraintComponent" => {
            range_validator(c, range::Bound::MinExclusive, "minExclusive")
        }
        "MinInclusiveConstraintComponent" => {
            range_validator(c, range::Bound::MinInclusive, "minInclusive")
        }
        "MaxExclusiveConstraintComponent" => {
            range_validator(c, range::Bound::MaxExclusive, "maxExclusive")
        }
        "MaxInclusiveConstraintComponent" => {
            range_validator(c, range::Bound::MaxInclusive, "maxInclusive")
        }

        // §7.4 — string components.
        "MinLengthConstraintComponent" => param_int(c, "minLength")
            .map(|min| Box::new(string::MinLengthValidator { min }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        "MaxLengthConstraintComponent" => param_int(c, "maxLength")
            .map(|max| Box::new(string::MaxLengthValidator { max }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        // sh:pattern (+ optional sh:flags). REQ-PATTERN-4: >1 pattern/flags is ill-formed — we take
        // the first of each.
        "PatternConstraintComponent" => match param_term(c, "pattern") {
            Some(Term::Literal(p)) => {
                let flags = match param_term(c, "flags") {
                    Some(Term::Literal(f)) => Some(f.value().to_string()),
                    _ => None,
                };
                vec![
                    Box::new(string::PatternValidator::new(p.value(), flags.as_deref()))
                        as Box<dyn Validator<G>>,
                ]
            }
            _ => Vec::new(),
        },
        "SingleLineConstraintComponent" => match param_bool(c, "singleLine") {
            Some(true) => vec![Box::new(string::SingleLineValidator) as Box<dyn Validator<G>>],
            _ => Vec::new(),
        },
        "LanguageInConstraintComponent" => {
            // An empty sh:languageIn admits no language tag → every value node violates.
            let ranges: Vec<String> = param_terms(c, "languageIn")
                .into_iter()
                .filter_map(|t| match t {
                    Term::Literal(l) => Some(l.value().to_ascii_lowercase()),
                    _ => None,
                })
                .collect();
            vec![Box::new(string::LanguageInValidator { ranges }) as Box<dyn Validator<G>>]
        }
        "UniqueLangConstraintComponent" => match param_bool(c, "uniqueLang") {
            Some(true) => vec![Box::new(string::UniqueLangValidator) as Box<dyn Validator<G>>],
            _ => Vec::new(),
        },

        // §7.9 — value membership.
        "HasValueConstraintComponent" => param_term(c, "hasValue")
            .map(|value| Box::new(membership::HasValueValidator { value }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        "InConstraintComponent" => {
            // An empty sh:in is the empty set → every value node violates.
            let members = param_terms(c, "in");
            vec![Box::new(membership::InValidator { members }) as Box<dyn Validator<G>>]
        }

        // §7.6 — property pair (each takes one predicate IRI; property shapes only).
        "EqualsConstraintComponent" => param_iris(c, "equals")
            .into_iter()
            .map(|predicate| Box::new(pair::EqualsValidator { predicate }) as Box<dyn Validator<G>>)
            .collect(),
        "DisjointConstraintComponent" => param_iris(c, "disjoint")
            .into_iter()
            .map(|predicate| {
                Box::new(pair::DisjointValidator { predicate }) as Box<dyn Validator<G>>
            })
            .collect(),
        "SubsetOfConstraintComponent" => param_iris(c, "subsetOf")
            .into_iter()
            .map(|predicate| {
                Box::new(pair::SubsetOfValidator { predicate }) as Box<dyn Validator<G>>
            })
            .collect(),
        "LessThanConstraintComponent" => param_iris(c, "lessThan")
            .into_iter()
            .map(|predicate| {
                Box::new(pair::LessThanValidator {
                    predicate,
                    or_equals: false,
                }) as Box<dyn Validator<G>>
            })
            .collect(),
        "LessThanOrEqualsConstraintComponent" => param_iris(c, "lessThanOrEquals")
            .into_iter()
            .map(|predicate| {
                Box::new(pair::LessThanValidator {
                    predicate,
                    or_equals: true,
                }) as Box<dyn Validator<G>>
            })
            .collect(),

        // §7.5 — rdf:List components (sh:memberShape waits for the recursion guard, 9b).
        "MinListLengthConstraintComponent" => param_int(c, "minListLength")
            .map(|bound| {
                Box::new(list::ListLengthValidator {
                    bound,
                    is_min: true,
                }) as Box<dyn Validator<G>>
            })
            .into_iter()
            .collect(),
        "MaxListLengthConstraintComponent" => param_int(c, "maxListLength")
            .map(|bound| {
                Box::new(list::ListLengthValidator {
                    bound,
                    is_min: false,
                }) as Box<dyn Validator<G>>
            })
            .into_iter()
            .collect(),
        "UniqueMembersConstraintComponent" => match param_bool(c, "uniqueMembers") {
            Some(true) => vec![Box::new(list::UniqueMembersValidator) as Box<dyn Validator<G>>],
            _ => Vec::new(),
        },

        // §7.7 — logical (operands are shape references).
        "NotConstraintComponent" => param_shape(c, "not")
            .map(|shape| Box::new(shape::NotValidator { shape }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),
        "AndConstraintComponent" => shape_list(c, "and", |shapes| shape::AndValidator { shapes }),
        "OrConstraintComponent" => shape_list(c, "or", |shapes| shape::OrValidator { shapes }),
        "XoneConstraintComponent" => {
            shape_list(c, "xone", |shapes| shape::XoneValidator { shapes })
        }

        // §7.8 — shape (sh:node summarises; sh:property bubbles). May repeat → one validator each.
        "NodeConstraintComponent" => param_shapes(c, "node")
            .into_iter()
            .map(|shape| Box::new(shape::NodeValidator { shape }) as Box<dyn Validator<G>>)
            .collect(),
        "PropertyConstraintComponent" => param_shapes(c, "property")
            .into_iter()
            .map(|shape| Box::new(shape::PropertyValidator { shape }) as Box<dyn Validator<G>>)
            .collect(),
        "QualifiedMinCountConstraintComponent" => qualified(c, true),
        "QualifiedMaxCountConstraintComponent" => qualified(c, false),

        // §7.5.1 — sh:memberShape (recurses into a shape; lives with the shape components).
        "MemberShapeConstraintComponent" => param_shape(c, "memberShape")
            .map(|shape| Box::new(shape::MemberShapeValidator { shape }) as Box<dyn Validator<G>>)
            .into_iter()
            .collect(),

        // §7.9.1 — sh:closed (+ sh:ignoredProperties).
        "ClosedConstraintComponent" => match param_bool(c, "closed") {
            Some(true) => {
                let ignored = param_iris(c, "ignoredProperties");
                vec![Box::new(other::ClosedValidator { ignored }) as Box<dyn Validator<G>>]
            }
            _ => Vec::new(),
        },

        _ => Vec::new(),
    }
}

/// Build a logical validator over the shape list bound to `sh:<local>` (`sh:and`/`sh:or`/`sh:xone`).
/// An empty list is kept (not dropped): empty `sh:and` conforms vacuously, empty `sh:or`/`sh:xone`
/// cannot be satisfied — all defined semantics the validators implement directly.
fn shape_list<G: RdfGraph, V: Validator<G> + 'static>(
    c: &Constraint,
    local: &str,
    make: impl FnOnce(Vec<ShapeId>) -> V,
) -> Vec<Box<dyn Validator<G>>> {
    vec![Box::new(make(param_shapes(c, local))) as Box<dyn Validator<G>>]
}

/// Build a `sh:qualifiedValueShape` count validator (`is_min` selects min vs max count).
fn qualified<G: RdfGraph>(c: &Constraint, is_min: bool) -> Vec<Box<dyn Validator<G>>> {
    let count_param = if is_min {
        "qualifiedMinCount"
    } else {
        "qualifiedMaxCount"
    };
    match (
        param_shape(c, "qualifiedValueShape"),
        param_int(c, count_param),
    ) {
        (Some(shape), Some(bound)) => {
            vec![Box::new(shape::QualifiedValidator {
                shape,
                bound,
                is_min,
            }) as Box<dyn Validator<G>>]
        }
        _ => Vec::new(),
    }
}

/// Build a single [`range::RangeValidator`] for one of the four `sh:*Inclusive`/`sh:*Exclusive`
/// components, reading its single threshold literal from `sh:<local>`.
fn range_validator<G: RdfGraph>(
    c: &Constraint,
    bound: range::Bound,
    local: &str,
) -> Vec<Box<dyn Validator<G>>> {
    param_term(c, local)
        .map(|limit| Box::new(range::RangeValidator { bound, limit }) as Box<dyn Validator<G>>)
        .into_iter()
        .collect()
}

/// The IRI values bound to parameter `sh:<local>` on a constraint, in declaration order.
fn param_iris(c: &Constraint, local: &str) -> Vec<NamedNode> {
    let pred = format!("{SH}{local}");
    c.params
        .iter()
        .filter(|(p, _)| p.as_str() == pred)
        .filter_map(|(_, v)| match v {
            Term::NamedNode(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

/// The first integer value bound to parameter `sh:<local>` (e.g. `sh:minCount`), parsed from its
/// literal lexical form. Single-valued integer parameters are exactly-one per shape (`REQ-ING-5`);
/// a missing or non-integer value yields `None`, so the component is silently skipped.
fn param_int(c: &Constraint, local: &str) -> Option<i64> {
    match param_term(c, local) {
        Some(Term::Literal(lit)) => lit.value().parse::<i64>().ok(),
        _ => None,
    }
}

/// The first value (of any term kind) bound to parameter `sh:<local>`, in declaration order.
/// Used for single-valued parameters whose value is an arbitrary term (e.g. `sh:hasValue`,
/// `sh:minInclusive`).
fn param_term(c: &Constraint, local: &str) -> Option<Term> {
    let pred = format!("{SH}{local}");
    c.params
        .iter()
        .find(|(p, _)| p.as_str() == pred)
        .map(|(_, v)| v.clone())
}

/// All values bound to parameter `sh:<local>`, in declaration order. List-valued parameters
/// (`sh:in`, `sh:languageIn`, `sh:and`/`sh:or`/`sh:xone`, `sh:ignoredProperties`) are represented
/// as **repeated** `(predicate, element)` pairs in list order — ingestion (Phase 10) flattens the
/// `rdf:List` into this shape, exactly as it does for repeated single-valued params like `sh:class`.
fn param_terms(c: &Constraint, local: &str) -> Vec<Term> {
    let pred = format!("{SH}{local}");
    c.params
        .iter()
        .filter(|(p, _)| p.as_str() == pred)
        .map(|(_, v)| v.clone())
        .collect()
}

/// The first boolean value bound to parameter `sh:<local>` (e.g. `sh:uniqueLang`, `sh:singleLine`),
/// read from an `xsd:boolean` literal (`"true"`/`"false"`, or `"1"`/`"0"`).
fn param_bool(c: &Constraint, local: &str) -> Option<bool> {
    match param_term(c, local) {
        Some(Term::Literal(lit)) => match lit.value() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// The first shape reference bound to parameter `sh:<local>` (e.g. `sh:not`, `sh:qualifiedValueShape`).
fn param_shape(c: &Constraint, local: &str) -> Option<ShapeId> {
    param_term(c, local).as_ref().and_then(term_to_shape_id)
}

/// All shape references bound to parameter `sh:<local>`, in order. Used for repeated single-valued
/// references (`sh:node`/`sh:property`) and for flattened shape lists (`sh:and`/`sh:or`/`sh:xone`).
fn param_shapes(c: &Constraint, local: &str) -> Vec<ShapeId> {
    param_terms(c, local)
        .iter()
        .filter_map(term_to_shape_id)
        .collect()
}

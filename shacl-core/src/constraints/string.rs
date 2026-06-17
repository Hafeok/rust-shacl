//! String-based constraint components (§7.4): `sh:minLength`/`sh:maxLength` (§7.4.1–2,
//! `CMP-LENGTH-*`), `sh:pattern`/`sh:flags` (§7.4.3, `CMP-PATTERN`), `sh:singleLine` (§7.4.4,
//! `CMP-SINGLELINE`, new in 1.2), `sh:languageIn` (§7.4.5, `CMP-LANGUAGEIN`), and `sh:uniqueLang`
//! (§7.4.6, `CMP-UNIQUELANG`).
//!
//! The "string form" of a value node is its lexical form (literals) or IRI string (IRIs); blank
//! nodes have no string form and therefore always violate length / pattern / singleLine
//! constraints.

use super::{comp, result_for};
use crate::graph::RdfGraph;
use crate::report::ValidationResult;
use crate::validator::{Ctx, Validator};
use fancy_regex::Regex;
use shacl_model::term::{NamedNodeRef, Term};

/// String form used by §7.4 components: lexical form for literals, IRI for IRIs, `None` for blanks.
fn string_form(t: &Term) -> Option<&str> {
    match t {
        Term::Literal(l) => Some(l.value()),
        Term::NamedNode(n) => Some(n.as_str()),
        _ => None, // blank node (and any RDF-1.2 triple term): no string form.
    }
}

/// The language tag of a value node, if it is a language-tagged literal.
fn language_of(t: &Term) -> Option<&str> {
    match t {
        Term::Literal(l) => l.language(),
        _ => None,
    }
}

// ── sh:minLength / sh:maxLength (§7.4.1–2) ──────────────────────────────────────────────────────

/// `sh:MinLengthConstraintComponent`. Violated when a value node's string form has fewer than
/// `min` characters, or when it is a blank node (no string form).
pub struct MinLengthValidator {
    /// Minimum character count.
    pub min: i64,
}

impl<G: RdfGraph> Validator<G> for MinLengthValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MinLengthConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let ok = string_form(v).is_some_and(|s| (s.chars().count() as i64) >= self.min);
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("MinLengthConstraintComponent"),
                ));
            }
        }
    }
}

/// `sh:MaxLengthConstraintComponent`. Violated when a value node's string form has more than `max`
/// characters, or when it is a blank node.
pub struct MaxLengthValidator {
    /// Maximum character count.
    pub max: i64,
}

impl<G: RdfGraph> Validator<G> for MaxLengthValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#MaxLengthConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let ok = string_form(v).is_some_and(|s| (s.chars().count() as i64) <= self.max);
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("MaxLengthConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:pattern / sh:flags (§7.4.3) ──────────────────────────────────────────────────────────────

/// `sh:PatternConstraintComponent`. `REQ-PATTERN-1..3`. A value node conforms iff its string form
/// matches the regex; blank nodes (no lexical form) always violate (`REQ-PATTERN-3`).
pub struct PatternValidator {
    /// Compiled regex (built from `sh:pattern` + `sh:flags`, ADR-005 fancy-regex). `None` if the
    /// pattern failed to compile — the constraint is then skipped (ill-formed shape).
    pub regex: Option<Regex>,
}

impl PatternValidator {
    /// Build from the raw `sh:pattern` string and optional `sh:flags` (SPARQL `REGEX` flags).
    #[must_use]
    pub fn new(pattern: &str, flags: Option<&str>) -> Self {
        // SPARQL/XPath flags i, s, m, x map onto inline flags; prepend `(?flags)`. `q` (literal)
        // and `u` are not supported by the inline syntax and are dropped (documented gap, ADR-005).
        let supported: String = flags
            .unwrap_or("")
            .chars()
            .filter(|c| matches!(c, 'i' | 's' | 'm' | 'x'))
            .collect();
        let full = if supported.is_empty() {
            pattern.to_string()
        } else {
            format!("(?{supported}){pattern}")
        };
        PatternValidator {
            regex: Regex::new(&full).ok(),
        }
    }
}

impl<G: RdfGraph> Validator<G> for PatternValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#PatternConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        let Some(re) = &self.regex else { return };
        for v in value_nodes {
            // REQ-PATTERN-3: a value with no lexical form (blank node) produces a result.
            let ok = string_form(v).is_some_and(|s| re.is_match(s).unwrap_or(false));
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("PatternConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:singleLine (§7.4.4, new in 1.2) ──────────────────────────────────────────────────────────

/// A line-break character: LF, CR, form feed, vertical tab, NEL, line/paragraph separator. A value
/// containing any of these is multi-line for `sh:singleLine` (W3C `core/property/singleLine-001`).
fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

/// `sh:SingleLineConstraintComponent`. When enabled, a value node's string form must contain no
/// line break (see [`is_line_break`]). Blank nodes violate.
pub struct SingleLineValidator;

impl<G: RdfGraph> Validator<G> for SingleLineValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#SingleLineConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let ok = string_form(v).is_some_and(|s| !s.contains(is_line_break));
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("SingleLineConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:languageIn (§7.4.5) ──────────────────────────────────────────────────────────────────────

/// `sh:LanguageInConstraintComponent`. A value node conforms iff it is a language-tagged literal
/// whose tag matches one of the listed basic language ranges (case-insensitive; `range` matches a
/// tag equal to it or extending it with a `-` subtag, per BCP47 basic filtering).
pub struct LanguageInValidator {
    /// The admitted basic language ranges (lower-cased).
    pub ranges: Vec<String>,
}

impl<G: RdfGraph> Validator<G> for LanguageInValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#LanguageInConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        for v in value_nodes {
            let ok = language_of(v).is_some_and(|tag| {
                let tag = tag.to_ascii_lowercase();
                self.ranges
                    .iter()
                    .any(|r| tag == *r || tag.starts_with(&format!("{r}-")))
            });
            if !ok {
                out.push(result_for(
                    ctx,
                    Some(v.clone()),
                    comp("LanguageInConstraintComponent"),
                ));
            }
        }
    }
}

// ── sh:uniqueLang (§7.4.6) ──────────────────────────────────────────────────────────────────────

/// `sh:UniqueLangConstraintComponent` (property shapes only). When enabled, no language tag may be
/// used by more than one value node. One result per offending tag, with no `sh:value`.
pub struct UniqueLangValidator;

impl<G: RdfGraph> Validator<G> for UniqueLangValidator {
    fn component_iri(&self) -> NamedNodeRef<'static> {
        NamedNodeRef::new_unchecked("http://www.w3.org/ns/shacl#UniqueLangConstraintComponent")
    }
    fn validate(&self, value_nodes: &[Term], ctx: &Ctx<'_, G>, out: &mut Vec<ValidationResult>) {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for v in value_nodes {
            if let Some(tag) = language_of(v) {
                if !tag.is_empty() {
                    *counts.entry(tag.to_ascii_lowercase()).or_default() += 1;
                }
            }
        }
        for (_tag, n) in counts {
            if n > 1 {
                out.push(result_for(ctx, None, comp("UniqueLangConstraintComponent")));
            }
        }
    }
}

//! Prefix handling (§8.3, `REQ-SPQ-13`). Full collection via the `sh:prefixes/owl:imports*/
//! sh:declare` property path over the shapes graph is deferred; this provides the surface step —
//! prepending `PREFIX` lines to a query before it is parsed.

/// Prepend `PREFIX p: <ns>` lines for each `(prefix, namespace)` mapping to `query`.
#[must_use]
pub fn with_prefixes(mappings: &[(String, String)], query: &str) -> String {
    let mut out = String::new();
    for (prefix, namespace) in mappings {
        out.push_str(&format!("PREFIX {prefix}: <{namespace}>\n"));
    }
    out.push_str(query);
    out
}

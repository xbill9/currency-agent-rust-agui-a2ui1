//! The Express output contract for the system prompt. Structured like the
//! official SDK's Express prompt (a2ui-agent-sdk 0.11.0): grammar rules, then
//! per-catalog positional signatures generated from the catalog schemas.

use super::catalog::{Catalog, schema_allows_databinding};

const CONTRACT: &str = r#"# A2UI Express Output Contract

Output the user interface in A2UI Express. Surround the whole Express block with the literal tags `<a2ui>` and `</a2ui>` on their own lines. Do not wrap the tags in backticks or a markdown code fence. The host compiles the block into A2UI JSON.

## Grammar Rules

1. Component constructors are assigned to variables or nested inline in a parent's arguments:
   header = Text("Rates")
   root = Column([header, Card(Text("Hello"))])
   Positional arguments follow the signatures below; `name=value` keyword arguments and `_` placeholders for skipped optional arguments are also accepted.
2. The tree has one entry point, the reserved variable `root`.
3. Strings use `"` or `"""` (escapes \n, \t, \\, \"); raw strings are prefixed with `r`. Numbers, `true`, `false` and `null` are written as in JSON.
4. Lists are `[a, b]`; maps are `{title: "Overview", child: body}` with literal keys.
5. Data bindings are absolute paths prefixed with `$`, e.g. `$/user/name`. Populate the data model by assigning to a path: `$/user/name = "Ada"`.
6. Checks are prefixed with `?`, e.g. `?required("Name is required")`.
7. Actions use `Event(name, context?)`, e.g. `Event("refresh", {pair: "USD/EUR"})`.
8. Call catalog functions by their signatures, e.g. `formatString("Hello ${/user/name}")`.
9. Arguments marked `(static)` must be literals; do not bind them to `$` paths.
10. Target a surface with `surface("id")` on its own line; remove one with `deleteSurface("id")`.

## Multiple Catalogs

Several catalogs are active and the signatures below are grouped by catalog. Do not name a catalog on a `surface` line; each component is found by its name. Add `catalogId="..."` to a component only when two catalogs define the same name.
"#;

fn signature(cat: &Catalog, name: &str, params: &[String], required: &[String], schema_of: impl Fn(&str) -> Option<serde_json::Value>) -> String {
    let parts: Vec<String> = params
        .iter()
        .map(|p| {
            let opt = if required.iter().any(|r| r == p) { "" } else { "?" };
            let stat = match schema_of(p) {
                Some(s) if !schema_allows_databinding(&s) => " (static)",
                _ => "",
            };
            format!("{p}{opt}{stat}")
        })
        .collect();
    let _ = cat;
    format!("• {name}({})", parts.join(", "))
}

fn describe(s: &Option<serde_json::Value>) -> Option<String> {
    let s = s.as_ref()?;
    let mut d = s.get("description")?.as_str()?.to_string();
    if let Some(e) = s.get("enum").and_then(|e| e.as_array()) {
        let vals: Vec<String> = e.iter().map(|v| format!("'{}'", v.as_str().unwrap_or(&v.to_string()))).collect();
        d.push_str(&format!(" Must be one of: {}", vals.join(", ")));
    }
    Some(d)
}

/// Builds the prompt snippet for the given catalogs (inference catalogs,
/// including a macro sidecar if there is one).
pub fn express_prompt(catalogs: &[Catalog]) -> String {
    let mut out = String::from(CONTRACT);
    for cat in catalogs {
        out.push_str(&format!("- `{}`\n", cat.catalog_id));
    }
    for cat in catalogs {
        out.push_str(&format!("\n# Catalog `{}`\n\n## Component Signatures\n\n", cat.catalog_id));
        let mut names: Vec<&String> = cat.components.keys().collect();
        names.sort();
        for name in names {
            let params = cat.component_properties(name).to_vec();
            let required = cat.component_required(name).to_vec();
            out.push_str(&signature(cat, name, &params, &required, |p| cat.property_schema(name, p)));
            out.push('\n');
            if let Some(d) = cat.component_description(name) {
                out.push_str(&format!("  - Description: {d}\n"));
            }
            for p in &params {
                if let Some(d) = describe(&cat.property_schema(name, p)) {
                    out.push_str(&format!("  - {p}: {d}\n"));
                }
            }
        }
        if !cat.functions.is_empty() {
            out.push_str("\n## Function Signatures\n\n");
            let mut fnames: Vec<&String> = cat.functions.keys().collect();
            fnames.sort();
            for f in fnames {
                let params = cat.function_properties(f).to_vec();
                let req: Vec<String> = cat.functions[f]
                    .get("properties")
                    .and_then(|p| p.get("args"))
                    .and_then(|a| a.get("required"))
                    .and_then(|r| r.as_array())
                    .map(|r| r.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                out.push_str(&signature(cat, f, &params, &req, |p| cat.function_property_schema(f, p)));
                out.push('\n');
                if let Some(d) = cat.function_description(f) {
                    out.push_str(&format!("  - Description: {d}\n"));
                }
            }
        }
    }
    out
}

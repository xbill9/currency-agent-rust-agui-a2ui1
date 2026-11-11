//! A2UI 1.0 server-side macros in Rust, modeled on the Python SDK's
//! `@macro` / `MacroExpander` (a2ui-agent-sdk 0.11.0).
//!
//! A macro is published to the model as a component of an inference-only
//! sidecar catalog (`https://a2ui.org/catalogs/macros`), so the model writes one
//! line such as `root = RateCard("USD", "EUR", 250)`. After compilation,
//! [`MacroExpander::expand`] replaces each macro component with the catalog
//! components its Rust function builds, with ids `<instance>__<name>_<n>`, and
//! the sidecar never reaches the renderer.

use super::ExpressError;
use super::catalog::Catalog;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

pub const MACRO_CATALOG_ID: &str = "https://a2ui.org/catalogs/macros";

/// A component tree a macro returns. Child slots hold nested trees.
#[derive(Debug, Clone)]
pub struct UiNode {
    pub component: String,
    pub catalog_id: String,
    pub props: Vec<(String, Prop)>,
}

#[derive(Debug, Clone)]
pub enum Prop {
    Value(Value),
    Child(Box<UiNode>),
    Children(Vec<UiNode>),
}

impl UiNode {
    pub fn new(component: &str, catalog_id: &str) -> Self {
        Self {
            component: component.into(),
            catalog_id: catalog_id.into(),
            props: Vec::new(),
        }
    }
    pub fn prop(mut self, k: &str, v: impl Into<Value>) -> Self {
        self.props.push((k.into(), Prop::Value(v.into())));
        self
    }
    pub fn child(mut self, k: &str, c: UiNode) -> Self {
        self.props.push((k.into(), Prop::Child(Box::new(c))));
        self
    }
    pub fn children(mut self, k: &str, cs: Vec<UiNode>) -> Self {
        self.props.push((k.into(), Prop::Children(cs)));
        self
    }
}

/// A static macro parameter (the model passes a literal).
#[derive(Debug, Clone)]
pub struct MacroParam {
    pub name: &'static str,
    /// JSON Schema type: "string", "number" or "integer".
    pub json_type: &'static str,
    pub description: &'static str,
    pub required: bool,
}

pub type MacroFn = fn(&Map<String, Value>) -> Result<UiNode, String>;

#[derive(Clone)]
pub struct Macro {
    pub name: &'static str,
    pub description: &'static str,
    pub params: Vec<MacroParam>,
    pub expand: MacroFn,
}

pub struct MacroExpander {
    macros: Vec<Macro>,
}

impl MacroExpander {
    pub fn new(macros: Vec<Macro>) -> Self {
        Self { macros }
    }

    /// The sidecar catalog that makes the macros visible to the compiler and prompt.
    pub fn sidecar_catalog(&self) -> Catalog {
        let mut components = Map::new();
        let mut any = Vec::new();
        for m in &self.macros {
            let mut props = Map::new();
            props.insert("component".into(), json!({"const": m.name}));
            let mut required = vec![json!("component")];
            for p in &m.params {
                props.insert(p.name.into(), json!({"type": p.json_type, "description": p.description}));
                if p.required {
                    required.push(json!(p.name));
                }
            }
            components.insert(
                m.name.into(),
                json!({"type": "object", "description": m.description,
                       "properties": props, "required": required}),
            );
            any.push(json!({"$ref": format!("#/components/{}", m.name)}));
        }
        let doc = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "catalogId": MACRO_CATALOG_ID,
            "protocolVersion": "1.0",
            "components": components,
            "$defs": {"anyComponent": {"oneOf": any, "discriminator": {"propertyName": "component"}},
                      "anyFunction": {"not": {}}}
        });
        Catalog::from_json(&doc).expect("macro sidecar catalog")
    }

    /// Expands every macro component in the messages into catalog components.
    pub fn expand(&self, messages: Vec<Value>) -> Result<Vec<Value>, ExpressError> {
        let by_name: HashMap<&str, &Macro> = self.macros.iter().map(|m| (m.name, m)).collect();
        let mut out = Vec::with_capacity(messages.len());
        for mut msg in messages {
            for op in ["createSurface", "updateComponents"] {
                if let Some(Value::Array(comps)) = msg.get_mut(op).and_then(|b| b.get_mut("components")) {
                    let mut expanded = Vec::with_capacity(comps.len());
                    for comp in comps.drain(..) {
                        let is_macro = comp.get("catalogId").and_then(|c| c.as_str()) == Some(MACRO_CATALOG_ID);
                        let name = comp["component"].as_str().unwrap_or_default().to_string();
                        match by_name.get(name.as_str()) {
                            Some(m) if is_macro || !comp.get("catalogId").is_some() => {
                                let id = comp["id"].as_str().unwrap_or("root").to_string();
                                let args: Map<String, Value> = comp
                                    .as_object()
                                    .unwrap()
                                    .iter()
                                    .filter(|(k, _)| !["id", "component", "catalogId"].contains(&k.as_str()))
                                    .map(|(k, v)| (k.clone(), v.clone()))
                                    .collect();
                                let tree = (m.expand)(&args).map_err(|e| {
                                    ExpressError::Validation(format!("Macro {name}({id}) failed: {e}"))
                                })?;
                                flatten(&tree, &id, &id, &mut HashMap::new(), &mut expanded);
                            }
                            _ => expanded.push(comp),
                        }
                    }
                    *comps = expanded;
                }
            }
            out.push(msg);
        }
        Ok(out)
    }
}

/// Post-order flattening: children first, the macro's root takes `id`.
fn flatten(node: &UiNode, id: &str, instance: &str, counters: &mut HashMap<String, usize>, out: &mut Vec<Value>) {
    let mut obj = Map::new();
    let child_id = |n: &UiNode, counters: &mut HashMap<String, usize>| {
        let key = n.component.to_lowercase();
        let c = counters.entry(key.clone()).or_insert(0);
        *c += 1;
        format!("{instance}__{key}_{c}")
    };
    let mut pending: Vec<(String, Value)> = Vec::new();
    for (k, p) in &node.props {
        match p {
            Prop::Value(v) => pending.push((k.clone(), v.clone())),
            Prop::Child(c) => {
                let cid = child_id(c, counters);
                flatten(c, &cid, instance, counters, out);
                pending.push((k.clone(), json!(cid)));
            }
            Prop::Children(cs) => {
                let mut ids = Vec::new();
                for c in cs {
                    let cid = child_id(c, counters);
                    flatten(c, &cid, instance, counters, out);
                    ids.push(json!(cid));
                }
                pending.push((k.clone(), Value::Array(ids)));
            }
        }
    }
    obj.insert("id".into(), json!(id));
    obj.insert("component".into(), json!(node.component));
    obj.insert("catalogId".into(), json!(node.catalog_id));
    for (k, v) in pending {
        obj.insert(k, v);
    }
    out.push(Value::Object(obj));
}

/// Checks expanded messages against the catalogs the renderer implements:
/// every component is declared by the catalog it names, carries its required
/// properties and only declared ones, and every child id it names exists.
pub fn check_transport(messages: &[Value], catalogs: &[Catalog]) -> Result<(), ExpressError> {
    let err = |m: String| ExpressError::Validation(m);
    for msg in messages {
        for op in ["createSurface", "updateComponents"] {
            let Some(comps) = msg.get(op).and_then(|b| b.get("components")).and_then(|c| c.as_array()) else {
                continue;
            };
            let ids: HashSet<&str> = comps.iter().filter_map(|c| c["id"].as_str()).collect();
            for c in comps {
                let name = c["component"].as_str().unwrap_or_default();
                let cat = match c.get("catalogId").and_then(|v| v.as_str()) {
                    Some(id) => catalogs.iter().find(|k| k.catalog_id == id),
                    None if catalogs.len() == 1 => catalogs.first(),
                    None => None,
                }
                .ok_or_else(|| err(format!("Component '{name}' names no catalog the renderer implements.")))?;
                if !cat.defines_component(name) {
                    return Err(err(format!("Catalog '{}' does not define '{name}'.", cat.catalog_id)));
                }
                let props = cat.component_properties(name);
                for req in cat.component_required(name) {
                    if req != "component" && req != "checks" && c.get(req).is_none() {
                        return Err(err(format!("Component '{name}' is missing required '{req}'.")));
                    }
                }
                for (k, v) in c.as_object().unwrap() {
                    if ["id", "component", "catalogId", "metadata", "checks"].contains(&k.as_str()) {
                        continue;
                    }
                    if !props.iter().any(|p| p == k) {
                        return Err(err(format!("Component '{name}' has undeclared property '{k}'.")));
                    }
                    if k == "child" {
                        if let Some(r) = v.as_str() {
                            if !ids.contains(r) {
                                return Err(err(format!("Component '{name}' child '{r}' does not exist.")));
                            }
                        }
                    }
                    if k == "children" {
                        if let Some(list) = v.as_array() {
                            for r in list.iter().filter_map(|r| r.as_str()) {
                                if !ids.contains(r) {
                                    return Err(err(format!("Component '{name}' child '{r}' does not exist.")));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

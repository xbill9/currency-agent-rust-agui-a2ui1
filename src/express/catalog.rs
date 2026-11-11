//! A2UI 1.0 catalog loading and the schema lookups the Express compiler and
//! prompt generator need. Ported from `CatalogSchemaHelper`
//! (a2ui-agent-sdk 0.11.0, `inference_formats/_shared/schema_helper.py`).

use super::ExpressError;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::LazyLock;

/// The A2UI v1.0 common types schema (`specification/v1_0/json/common_types.json`).
pub static COMMON_TYPES_V1_0: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("common_types_v1_0.json")).expect("common types JSON")
});

/// The v1.0 Basic catalog (`catalogs/basic/v1/catalog.json`).
pub fn basic_catalog_v1_0() -> Catalog {
    let doc: Value =
        serde_json::from_str(include_str!("basic_catalog_v1_0.json")).expect("basic catalog JSON");
    Catalog::from_json(&doc).expect("basic catalog loads")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    Child,
    ChildList,
    Action,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub catalog_id: String,
    pub protocol_version: String,
    /// The catalog document with local references inlined.
    pub doc: Value,
    pub components: Map<String, Value>,
    pub functions: Map<String, Value>,
    pub system_functions: Map<String, Value>,
    component_properties: HashMap<String, Vec<String>>,
    component_required: HashMap<String, Vec<String>>,
    component_checkable: HashMap<String, bool>,
    component_enums: HashMap<(String, String), Vec<Value>>,
    function_properties: HashMap<String, Vec<String>>,
}

fn common_def_name(r: &str) -> Option<String> {
    // (?:^|/)common_types\.json#/(?:\$defs|definitions)/(\w+)$
    let idx = r.find("common_types.json#/")?;
    if idx > 0 && !r[..idx].ends_with('/') {
        return None;
    }
    let tail = &r[idx + "common_types.json#/".len()..];
    let name = tail
        .strip_prefix("$defs/")
        .or_else(|| tail.strip_prefix("definitions/"))?;
    if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Some(name.to_string())
    } else {
        None
    }
}

fn lookup_common_def(name: &str) -> Option<&'static Value> {
    COMMON_TYPES_V1_0.get("$defs").and_then(|d| d.get(name))
}

fn json_pointer<'a>(root: &'a Value, pointer: &str) -> Option<&'a Value> {
    let p = pointer.strip_prefix('#')?;
    if p.is_empty() {
        return Some(root);
    }
    root.pointer(p)
}

fn preserved_type_refs() -> Vec<String> {
    COMMON_TYPES_V1_0
        .get("$defs")
        .and_then(|d| d.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Writes local `#/...` references inline, merging sibling keys over the target.
fn inline_local_refs(node: &Value, root: &Value, visited: &mut Vec<String>, keep: &[String]) -> Value {
    match node {
        Value::Object(obj) => {
            if let Some(Value::String(r)) = obj.get("$ref") {
                if r.starts_with("#/") {
                    let name = r.rsplit('/').next().unwrap_or("");
                    if r.starts_with("#/$defs/") && keep.iter().any(|k| k == name) {
                        return node.clone();
                    }
                    if visited.contains(r) {
                        return node.clone();
                    }
                    if let Some(target) = json_pointer(root, r) {
                        visited.push(r.clone());
                        let resolved = inline_local_refs(target, root, visited, keep);
                        visited.pop();
                        let mut res = match resolved {
                            Value::Object(m) => m,
                            other => return other,
                        };
                        for (k, v) in obj {
                            if k == "$ref" {
                                continue;
                            }
                            let v = inline_local_refs(v, root, visited, keep);
                            match (res.get_mut(k), v) {
                                (Some(Value::Object(a)), Value::Object(b)) => {
                                    for (bk, bv) in b {
                                        a.insert(bk, bv);
                                    }
                                }
                                (Some(Value::Array(a)), Value::Array(b)) => {
                                    for x in b {
                                        if !a.contains(&x) {
                                            a.push(x);
                                        }
                                    }
                                }
                                (_, v) => {
                                    res.insert(k.clone(), v);
                                }
                            }
                        }
                        return Value::Object(res);
                    }
                }
            }
            Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), inline_local_refs(v, root, visited, keep)))
                    .collect(),
            )
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .map(|v| inline_local_refs(v, root, visited, keep))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Component sub-schemas: the specific definition (whose properties include
/// `component`) first, then common mixins, each in pre-order.
fn ordered_component_sub_schemas(schema: &Value) -> Vec<&Value> {
    let mut specific = Vec::new();
    let mut common = Vec::new();
    fn visit<'a>(node: &'a Value, is_root: bool, specific: &mut Vec<&'a Value>, common: &mut Vec<&'a Value>) {
        let Some(obj) = node.as_object() else { return };
        let props = obj.get("properties").and_then(|p| p.as_object());
        if props.is_some_and(|p| p.contains_key("component")) {
            specific.push(node);
        } else if !is_root || props.is_some() || obj.contains_key("required") || obj.contains_key("$ref") {
            common.push(node);
        }
        if let Some(Value::Array(all)) = obj.get("allOf") {
            for sub in all {
                visit(sub, false, specific, common);
            }
        }
    }
    visit(schema, true, &mut specific, &mut common);
    specific.extend(common);
    specific
}

fn find_enum(s: &Value) -> Option<Vec<Value>> {
    let obj = s.as_object()?;
    if let Some(Value::Array(e)) = obj.get("enum") {
        return Some(e.clone());
    }
    for k in ["oneOf", "anyOf", "allOf"] {
        if let Some(Value::Array(subs)) = obj.get(k) {
            for sub in subs {
                if let Some(e) = find_enum(sub) {
                    if !e.is_empty() {
                        return Some(e);
                    }
                }
            }
        }
    }
    None
}

fn function_signature(schema: &Value) -> Vec<String> {
    let mut subs = vec![schema];
    if let Some(Value::Array(all)) = schema.get("allOf") {
        subs.extend(all.iter());
    }
    let mut props: Vec<String> = Vec::new();
    for sub in subs {
        if let Some(args_props) = sub
            .get("properties")
            .and_then(|p| p.get("args"))
            .and_then(|a| a.get("properties"))
            .and_then(|p| p.as_object())
        {
            for k in args_props.keys() {
                if !props.contains(k) {
                    props.push(k.clone());
                }
            }
        }
    }
    props
}

/// Returns `Action` if a property schema references the common `Action` type.
pub fn is_action_slot(schema: &Value) -> bool {
    let Some(obj) = schema.as_object() else { return false };
    if let Some(Value::String(r)) = obj.get("$ref") {
        if common_def_name(r).as_deref() == Some("Action") {
            return true;
        }
    }
    ["allOf", "oneOf", "anyOf"].iter().any(|k| {
        obj.get(*k)
            .and_then(|v| v.as_array())
            .is_some_and(|subs| subs.iter().any(is_action_slot))
    })
}

/// `Child`, `ChildList` or `Action` when a schema references one.
pub fn schema_ref_type(schema: &Value) -> Option<RefType> {
    let obj = schema.as_object()?;
    if let Some(Value::String(r)) = obj.get("$ref") {
        if r.contains("ChildList") {
            return Some(RefType::ChildList);
        }
        if r.contains("Child") || r.contains("ComponentId") {
            return Some(RefType::Child);
        }
        if r.contains("Action") {
            return Some(RefType::Action);
        }
    }
    for k in ["oneOf", "anyOf", "allOf"] {
        if let Some(Value::Array(subs)) = obj.get(k) {
            for sub in subs {
                if let Some(t) = schema_ref_type(sub) {
                    return Some(t);
                }
            }
        }
    }
    None
}

/// Whether a property schema admits a `$path` binding.
pub fn schema_allows_databinding(schema: &Value) -> bool {
    let Some(obj) = schema.as_object() else { return false };
    if let Some(Value::String(r)) = obj.get("$ref") {
        if r.contains("DataBinding") || r.contains("Dynamic") {
            return true;
        }
    }
    if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
        if (props.contains_key("path") || props.contains_key("@path"))
            && !props.contains_key("componentId")
        {
            return true;
        }
    }
    if let Some(items) = obj.get("items") {
        if schema_allows_databinding(items) {
            return true;
        }
    }
    ["allOf", "oneOf", "anyOf"].iter().any(|k| {
        obj.get(*k)
            .and_then(|v| v.as_array())
            .is_some_and(|subs| subs.iter().any(schema_allows_databinding))
    })
}

/// Whether a list schema expects `{label, value}` option objects.
pub fn schema_expects_option_objects(schema: &Value) -> bool {
    fn has_label_value(sub: &Value) -> bool {
        let Some(o) = sub.as_object() else { return false };
        if o.get("properties")
            .and_then(|p| p.as_object())
            .is_some_and(|p| p.contains_key("label") && p.contains_key("value"))
        {
            return true;
        }
        ["allOf", "oneOf", "anyOf"].iter().any(|k| {
            o.get(*k)
                .and_then(|v| v.as_array())
                .is_some_and(|s| s.iter().any(has_label_value))
        })
    }
    let Some(obj) = schema.as_object() else { return false };
    if let Some(items) = obj.get("items") {
        return has_label_value(items);
    }
    ["allOf", "oneOf", "anyOf"].iter().any(|k| {
        obj.get(*k)
            .and_then(|v| v.as_array())
            .is_some_and(|s| s.iter().any(schema_expects_option_objects))
    })
}

impl Catalog {
    /// Loads a v1.0 catalog document.
    pub fn from_json(raw: &Value) -> Result<Self, ExpressError> {
        let catalog_id = raw
            .get("catalogId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExpressError::Catalog("catalog has no catalogId".into()))?
            .to_string();
        let protocol_version = raw
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0")
            .to_string();
        if protocol_version != "1.0" {
            return Err(ExpressError::Catalog(format!(
                "catalog '{catalog_id}' targets A2UI {protocol_version}; this compiler supports 1.0 only"
            )));
        }
        let keep = preserved_type_refs();
        let doc = inline_local_refs(raw, raw, &mut Vec::new(), &keep);
        let components = doc
            .get("components")
            .and_then(|c| c.as_object())
            .cloned()
            .unwrap_or_default();
        let functions = match doc.get("functions") {
            Some(Value::Object(m)) => m.clone(),
            Some(Value::Array(a)) => a
                .iter()
                .filter_map(|f| {
                    let name = f
                        .get("properties")
                        .and_then(|p| p.get("@call"))
                        .and_then(|c| c.get("const"))
                        .and_then(|c| c.as_str())?;
                    Some((name.to_string(), f.clone()))
                })
                .collect(),
            _ => Map::new(),
        };
        let mut system_functions = Map::new();
        if let Some(defs) = COMMON_TYPES_V1_0.get("$defs").and_then(|d| d.as_object()) {
            for schema in defs.values() {
                if let Some(name) = schema
                    .get("properties")
                    .and_then(|p| p.get("@call"))
                    .and_then(|c| c.get("const"))
                    .and_then(|c| c.as_str())
                {
                    if name.starts_with('@') {
                        system_functions.insert(name.to_string(), schema.clone());
                    }
                }
            }
        }
        let mut cat = Catalog {
            catalog_id,
            protocol_version,
            doc,
            components,
            functions,
            system_functions,
            component_properties: HashMap::new(),
            component_required: HashMap::new(),
            component_checkable: HashMap::new(),
            component_enums: HashMap::new(),
            function_properties: HashMap::new(),
        };
        cat.load_mappings();
        Ok(cat)
    }

    fn load_mappings(&mut self) {
        let names: Vec<String> = self.components.keys().cloned().collect();
        for name in names {
            let schema = self.components[&name].clone();
            let mut props: Vec<String> = Vec::new();
            let mut reqs: Vec<String> = Vec::new();
            let mut checkable = false;
            for sub in ordered_component_sub_schemas(&schema) {
                if let Some(Value::String(r)) = sub.get("$ref") {
                    if r.contains("Checkable") {
                        checkable = true;
                    }
                }
                if let Some(p) = sub.get("properties").and_then(|p| p.as_object()) {
                    for k in p.keys() {
                        if !props.contains(k) {
                            props.push(k.clone());
                        }
                    }
                }
                if let Some(Value::Array(r)) = sub.get("required") {
                    for x in r.iter().filter_map(|x| x.as_str()) {
                        if !reqs.iter().any(|e| e == x) {
                            reqs.push(x.to_string());
                        }
                    }
                }
            }
            let mut ordered: Vec<String> = props
                .into_iter()
                .filter(|k| {
                    !(k == "component" || k == "id" || k == "catalogId" || (checkable && k == "checks"))
                })
                .collect();
            for k in &ordered {
                if let Some(e) = self.property_schema(&name, k).and_then(|s| find_enum(&s)) {
                    if !e.is_empty() {
                        self.component_enums.insert((name.clone(), k.clone()), e);
                    }
                }
            }
            if checkable {
                ordered.push("checks".into());
            }
            self.component_properties.insert(name.clone(), ordered);
            self.component_required.insert(name.clone(), reqs);
            self.component_checkable.insert(name, checkable);
        }
        for (name, schema) in self.functions.iter().chain(self.system_functions.iter()) {
            self.function_properties
                .insert(name.clone(), function_signature(schema));
        }
    }

    pub fn defines_component(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }
    pub fn defines_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }
    pub fn component_properties(&self, name: &str) -> &[String] {
        self.component_properties.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn component_required(&self, name: &str) -> &[String] {
        self.component_required.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn is_checkable(&self, name: &str) -> bool {
        self.component_checkable.get(name).copied().unwrap_or(false)
    }
    pub fn property_enum(&self, comp: &str, prop: &str) -> Option<&Vec<Value>> {
        self.component_enums.get(&(comp.to_string(), prop.to_string()))
    }
    pub fn function_properties(&self, name: &str) -> &[String] {
        self.function_properties.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn component_description(&self, name: &str) -> Option<String> {
        let s = self.components.get(name)?;
        if let Some(d) = s.get("description").and_then(|d| d.as_str()) {
            return Some(d.to_string());
        }
        s.get("allOf")?
            .as_array()?
            .iter()
            .find_map(|sub| sub.get("description").and_then(|d| d.as_str()).map(String::from))
    }

    pub fn function_description(&self, name: &str) -> Option<String> {
        self.functions
            .get(name)
            .or_else(|| self.system_functions.get(name))?
            .get("description")?
            .as_str()
            .map(String::from)
    }

    /// The schema of a component property; the component's own definition wins.
    pub fn property_schema(&self, comp: &str, prop: &str) -> Option<Value> {
        let schema = self.components.get(comp)?;
        if let Some(v) = schema.get("properties").and_then(|p| p.get(prop)) {
            return v.is_object().then(|| v.clone());
        }
        for sub in ordered_component_sub_schemas(schema) {
            if let Some(v) = sub.get("properties").and_then(|p| p.get(prop)) {
                return v.is_object().then(|| v.clone());
            }
        }
        None
    }

    pub fn property_type(&self, comp: &str, prop: &str) -> Option<RefType> {
        schema_ref_type(&self.property_schema(comp, prop)?)
    }

    pub fn function_property_schema(&self, func: &str, prop: &str) -> Option<Value> {
        let schema = self.functions.get(func).or_else(|| self.system_functions.get(func))?;
        let mut subs = vec![schema];
        if let Some(Value::Array(all)) = schema.get("allOf") {
            subs.extend(all.iter());
        }
        for sub in subs {
            if let Some(v) = sub
                .get("properties")
                .and_then(|p| p.get("args"))
                .and_then(|a| a.get("properties"))
                .and_then(|p| p.get(prop))
            {
                return v.is_object().then(|| v.clone());
            }
        }
        None
    }

    /// Whether a property schema accepts a numeric value, following `$ref`s.
    pub fn accepts_number(&self, schema: &Value) -> bool {
        let mut visited: Vec<*const Value> = Vec::new();
        self.check_number(schema, false, &mut visited)
    }

    fn check_number(&self, s: &Value, in_common: bool, visited: &mut Vec<*const Value>) -> bool {
        let Some(obj) = s.as_object() else { return false };
        let ptr = s as *const Value;
        if visited.contains(&ptr) {
            return false;
        }
        visited.push(ptr);
        match obj.get("type") {
            Some(Value::String(t)) if t == "number" || t == "integer" => return true,
            Some(Value::Array(ts))
                if ts.iter().any(|t| t == "number" || t == "integer") =>
            {
                return true;
            }
            _ => {}
        }
        if obj.get("const").is_some_and(|c| c.is_number()) {
            return true;
        }
        if let Some(Value::Array(e)) = obj.get("enum") {
            if e.iter().any(|v| v.is_number()) {
                return true;
            }
        }
        if let Some(Value::String(r)) = obj.get("$ref") {
            let (resolved, common) = if let Some(n) = common_def_name(r) {
                (lookup_common_def(&n), true)
            } else if in_common && (r.starts_with("#/$defs/") || r.starts_with("#/definitions/")) {
                let n = r.rsplit('/').next().unwrap_or("");
                (lookup_common_def(n), true)
            } else if r.starts_with("#/") {
                (json_pointer(&self.doc, r), false)
            } else if let Some(p) = r.strip_prefix("catalog.json") {
                (json_pointer(&self.doc, p), false)
            } else {
                return true;
            };
            match resolved {
                None => return true,
                Some(v) => {
                    if self.check_number(v, common, visited) {
                        return true;
                    }
                }
            }
        }
        for k in ["allOf", "oneOf", "anyOf"] {
            if let Some(Value::Array(subs)) = obj.get(k) {
                for sub in subs {
                    if self.check_number(sub, in_common, visited) {
                        return true;
                    }
                }
            }
        }
        const TYPE_KEYS: [&str; 11] = [
            "type", "const", "enum", "$ref", "allOf", "oneOf", "anyOf", "properties", "items",
            "prefixItems", "not",
        ];
        !TYPE_KEYS.iter().any(|k| obj.contains_key(*k))
    }

    /// The schema at an array's items (`prop == None`) or an object's property.
    pub fn resolve_at(&self, schema: &Value, prop: Option<&str>) -> Option<Value> {
        let mut visited: Vec<*const Value> = Vec::new();
        self.resolve_inner(schema, prop, false, &mut visited)
    }

    fn resolve_inner(
        &self,
        s: &Value,
        prop: Option<&str>,
        in_common: bool,
        visited: &mut Vec<*const Value>,
    ) -> Option<Value> {
        let obj = s.as_object()?;
        let ptr = s as *const Value;
        if visited.contains(&ptr) {
            return None;
        }
        visited.push(ptr);
        match prop {
            None => {
                if let Some(items) = obj.get("items").filter(|v| v.is_object()) {
                    return Some(items.clone());
                }
            }
            Some(p) => {
                if let Some(d) = obj.get("properties").and_then(|ps| ps.get(p)).filter(|v| v.is_object()) {
                    return Some(d.clone());
                }
                if let Some(a) = obj.get("additionalProperties").filter(|v| v.is_object()) {
                    return Some(a.clone());
                }
            }
        }
        if let Some(Value::String(r)) = obj.get("$ref") {
            let found = if let Some(n) = common_def_name(r) {
                lookup_common_def(&n).and_then(|v| self.resolve_inner(v, prop, true, visited))
            } else if in_common && (r.starts_with("#/$defs/") || r.starts_with("#/definitions/")) {
                let n = r.rsplit('/').next().unwrap_or("");
                lookup_common_def(n).and_then(|v| self.resolve_inner(v, prop, true, visited))
            } else if r.starts_with("#/") {
                json_pointer(&self.doc, r).and_then(|v| self.resolve_inner(v, prop, false, visited))
            } else {
                None
            };
            if found.is_some() {
                return found;
            }
        }
        for k in ["allOf", "oneOf", "anyOf"] {
            if let Some(Value::Array(subs)) = obj.get(k) {
                for sub in subs {
                    if let Some(f) = self.resolve_inner(sub, prop, in_common, visited) {
                        return Some(f);
                    }
                }
            }
        }
        None
    }
}

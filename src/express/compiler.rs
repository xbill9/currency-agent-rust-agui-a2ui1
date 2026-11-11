//! A2UI Express → A2UI 1.0 message compiler. A port of `ExpressCompiler`
//! (a2ui-agent-sdk 0.11.0, `inference_formats/express/compiler.py`) for
//! protocol v1.0, the version the Rust agent targets.

use super::catalog::{
    Catalog, RefType, is_action_slot, schema_allows_databinding, schema_expects_option_objects,
    schema_ref_type,
};
use super::normalize::{normalize_source, strip_sentinel_tags};
use super::syntax::{Call, Check, Node, Stmt, parse_program};
use super::ExpressError;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

const VERSION: &str = "1.0";
const PATH_KEY: &str = "@path";
const CALL_KEY: &str = "@call";
const SURFACE: &str = "surface";
const DELETE_SURFACE: &str = "deleteSurface";
const EVENT_KEYWORDS: [&str; 2] = ["userMessage", "context"];
const MAX_ARRAY_INDEX: usize = 10_000;

fn verr(msg: impl Into<String>) -> ExpressError {
    ExpressError::Validation(msg.into())
}

pub fn is_uax31_identifier(s: &str) -> bool {
    let mut it = s.chars();
    match it.next() {
        Some(c) if c == '_' || unicode_ident::is_xid_start(c) => {}
        _ => return false,
    }
    it.all(unicode_ident::is_xid_continue)
}

/// Ordered symbol table: a reassigned name keeps its first position.
#[derive(Default, Clone)]
struct Symbols(Vec<(String, Node)>);

impl Symbols {
    fn get(&self, k: &str) -> Option<&Node> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }
    fn contains(&self, k: &str) -> bool {
        self.get(k).is_some()
    }
    fn set(&mut self, k: String, v: Node) {
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| *n == k) {
            slot.1 = v;
        } else {
            self.0.push((k, v));
        }
    }
    /// Removes then appends, so the latest assignment is last.
    fn set_last(&mut self, k: String, v: Node) {
        self.0.retain(|(n, _)| *n != k);
        self.0.push((k, v));
    }
}

#[derive(Default)]
struct Scope {
    surface_id: String,
    send_data_model: Option<bool>,
    is_update: bool,
    extensions: Option<Node>,
    symbols: Symbols,
    data: Symbols,
}

enum Unit {
    Scope(Scope),
    Delete(String),
    Standalone(Call, Option<usize>),
}

#[derive(Default)]
struct Ctx {
    extra: Vec<Value>,
    inline_counter: usize,
    call_counter: usize,
    active_value_path: Option<Value>,
    resolving: HashSet<String>,
    used_ids: HashSet<String>,
}

/// A schema position (property, array item or map entry) in a given catalog.
#[derive(Clone)]
struct Slot<'a> {
    schema: Value,
    cat: &'a Catalog,
}

#[derive(Clone, Copy, Default)]
struct ValueOpts<'p> {
    is_action: bool,
    parent_id: Option<&'p str>,
    parent_prop: Option<&'p str>,
    list_index: Option<usize>,
    allow_component_ref: bool,
    allow_list_component_refs: bool,
}

pub struct Compiler {
    catalogs: Vec<Catalog>,
}

fn call_arg<'n>(node: &'n Call, index: usize, key: &str) -> Option<&'n Node> {
    node.kwarg(key).or_else(|| node.args.get(index))
}

fn string_call_arg(node: &Call, index: usize, key: &str, stmt: &str) -> Result<Option<String>, ExpressError> {
    match call_arg(node, index, key) {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Str(s)) => Ok(Some(s.clone())),
        Some(other) => Err(verr(format!(
            "{stmt}() argument '{key}' must be a string, got: {other:?}"
        ))),
    }
}

fn component_id(var_name: &str, node: &Call) -> Result<String, ExpressError> {
    match node.kwarg("id") {
        None => Ok(var_name.to_string()),
        Some(Node::Str(s)) if !s.is_empty() => {
            if !is_uax31_identifier(s) {
                return Err(verr(format!(
                    "Component id {s:?} is not a valid UAX 31 identifier."
                )));
            }
            Ok(s.clone())
        }
        Some(other) => Err(verr(format!(
            "Component id must be a non-empty string, got: {other:?}"
        ))),
    }
}

fn is_check_expression(n: &Node) -> bool {
    match n {
        Node::Check(_) => true,
        Node::Array(items) => !items.is_empty() && items.iter().all(is_check_expression),
        _ => false,
    }
}

fn binding_path(v: &Value) -> Option<String> {
    let obj = v.as_object()?;
    if obj.contains_key("componentId") {
        return None;
    }
    obj.get(PATH_KEY)?.as_str().map(String::from)
}

fn resolves_to_path(node: &Node, syms: &Symbols) -> bool {
    let mut seen = HashSet::new();
    let mut cur = node;
    while let Node::Var(name) = cur {
        if seen.contains(name) || !syms.contains(name) {
            return false;
        }
        seen.insert(name.clone());
        cur = syms.get(name).unwrap();
    }
    matches!(cur, Node::Path(_))
}

fn has_databinding(node: &Node, syms: &Symbols, seen: &HashSet<String>) -> bool {
    match node {
        Node::Path(_) => true,
        Node::Var(name) => {
            if seen.contains(name) {
                return false;
            }
            match syms.get(name) {
                None => false,
                Some(v) => {
                    let mut s2 = seen.clone();
                    s2.insert(name.clone());
                    has_databinding(v, syms, &s2)
                }
            }
        }
        Node::Map(entries) => entries.iter().any(|(_, v)| has_databinding(v, syms, seen)),
        Node::Array(items) => items.iter().any(|v| has_databinding(v, syms, seen)),
        _ => false,
    }
}

fn to_json_pointer(path: &str) -> String {
    let p = path.strip_prefix("$/").or_else(|| path.strip_prefix('$')).unwrap_or(path);
    format!("/{p}")
}

fn parse_array_index(seg: &str, path: &str) -> Result<usize, ExpressError> {
    let canonical = seg == "0"
        || (!seg.is_empty() && !seg.starts_with('0') && seg.bytes().all(|b| b.is_ascii_digit()));
    if !canonical {
        return Err(verr(format!(
            "Cannot use non-numeric segment {seg:?} on an array in path {path:?}."
        )));
    }
    let idx: usize = seg.parse().map_err(|_| verr("bad index"))?;
    if idx > MAX_ARRAY_INDEX {
        return Err(verr(format!(
            "Cannot set path {path:?}: array index {seg:?} exceeds maximum supported index ({MAX_ARRAY_INDEX})."
        )));
    }
    Ok(idx)
}

fn is_index(seg: &str) -> bool {
    seg == "0" || (!seg.is_empty() && !seg.starts_with('0') && seg.bytes().all(|b| b.is_ascii_digit()))
}

fn set_nested_path(root: &mut Map<String, Value>, path: &str, val: Value) -> Result<(), ExpressError> {
    let clean = path.strip_prefix("$/").or_else(|| path.strip_prefix('$')).unwrap_or(path);
    if clean.is_empty() {
        let Value::Object(m) = val else {
            return Err(verr(format!(
                "Cannot set root data model {path:?} to a non-map value."
            )));
        };
        *root = m;
        return Ok(());
    }
    let keys: Vec<&str> = clean.split('/').collect();
    let mut cur: &mut Value = &mut Value::Null;
    // Work on a temporary Value wrapper around root.
    let mut wrapper = Value::Object(std::mem::take(root));
    let res = (|| {
        cur = &mut wrapper;
        for i in 0..keys.len() - 1 {
            let key = keys[i];
            let next_is_index = is_index(keys[i + 1]);
            let fresh = || if next_is_index { Value::Array(vec![]) } else { Value::Object(Map::new()) };
            cur = match cur {
                Value::Object(m) => {
                    let e = m.entry(key.to_string()).or_insert(Value::Null);
                    if e.is_null() {
                        *e = fresh();
                    } else if !(e.is_object() || e.is_array()) {
                        return Err(verr(format!(
                            "Cannot set path {path:?}: segment {key:?} is a primitive value."
                        )));
                    }
                    e
                }
                Value::Array(a) => {
                    let idx = parse_array_index(key, path)?;
                    while a.len() <= idx {
                        a.push(Value::Null);
                    }
                    if a[idx].is_null() {
                        a[idx] = fresh();
                    } else if !(a[idx].is_object() || a[idx].is_array()) {
                        return Err(verr(format!(
                            "Cannot set path {path:?}: segment {key:?} is a primitive value."
                        )));
                    }
                    &mut a[idx]
                }
                _ => unreachable!(),
            };
        }
        let last = keys[keys.len() - 1];
        match cur {
            Value::Object(m) => {
                m.insert(last.to_string(), val);
            }
            Value::Array(a) => {
                let idx = parse_array_index(last, path)?;
                while a.len() <= idx {
                    a.push(Value::Null);
                }
                a[idx] = val;
            }
            _ => unreachable!(),
        }
        Ok(())
    })();
    if let Value::Object(m) = wrapper {
        *root = m;
    }
    res
}

fn describe(n: &Node) -> String {
    match n {
        Node::Map(_) => "a map literal".into(),
        Node::Array(_) => "an array".into(),
        Node::Num(x) => format!("the number {x}"),
        Node::Bool(b) => format!("the boolean {b}"),
        Node::Null => "null".into(),
        Node::Call(c) => format!("a call to {}()", c.name),
        Node::Check(c) => format!("the check ?{}", c.name),
        _ => format!("{n:?}"),
    }
}

impl Compiler {
    pub fn new(catalogs: Vec<Catalog>) -> Self {
        assert!(!catalogs.is_empty(), "at least one catalog");
        Self { catalogs }
    }

    pub fn catalogs(&self) -> &[Catalog] {
        &self.catalogs
    }

    fn multi(&self) -> bool {
        self.catalogs.len() > 1
    }

    fn helper_for(&self, id: &str) -> Result<&Catalog, ExpressError> {
        self.catalogs.iter().find(|c| c.catalog_id == id).ok_or_else(|| {
            verr(format!(
                "Unknown catalog '{id}'. Available catalogs: {:?}",
                self.catalogs.iter().map(|c| &c.catalog_id).collect::<Vec<_>>()
            ))
        })
    }

    fn defines_component(&self, name: &str, cat: Option<&Node>) -> bool {
        if let Some(Node::Str(id)) = cat {
            if let Some(c) = self.catalogs.iter().find(|c| &c.catalog_id == id) {
                return c.defines_component(name);
            }
        }
        self.catalogs.iter().any(|c| c.defines_component(name))
    }

    fn defines_function(&self, name: &str, cat: Option<&Node>) -> bool {
        if name.starts_with('@') {
            return true;
        }
        if let Some(cat) = cat {
            if let Node::Str(id) = cat {
                if let Some(c) = self.catalogs.iter().find(|c| &c.catalog_id == id) {
                    return c.defines_function(name);
                }
            }
            return false;
        }
        self.catalogs.iter().any(|c| c.defines_function(name))
    }

    /// Resolves the catalog for a component or function, and the catalogId to write.
    fn resolve(
        &self,
        component: bool,
        name: &str,
        cat: Option<&Node>,
        target: &str,
        label: &str,
    ) -> Result<(&Catalog, Option<String>), ExpressError> {
        if let Some(cat) = cat {
            let Node::Str(id) = cat else {
                return Err(verr(format!("{target} catalogId must be a string, got: {cat:?}")));
            };
            let h = self.helper_for(id)?;
            let defined = if component { h.defines_component(name) } else { h.defines_function(name) };
            if !defined {
                return Err(verr(format!(
                    "Unknown {label} '{name}' not defined in catalog '{id}'."
                )));
            }
            return Ok((h, Some(id.clone())));
        }
        let matches: Vec<&Catalog> = self
            .catalogs
            .iter()
            .filter(|c| if component { c.defines_component(name) } else { c.defines_function(name) })
            .collect();
        if matches.len() > 1 {
            return Err(verr(format!(
                "{label} '{name}' is defined in several catalogs. Name the catalog to use with a catalogId argument."
            )));
        }
        let Some(h) = matches.first() else {
            return Err(verr(format!("Unknown {label} '{name}' not defined in any catalog.")));
        };
        Ok((h, self.multi().then(|| h.catalog_id.clone())))
    }

    fn system_function<'c>(&'c self, name: &str, call_has_catalog: bool) -> Result<&'c Catalog, ExpressError> {
        let h = &self.catalogs[0];
        if !h.system_functions.contains_key(name) {
            let names: Vec<&String> = h.system_functions.keys().collect();
            return Err(verr(format!(
                "Unknown system function '{name}' in A2UI {VERSION}. System functions: {names:?}."
            )));
        }
        if call_has_catalog {
            return Err(verr(format!(
                "System function '{name}' is defined by the protocol, not a catalog, so it takes no catalogId."
            )));
        }
        Ok(h)
    }

    /// Compiles one Express block (with or without `<a2ui>` tags) into A2UI 1.0 messages.
    pub fn compile(&self, dsl: &str, default_surface: &str) -> Result<Vec<Value>, ExpressError> {
        let body = strip_sentinel_tags(dsl);
        let body = normalize_source(&body);
        let stmts = parse_program(&body)?;
        let units = self.build_units(stmts, default_surface)?;
        if units.is_empty() {
            return Err(ExpressError::Parse(
                "No root component or data model update found in the A2UI block.".into(),
            ));
        }
        let mut ctx = Ctx::default();
        let mut out = Vec::new();
        let scopes: Vec<&Unit> = units.iter().collect();
        for unit in &scopes {
            match unit {
                Unit::Delete(id) => out.push(json!({"version": VERSION, "deleteSurface": {"surfaceId": id}})),
                Unit::Standalone(call, scope_idx) => {
                    let empty = Symbols::default();
                    let syms = match scope_idx {
                        Some(i) => match &units[*i] {
                            Unit::Scope(s) => &s.symbols,
                            _ => &empty,
                        },
                        None => &empty,
                    };
                    out.push(self.compile_standalone(call.clone(), syms, &mut ctx)?);
                }
                Unit::Scope(scope) => out.extend(self.compile_scope(scope, &mut ctx)?),
            }
        }
        Ok(out)
    }

    fn build_units(&self, stmts: Vec<Stmt>, default_surface: &str) -> Result<Vec<Unit>, ExpressError> {
        let mut units: Vec<Unit> = Vec::new();
        let mut current: Option<usize> = None;
        for st in stmts {
            match st {
                Stmt::Expr(Node::Call(call)) => {
                    if call.name == SURFACE {
                        units.push(Unit::Scope(self.open_scope(&call, default_surface)?));
                        current = Some(units.len() - 1);
                    } else if call.name == DELETE_SURFACE {
                        let target = string_call_arg(&call, 0, "surfaceId", DELETE_SURFACE)?
                            .ok_or_else(|| verr("deleteSurface() requires the id of the surface to delete."))?;
                        units.push(Unit::Delete(target));
                    } else {
                        units.push(Unit::Standalone(call, current));
                    }
                }
                Stmt::Expr(_) => {}
                Stmt::Assign(name, value) => {
                    let idx = match current {
                        Some(i) => i,
                        None => {
                            units.push(Unit::Scope(Scope {
                                surface_id: default_surface.to_string(),
                                ..Default::default()
                            }));
                            current = Some(units.len() - 1);
                            units.len() - 1
                        }
                    };
                    let Unit::Scope(scope) = &mut units[idx] else { unreachable!() };
                    if name.starts_with('$') {
                        scope.data.set_last(name, value);
                    } else {
                        scope.symbols.set(name, value);
                    }
                }
            }
        }
        Ok(units)
    }

    fn open_scope(&self, node: &Call, default_surface: &str) -> Result<Scope, ExpressError> {
        let surface_id =
            string_call_arg(node, 0, "surfaceId", SURFACE)?.unwrap_or_else(|| default_surface.to_string());
        if let Some(cat) = string_call_arg(node, 1, "catalogId", SURFACE)? {
            if self.multi() {
                return Err(verr(format!(
                    "surface() names catalog '{cat}', but a surface names no catalog when several catalogs are active."
                )));
            }
            self.helper_for(&cat)?;
        }
        let send_data_model = match node.kwarg("sendDataModel") {
            None | Some(Node::Null) => None,
            Some(Node::Bool(b)) => Some(*b),
            Some(other) => {
                return Err(verr(format!("sendDataModel must be true or false, got: {other:?}")));
            }
        };
        let is_update = match node.kwarg("update") {
            None | Some(Node::Null) => false,
            Some(Node::Bool(b)) => *b,
            Some(other) => return Err(verr(format!("update must be true or false, got: {other:?}"))),
        };
        let mut extensions = None;
        if let Some(ext) = node.kwarg("extensions") {
            if is_update {
                return Err(verr("surface() with update=true does not accept extensions."));
            }
            extensions = Some(ext.clone());
        }
        Ok(Scope {
            surface_id,
            send_data_model,
            is_update,
            extensions,
            ..Default::default()
        })
    }

    fn compile_scope(&self, scope: &Scope, ctx: &mut Ctx) -> Result<Vec<Value>, ExpressError> {
        // Seed component ids so inline ids cannot collide with declared ones.
        ctx.used_ids.clear();
        for (var, node) in &scope.symbols.0 {
            if let Node::Call(c) = node {
                if self.defines_component(&c.name, c.kwarg("catalogId")) {
                    let id = component_id(var, c)?;
                    if !ctx.used_ids.insert(id.clone()) {
                        return Err(verr(format!("Duplicate component ID '{id}'.")));
                    }
                }
            }
        }
        let mut components: Vec<Value> = Vec::new();
        for (var, node) in &scope.symbols.0 {
            if let Some(comp) = self.compile_ast_node(var, node, &scope.symbols, ctx)? {
                components.push(comp);
                components.append(&mut ctx.extra);
            }
        }
        let is_create = !scope.is_update && components.iter().any(|c| c["id"] == "root");
        if !is_create {
            if scope.extensions.is_some() {
                return Err(verr(
                    "surface() extensions require a surface-creating scope with a root component.",
                ));
            }
            let mut updates = Vec::new();
            for (path, node) in &scope.data.0 {
                let pointer = to_json_pointer(path);
                let val = self.compile_value(node, &scope.symbols, ctx, ValueOpts::default(), None)?;
                if pointer == "/" && !val.is_object() {
                    return Err(verr(format!(
                        "Cannot assign a non-map value to the root data path '$/' (got {val})."
                    )));
                }
                updates.push(json!({"version": VERSION, "updateDataModel": {
                    "surfaceId": scope.surface_id, "path": pointer, "value": val}}));
            }
            if components.is_empty() {
                if !updates.is_empty() {
                    return Ok(updates);
                }
                return Err(ExpressError::Parse("No root component found in the A2UI block.".into()));
            }
            let mut msgs = vec![json!({"version": VERSION, "updateComponents": {
                "surfaceId": scope.surface_id, "components": components}})];
            msgs.extend(updates);
            return Ok(msgs);
        }
        let mut data_model = Map::new();
        for (path, node) in &scope.data.0 {
            let val = self.compile_value(node, &scope.symbols, ctx, ValueOpts::default(), None)?;
            set_nested_path(&mut data_model, path, val)?;
        }
        let mut create = Map::new();
        create.insert("surfaceId".into(), json!(scope.surface_id));
        if !self.multi() {
            create.insert("catalogId".into(), json!(self.catalogs[0].catalog_id));
        }
        if let Some(sdm) = scope.send_data_model {
            create.insert("sendDataModel".into(), json!(sdm));
        }
        if let Some(ext) = &scope.extensions {
            let compiled = self.compile_extensions(ext, &scope.symbols, "surface()", ctx)?;
            create.insert("metadata".into(), json!({"extensions": compiled}));
        }
        create.insert("components".into(), Value::Array(components));
        if !data_model.is_empty() {
            create.insert("dataModel".into(), Value::Object(data_model));
        }
        Ok(vec![json!({"version": VERSION, "createSurface": create})])
    }

    fn compile_standalone(&self, mut node: Call, syms: &Symbols, ctx: &mut Ctx) -> Result<Value, ExpressError> {
        ctx.call_counter += 1;
        let mut fcid = format!("call_{}", ctx.call_counter);
        if let Some(explicit) = node.remove_kwarg("functionCallId") {
            match explicit {
                Node::Str(s) if !s.is_empty() => fcid = s,
                other => {
                    return Err(verr(format!("functionCallId must be a non-empty string, got: {other:?}")));
                }
            }
        }
        if node.name.starts_with('@') {
            return Err(verr(format!(
                "System function '{}' cannot be called as a standalone statement, since a renderer function call names a catalog function.",
                node.name
            )));
        }
        let compiled = self.compile_value(&Node::Call(node), syms, ctx, ValueOpts::default(), None)?;
        let Some(obj) = compiled.as_object().filter(|o| o.contains_key(CALL_KEY)) else {
            return Err(verr(format!(
                "Standalone statement did not compile to a valid function call: {compiled}"
            )));
        };
        let cat_id = obj
            .get("catalogId")
            .cloned()
            .unwrap_or_else(|| json!(self.catalogs[0].catalog_id));
        // The renderer only runs calls the catalog lets an agent make: not a
        // `rendererOnly` function (the default) and not one that needs a user
        // gesture (validate_payload's INVALID_FUNCTION_CALL rule in 0.11.0).
        let fname = obj[CALL_KEY].as_str().unwrap_or_default();
        if let Some(schema) = cat_id
            .as_str()
            .and_then(|id| self.catalogs.iter().find(|c| c.catalog_id == id))
            .and_then(|c| c.functions.get(fname))
        {
            let callers = schema.get("allowedCallers").and_then(|v| v.as_str()).unwrap_or("rendererOnly");
            if callers == "rendererOnly" {
                return Err(verr(format!(
                    "Function '{fname}' is rendererOnly, so an agent cannot call it with callRendererFunction."
                )));
            }
            if schema.get("requiresUserActivation").and_then(|v| v.as_bool()) == Some(true) {
                return Err(verr(format!(
                    "Function '{fname}' requires user activation, so an agent cannot call it with callRendererFunction."
                )));
            }
        }
        Ok(json!({"version": VERSION, "callRendererFunction": {
            "functionCallId": fcid,
            "callFunction": {
                "catalogId": cat_id,
                CALL_KEY: obj[CALL_KEY],
                "args": obj.get("args").cloned().unwrap_or_else(|| json!({})),
            }
        }}))
    }

    fn resolve_ast_var<'n>(
        &self,
        node: &'n Node,
        syms: &'n Symbols,
        seen: &mut HashSet<String>,
    ) -> Result<&'n Node, ExpressError> {
        let mut cur = node;
        while let Node::Var(name) = cur {
            if !seen.insert(name.clone()) {
                return Err(verr(format!("Variable '{name}' refers to itself.")));
            }
            cur = syms
                .get(name)
                .ok_or_else(|| verr(format!("Undefined variable '{name}' in {VERSION} expression.")))?;
        }
        Ok(cur)
    }

    fn check_extensions_ast(&self, node: &Node, syms: &Symbols, target: &str, seen: &HashSet<String>) -> Result<(), ExpressError> {
        let mut s2 = seen.clone();
        let resolved = self.resolve_ast_var(node, syms, &mut s2)?;
        match resolved {
            Node::Map(entries) => {
                for (_, v) in entries {
                    self.check_extensions_ast(v, syms, target, seen)?;
                }
            }
            Node::Array(items) => {
                for v in items {
                    self.check_extensions_ast(v, syms, target, seen)?;
                }
            }
            Node::Call(c) => {
                let cat = c.kwarg("catalogId");
                if self.defines_component(&c.name, cat) {
                    return Err(verr(format!("{target} extensions cannot contain component '{}'.", c.name)));
                }
                if ["Event", "_template", SURFACE, DELETE_SURFACE].contains(&c.name.as_str()) && cat.is_none() {
                    return Err(verr(format!("{target} extensions cannot contain '{}()'.", c.name)));
                }
                for a in &c.args {
                    self.check_extensions_ast(a, syms, target, seen)?;
                }
                for (_, v) in &c.kwargs {
                    self.check_extensions_ast(v, syms, target, seen)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_extensions(&self, raw: &Node, syms: &Symbols, target: &str, ctx: &mut Ctx) -> Result<Value, ExpressError> {
        let top = self.resolve_ast_var(raw, syms, &mut HashSet::new())?;
        if !matches!(top, Node::Map(_)) {
            return Err(verr(format!("{target} extensions must be a map, got: {top:?}")));
        }
        self.check_extensions_ast(top, syms, target, &HashSet::new())?;
        let compiled = self.compile_value(raw, syms, ctx, ValueOpts::default(), None)?;
        let Some(obj) = compiled.as_object() else {
            return Err(verr(format!("{target} extensions must be a map, got: {compiled}")));
        };
        for k in obj.keys() {
            if !is_uax31_identifier(k) {
                return Err(verr(format!("{target} extension key {k:?} is not a valid UAX 31 identifier.")));
            }
        }
        Ok(compiled)
    }

    /// Compiles a variable's value into a component, or None if it is not one.
    fn compile_ast_node(&self, var: &str, node: &Node, syms: &Symbols, ctx: &mut Ctx) -> Result<Option<Value>, ExpressError> {
        let Node::Call(call) = node else { return Ok(None) };
        let name = call.name.as_str();
        let mut kwargs = call.kwargs.clone();
        if name.starts_with('@') {
            self.system_function(name, call.has_kwarg("catalogId"))?;
            return Ok(None);
        }
        let cat_node = kwargs
            .iter()
            .position(|(k, _)| k == "catalogId")
            .map(|i| kwargs.remove(i).1);
        if !self.defines_component(name, cat_node.as_ref())
            && (((name == "Event" || name == "_template") && cat_node.is_none())
                || self.defines_function(name, cat_node.as_ref()))
        {
            return Ok(None);
        }
        let (cat, write_cat) =
            self.resolve(true, name, cat_node.as_ref(), &format!("Component '{name}'"), "component")?;
        let properties = cat.component_properties(name).to_vec();
        let comp_id = component_id(var, call)?;
        kwargs.retain(|(k, _)| k != "id");
        let raw_extensions = if !properties.iter().any(|p| p == "extensions") {
            kwargs
                .iter()
                .position(|(k, _)| k == "extensions")
                .map(|i| kwargs.remove(i).1)
        } else {
            None
        };
        let mut comp = Map::new();
        comp.insert("id".into(), json!(comp_id));
        comp.insert("component".into(), json!(name));
        if let Some(id) = &write_cat {
            comp.insert("catalogId".into(), json!(id));
        }
        if let Some(ext) = &raw_extensions {
            let compiled = self.compile_extensions(ext, syms, &format!("Component '{name}'"), ctx)?;
            comp.insert("metadata".into(), json!({"extensions": compiled}));
        }

        let non_check: Vec<String> = properties.iter().filter(|p| *p != "checks").cloned().collect();
        let mut raw_checks: Vec<Node> = Vec::new();
        let mut pos_args: Vec<&Node> = Vec::new();
        for a in &call.args {
            if is_check_expression(a) {
                match a {
                    Node::Array(items) => raw_checks.extend(items.iter().cloned()),
                    other => raw_checks.push(other.clone()),
                }
                continue;
            }
            pos_args.push(a);
        }
        let required: Vec<String> = cat.component_required(name).to_vec();
        let kw_names: Vec<String> = kwargs.iter().map(|(k, _)| k.clone()).collect();
        let mut pairs: Vec<(String, Node)> = Vec::new();
        let mut prop_idx = 0usize;
        for (arg_idx, arg) in pos_args.iter().enumerate() {
            if prop_idx >= non_check.len() {
                return Err(verr(format!(
                    "{name} takes at most {} positional arguments ({}), but component '{comp_id}' passes {}.",
                    non_check.len(),
                    non_check.join(", "),
                    pos_args.len()
                )));
            }
            let accepts = |idx: usize| {
                cat.property_schema(name, &non_check[idx])
                    .is_some_and(|s| cat.accepts_number(&s))
            };
            if matches!(arg, Node::Num(_)) && !accepts(prop_idx) {
                let mut moved = false;
                if arg_idx == pos_args.len() - 1 {
                    let later: Vec<usize> = (prop_idx + 1..non_check.len())
                        .filter(|i| !kw_names.contains(&non_check[*i]) && accepts(*i))
                        .collect();
                    if later.len() == 1 {
                        let target = later[0];
                        if (prop_idx..target).all(|i| {
                            !required.contains(&non_check[i]) && cat.property_enum(name, &non_check[i]).is_some()
                        }) {
                            prop_idx = target;
                            moved = true;
                        }
                    }
                }
                if !moved && !required.contains(&non_check[prop_idx]) {
                    return Err(verr(format!(
                        "Positional argument {} for component '{name}' is a number, which is not valid for optional property '{}'.",
                        arg_idx + 1,
                        non_check[prop_idx]
                    )));
                }
            }
            pairs.push((non_check[prop_idx].clone(), (*arg).clone()));
            prop_idx += 1;
        }
        for (k, v) in &kwargs {
            if is_check_expression(v) {
                match v {
                    Node::Array(items) => raw_checks.extend(items.iter().cloned()),
                    other => raw_checks.push(other.clone()),
                }
                continue;
            }
            pairs.push((k.clone(), v.clone()));
        }

        let mut seen_props: HashSet<String> = HashSet::new();
        let mut sibling_value_path: Option<Value> = None;
        for (prop, arg) in &pairs {
            if !properties.contains(prop) {
                return Err(verr(format!(
                    "Unknown property '{prop}' for component '{name}'. Valid properties: {}",
                    properties.join(", ")
                )));
            }
            if !seen_props.insert(prop.clone()) {
                return Err(verr(format!(
                    "Duplicate property '{prop}' for component '{name}'."
                )));
            }
            if matches!(arg, Node::Skipped) {
                comp.insert(prop.clone(), Value::Null);
                continue;
            }
            let schema = cat.property_schema(name, prop);
            let ptype = cat.property_type(name, prop);
            let opts = ValueOpts {
                is_action: schema.as_ref().is_some_and(is_action_slot),
                parent_id: Some(&comp_id),
                parent_prop: Some(prop),
                list_index: None,
                allow_component_ref: ptype == Some(RefType::Child),
                allow_list_component_refs: ptype == Some(RefType::ChildList) && matches!(arg, Node::Array(_)),
            };
            let slot = schema.clone().map(|s| Slot { schema: s, cat });
            let mut mapped = self.compile_value(arg, syms, ctx, opts, slot)?;
            if let Some(s) = &schema {
                if !schema_allows_databinding(s) {
                    if has_databinding(arg, syms, &HashSet::new()) {
                        return Err(verr(format!(
                            "Property '{prop}' of component '{name}' does not accept a data binding."
                        )));
                    }
                    if let Value::Array(items) = &mapped {
                        if schema_expects_option_objects(s) {
                            mapped = Value::Array(
                                items
                                    .iter()
                                    .map(|o| match o {
                                        Value::String(st) => json!({"label": st, "value": st}),
                                        other => other.clone(),
                                    })
                                    .collect(),
                            );
                        }
                    }
                }
            }
            if let Some(allowed) = cat.property_enum(name, prop) {
                if !mapped.is_object() && !mapped.is_array() && !allowed.contains(&mapped) {
                    return Err(verr(format!(
                        "Value '{mapped}' is not a valid enum choice for property '{prop}' of component '{name}'. Allowed values are: {allowed:?}"
                    )));
                }
            }
            if prop == "value" && resolves_to_path(arg, syms) {
                sibling_value_path = Some(mapped.clone());
            }
            comp.insert(prop.clone(), mapped);
        }
        if !raw_checks.is_empty() && !properties.iter().any(|p| p == "checks") {
            return Err(verr(format!(
                "Component '{name}' does not declare a 'checks' property, so it cannot take checks."
            )));
        }
        ctx.active_value_path = sibling_value_path;
        if !raw_checks.is_empty() {
            let mut compiled_checks = Vec::new();
            for rc in &raw_checks {
                if let Node::Check(chk) = rc {
                    let (cond, msg) = self.compile_check(chk, syms, ctx, false)?;
                    compiled_checks.push(json!({"condition": cond, "message": msg}));
                }
            }
            if !compiled_checks.is_empty() {
                comp.insert("checks".into(), Value::Array(compiled_checks));
            }
        }
        ctx.active_value_path = None;
        for req in &required {
            if !comp.contains_key(req) && req != "checks" {
                return Err(verr(format!(
                    "Component '{name}' missing required property '{req}'."
                )));
            }
        }
        comp.retain(|_, v| !v.is_null());
        Ok(Some(Value::Object(comp)))
    }

    fn compile_check(&self, rc: &Check, syms: &Symbols, ctx: &mut Ctx, is_action: bool) -> Result<(Value, String), ExpressError> {
        let name = &rc.name;
        let mut explicit: Vec<&Node> = Vec::new();
        let mut chk_cat: Option<Node> = None;
        for a in &rc.args {
            if let Node::Map(entries) = a {
                if let Some((_, cid)) = entries.iter().find(|(k, _)| k == "catalogId") {
                    let others: Vec<&String> = entries.iter().map(|(k, _)| k).filter(|k| *k != "catalogId").collect();
                    if !others.is_empty() {
                        return Err(verr(format!(
                            "Unsupported key(s) in check options for '?{name}': {others:?}. Only 'catalogId' is allowed."
                        )));
                    }
                    if matches!(cid, Node::Null) {
                        return Err(verr(format!("Check '{name}' catalogId must be a string, got: None")));
                    }
                    if chk_cat.as_ref().is_some_and(|c| c != cid) {
                        return Err(verr(format!("Check '?{name}' specifies conflicting catalogId values.")));
                    }
                    chk_cat = Some(cid.clone());
                    continue;
                }
            }
            explicit.push(a);
        }
        let (cat, write_cat) =
            self.resolve(false, name, chk_cat.as_ref(), &format!("Check '{name}'"), "check function")?;
        let props = cat.function_properties(name).to_vec();
        let mut cap = name.chars();
        let mut message = match cap.next() {
            Some(f) => format!("{}{} check failed", f.to_uppercase(), cap.as_str()),
            None => "Check failed".into(),
        };
        let mut args = Map::new();
        let mut injected = false;
        if props.first().map(|p| p.as_str()) == Some("value") {
            let explicit_binding = matches!(explicit.first(), Some(Node::Path(_)));
            if !explicit_binding {
                if let Some(v) = &ctx.active_value_path {
                    args.insert("value".into(), v.clone());
                    injected = true;
                }
            }
        }
        let start = usize::from(injected);
        for (i, a) in explicit.iter().enumerate() {
            let target = i + start;
            if target >= props.len() {
                if let Node::Str(s) = a {
                    message = s.clone();
                }
                continue;
            }
            let prop = &props[target];
            let schema = cat.function_property_schema(name, prop);
            if let Node::Str(s) = a {
                if schema
                    .as_ref()
                    .and_then(|s| s.get("type"))
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| ["integer", "number", "boolean"].contains(&t))
                {
                    message = s.clone();
                    break;
                }
            }
            if matches!(a, Node::Skipped) {
                continue;
            }
            let opts = ValueOpts { is_action, ..Default::default() };
            args.insert(prop.clone(), self.compile_value(a, syms, ctx, opts, None)?);
        }
        let mut cond = Map::new();
        cond.insert(CALL_KEY.into(), json!(name));
        if let Some(id) = write_cat {
            cond.insert("catalogId".into(), json!(id));
        }
        cond.insert("args".into(), Value::Object(args));
        Ok((Value::Object(cond), message))
    }

    fn compile_event(&self, call: &Call, syms: &Symbols, ctx: &mut Ctx) -> Result<Value, ExpressError> {
        if call.args.len() > 2 {
            return Err(verr(format!(
                "Event accepts at most 2 positional arguments (name, context), got {}.",
                call.args.len()
            )));
        }
        for (k, _) in &call.kwargs {
            if !EVENT_KEYWORDS.contains(&k.as_str()) {
                return Err(verr(format!(
                    "Invalid parameter '{k}' for Event. Valid parameters: userMessage, context"
                )));
            }
        }
        if call.args.len() == 2 && call.has_kwarg("context") {
            return Err(verr("Duplicate parameter 'context' for Event."));
        }
        let name_val = match call.args.first() {
            Some(a) => self.compile_value(a, syms, ctx, ValueOpts::default(), None)?,
            None => json!(""),
        };
        let mut ev = Map::new();
        ev.insert("name".into(), name_val);
        if let Some(um) = call.kwarg("userMessage") {
            let v = self.compile_value(um, syms, ctx, ValueOpts::default(), None)?;
            let ok = v.is_string()
                || v.as_object()
                    .is_some_and(|o| o.contains_key(PATH_KEY) || o.contains_key(CALL_KEY));
            if !ok {
                return Err(verr(format!(
                    "Event userMessage must be a string, a $path data binding or a function call, got {}.",
                    describe(um)
                )));
            }
            ev.insert("userMessage".into(), v);
        }
        let ctx_node = if call.args.len() == 2 {
            Some(&call.args[1])
        } else {
            call.kwarg("context")
        };
        if let Some(cn) = ctx_node {
            let raw = self.compile_value(cn, syms, ctx, ValueOpts::default(), None)?;
            let mut merged = Map::new();
            match raw {
                Value::Object(m) => merged.extend(m),
                Value::Array(items) => {
                    for it in items {
                        if let Value::Object(m) = it {
                            merged.extend(m);
                        }
                    }
                }
                _ => {}
            }
            ev.insert("context".into(), Value::Object(merged));
        }
        Ok(json!({"event": ev}))
    }

    fn compile_value(
        &self,
        val: &Node,
        syms: &Symbols,
        ctx: &mut Ctx,
        o: ValueOpts,
        slot: Option<Slot>,
    ) -> Result<Value, ExpressError> {
        match val {
            Node::Path(p) => Ok(json!({PATH_KEY: p})),
            Node::Var(name) => {
                let Some(sym) = syms.get(name) else {
                    if !o.allow_component_ref {
                        return Err(verr(format!("Variable '{name}' is not assigned in surface.")));
                    }
                    return Ok(json!(name));
                };
                if let Node::Call(c) = sym {
                    if self.defines_component(&c.name, c.kwarg("catalogId")) {
                        return Ok(json!(component_id(name, c)?));
                    }
                }
                if !ctx.resolving.insert(name.clone()) {
                    return Err(verr(format!(
                        "Variable '{name}' refers to itself through its own value."
                    )));
                }
                let opts = ValueOpts {
                    is_action: o.is_action,
                    allow_component_ref: o.allow_component_ref,
                    allow_list_component_refs: o.allow_list_component_refs,
                    ..Default::default()
                };
                let res = self.compile_value(sym, syms, ctx, opts, slot);
                ctx.resolving.remove(name);
                res
            }
            Node::Check(c) => Ok(self.compile_check(c, syms, ctx, o.is_action)?.0),
            Node::Call(c) => self.compile_call(c, syms, ctx, o),
            Node::Skipped => Err(verr("'_' can only stand in for an argument.")),
            Node::Map(entries) => {
                let mut out = Map::new();
                for (k, v) in entries {
                    let entry_slot = slot.as_ref().and_then(|s| {
                        s.cat.resolve_at(&s.schema, Some(k)).map(|schema| Slot { schema, cat: s.cat })
                    });
                    let rt = entry_slot.as_ref().and_then(|s| schema_ref_type(&s.schema));
                    let opts = ValueOpts {
                        is_action: o.is_action || entry_slot.as_ref().is_some_and(|s| is_action_slot(&s.schema)),
                        allow_component_ref: rt == Some(RefType::Child),
                        allow_list_component_refs: rt == Some(RefType::ChildList) && matches!(v, Node::Array(_)),
                        ..Default::default()
                    };
                    out.insert(k.clone(), self.compile_value(v, syms, ctx, opts, entry_slot)?);
                }
                Ok(Value::Object(out))
            }
            Node::Array(items) => {
                let item_slot = slot.as_ref().and_then(|s| {
                    s.cat.resolve_at(&s.schema, None).map(|schema| Slot { schema, cat: s.cat })
                });
                let rt = item_slot.as_ref().and_then(|s| schema_ref_type(&s.schema));
                let is_action = o.is_action || item_slot.as_ref().is_some_and(|s| is_action_slot(&s.schema));
                let mut out = Vec::with_capacity(items.len());
                for (i, it) in items.iter().enumerate() {
                    let opts = ValueOpts {
                        is_action,
                        parent_id: o.parent_id,
                        parent_prop: o.parent_prop,
                        list_index: Some(i),
                        allow_component_ref: o.allow_list_component_refs || rt == Some(RefType::Child),
                        allow_list_component_refs: false,
                    };
                    out.push(self.compile_value(it, syms, ctx, opts, item_slot.clone())?);
                }
                Ok(Value::Array(out))
            }
            Node::Str(s) => Ok(json!(s)),
            Node::Num(n) => Ok(Value::Number(n.clone())),
            Node::Bool(b) => Ok(json!(b)),
            Node::Null => Ok(Value::Null),
        }
    }

    fn compile_call(&self, c: &Call, syms: &Symbols, ctx: &mut Ctx, o: ValueOpts) -> Result<Value, ExpressError> {
        let name = c.name.as_str();
        if name == "_template" {
            if c.args.len() < 2 {
                return Err(ExpressError::Parse(
                    "_template helper requires exactly 2 arguments: path and templateComponent.".into(),
                ));
            }
            let path_val = self.compile_value(&c.args[0], syms, ctx, ValueOpts { is_action: o.is_action, ..Default::default() }, None)?;
            let Some(path) = binding_path(&path_val) else {
                return Err(ExpressError::Parse(format!(
                    "The first argument to _template must be a dynamic data binding path (prefixed by $), got: {:?}",
                    c.args[0]
                )));
            };
            let comp_id = self.compile_value(
                &c.args[1],
                syms,
                ctx,
                ValueOpts { is_action: o.is_action, allow_component_ref: true, ..Default::default() },
                None,
            )?;
            return Ok(json!({"path": path, "componentId": comp_id}));
        }
        if name == "Event" {
            return self.compile_event(c, syms, ctx);
        }
        if name.starts_with('@') {
            let cat = self.system_function(name, c.has_kwarg("catalogId"))?;
            let args = self.compile_call_args(cat, name, c, syms, ctx, o.is_action)?;
            let mut expr = Map::new();
            expr.insert(CALL_KEY.into(), json!(name));
            if !args.is_empty() {
                expr.insert("args".into(), Value::Object(args));
            }
            return Ok(if o.is_action { json!({"functionCall": expr}) } else { Value::Object(expr) });
        }
        let mut c2 = c.clone();
        let cat_node = c2.remove_kwarg("catalogId");
        if self.defines_component(name, cat_node.as_ref()) {
            let inline_id = match (o.parent_id, o.parent_prop) {
                (Some(pid), Some(pp)) => match o.list_index {
                    Some(i) => format!("{pid}_{pp}_{i}"),
                    None => format!("{pid}_{pp}"),
                },
                _ => {
                    ctx.inline_counter += 1;
                    format!("_inline_{}", ctx.inline_counter)
                }
            };
            let actual = component_id(&inline_id, c)?;
            if !ctx.used_ids.insert(actual.clone()) {
                return Err(verr(format!("Duplicate component ID '{actual}'.")));
            }
            let prev = std::mem::take(&mut ctx.extra);
            let compiled = self.compile_ast_node(&inline_id, &Node::Call(c.clone()), syms, ctx)?;
            let child_extras = std::mem::replace(&mut ctx.extra, prev);
            if let Some(comp) = compiled {
                let id = comp["id"].clone();
                ctx.extra.push(comp);
                ctx.extra.extend(child_extras);
                return Ok(id);
            }
            return Ok(json!(inline_id));
        }
        let (cat, write_cat) =
            self.resolve(false, name, cat_node.as_ref(), &format!("Call '{name}'"), "function")?;
        let args = self.compile_call_args(cat, name, &c2, syms, ctx, o.is_action)?;
        let mut expr = Map::new();
        expr.insert(CALL_KEY.into(), json!(name));
        if let Some(id) = write_cat {
            expr.insert("catalogId".into(), json!(id));
        }
        expr.insert("args".into(), Value::Object(args));
        Ok(if o.is_action { json!({"functionCall": expr}) } else { Value::Object(expr) })
    }

    fn compile_call_args(
        &self,
        cat: &Catalog,
        name: &str,
        c: &Call,
        syms: &Symbols,
        ctx: &mut Ctx,
        is_action: bool,
    ) -> Result<Map<String, Value>, ExpressError> {
        let props = cat.function_properties(name).to_vec();
        let mut out = Map::new();
        let opts = ValueOpts { is_action, ..Default::default() };
        for (i, a) in c.args.iter().enumerate() {
            if i >= props.len() || matches!(a, Node::Skipped) {
                continue;
            }
            let v = self.compile_value(a, syms, ctx, opts, None)?;
            if !v.is_null() {
                out.insert(props[i].clone(), v);
            }
        }
        for (k, v) in &c.kwargs {
            if !props.contains(k) {
                return Err(verr(format!(
                    "Invalid parameter '{k}' for function '{name}'. Valid parameters: {}",
                    props.join(", ")
                )));
            }
            if out.contains_key(k) {
                return Err(verr(format!("Duplicate parameter '{k}' for function '{name}'.")));
            }
            if matches!(v, Node::Skipped) {
                continue;
            }
            let cv = self.compile_value(v, syms, ctx, opts, None)?;
            if !cv.is_null() {
                out.insert(k.clone(), cv);
            }
        }
        Ok(out)
    }
}

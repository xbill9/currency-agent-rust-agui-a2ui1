//! Runs the upstream A2UI Express conformance suites (vendored from the
//! a2ui-agent-sdk 0.11.0 release, see tests/a2ui_conformance/SOURCE.txt)
//! against the Rust port, and prints a per-case report.
//!
//! `cargo test --test express_conformance -- --nocapture` shows the report.
//! Cases listed in KNOWN_GAPS are reported but do not fail the build; every
//! other case must pass.

use currency_agent::express::{
    Catalog, Compiler, ExpressError, Part, RawPart, has_format_content, parse_response, unwrap, wrap,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Cases the port does not pass yet, with the reason. Kept short and explicit.
const KNOWN_GAPS: &[(&str, &str)] = &[
    (
        "test_compile_express_unreferenced_variable_holding_a_system_function_before_v1_0_is_a_validation_error",
        "uses a v0.9 catalog; the port targets protocol v1.0 only",
    ),
    (
        "test_compile_express_index_system_function_before_v1_0_is_a_validation_error",
        "uses a v0.9 catalog; the port targets protocol v1.0 only",
    ),
    (
        "test_compile_express_event_user_message_before_v1_is_a_validation_error",
        "uses a v0.9 catalog; the port targets protocol v1.0 only",
    ),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/a2ui_conformance")
}

fn load_yaml(rel: &str) -> Vec<Value> {
    let text = std::fs::read_to_string(root().join(rel)).unwrap();
    let y: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
    let v: Value = serde_json::to_value(y).unwrap();
    v.as_array().unwrap().clone()
}

fn load_catalog(spec: &Value) -> Result<Catalog, ExpressError> {
    let doc = match spec {
        Value::String(rel) => {
            let text = std::fs::read_to_string(root().join(rel))
                .map_err(|e| ExpressError::Catalog(format!("{rel}: {e}")))?;
            serde_json::from_str(&text).map_err(|e| ExpressError::Catalog(e.to_string()))?
        }
        other => other.clone(),
    };
    Catalog::from_json(&doc)
}

fn make_compiler(args: &Value) -> Result<Compiler, ExpressError> {
    let mut cats = Vec::new();
    if let Some(c) = args.get("catalog") {
        cats.push(load_catalog(c)?);
    }
    if let Some(Value::Array(list)) = args.get("catalogs") {
        for c in list {
            cats.push(load_catalog(c)?);
        }
    }
    if cats.is_empty() {
        return Err(ExpressError::Catalog("no catalog in case".into()));
    }
    Ok(Compiler::new(cats))
}

fn error_matches(err: &ExpressError, expect: &Value) -> Result<(), String> {
    let cat = expect.get("category").and_then(|c| c.as_str()).unwrap_or("");
    if err.category() != cat {
        return Err(format!("expected {cat}, got {err}"));
    }
    if let Some(msg) = expect.get("message").and_then(|m| m.as_str()) {
        if !err.message().contains(msg) {
            return Err(format!("expected message containing {msg:?}, got {:?}", err.message()));
        }
    }
    Ok(())
}

fn delete_pointer(v: &mut Value, pointer: &str) {
    let (parent, last) = pointer.rsplit_once('/').unwrap();
    let target = if parent.is_empty() { Some(v) } else { v.pointer_mut(parent) };
    match target {
        Some(Value::Object(m)) => {
            m.shift_remove(last);
        }
        Some(Value::Array(a)) => {
            if let Ok(i) = last.parse::<usize>() {
                a.remove(i);
            }
        }
        _ => {}
    }
}

/// Numbers compare by value (YAML `1.0` and JSON `1` are the same number).
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| json_eq(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        _ => a == b,
    }
}

fn run_compile(case: &Value) -> Result<(), String> {
    let compiler = make_compiler(&case["args"]).map_err(|e| format!("catalog: {e}"))?;
    let input = case["input"].as_str().unwrap_or("");
    let res = compiler.compile(input, "default_surface");
    if let Some(expect_err) = case.get("expect_error") {
        return match res {
            Ok(m) => Err(format!("expected error, compiled to {}", Value::Array(m))),
            Err(e) => error_matches(&e, expect_err),
        };
    }
    let mut got = Value::Array(res.map_err(|e| e.to_string())?);
    if let Some(Value::Array(ptrs)) = case.get("expect_present") {
        for p in ptrs {
            let p = p.as_str().unwrap();
            match got.pointer(p) {
                None | Some(Value::Null) => return Err(format!("{p} missing")),
                Some(Value::String(s)) if s.is_empty() => return Err(format!("{p} empty")),
                _ => {}
            }
            delete_pointer(&mut got, p);
        }
    }
    if json_eq(&got, &case["expect"]) {
        Ok(())
    } else {
        Err(format!("\n  got:    {got}\n  expect: {}", case["expect"]))
    }
}

fn raw_parts_json(parts: &[RawPart]) -> Value {
    Value::Array(
        parts
            .iter()
            .map(|p| match p {
                RawPart::Text(t) => json!({"text": t}),
                RawPart::A2ui { raw, is_final: true } => json!({"a2ui_raw": raw}),
                RawPart::A2ui { raw, is_final: false } => json!({"a2ui_raw": raw, "is_final": false}),
            })
            .collect(),
    )
}

fn parts_from_json(v: &Value) -> Vec<RawPart> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| match p.get("text") {
            Some(t) => RawPart::Text(t.as_str().unwrap().to_string()),
            None => RawPart::A2ui {
                raw: p["a2ui_raw"].as_str().unwrap().to_string(),
                is_final: p.get("is_final").and_then(|f| f.as_bool()).unwrap_or(true),
            },
        })
        .collect()
}

fn run_response(case: &Value) -> Result<(), String> {
    let input = case.get("input").and_then(|i| i.as_str()).unwrap_or("");
    let args = &case["args"];
    match case["action"].as_str().unwrap() {
        "unwrap" => {
            let got = raw_parts_json(&unwrap(input));
            if !json_eq(&got, &case["expect"]) {
                return Err(format!("\n  got:    {got}\n  expect: {}", case["expect"]));
            }
            let exp = case["expect"].as_array().unwrap();
            let has_any = exp.iter().any(|p| p.get("a2ui_raw").is_some());
            let has_complete = exp
                .iter()
                .any(|p| p.get("a2ui_raw").is_some() && p.get("is_final").and_then(|f| f.as_bool()).unwrap_or(true));
            if has_format_content(input, false) != has_any || has_format_content(input, true) != has_complete {
                return Err("has_format_content disagrees with unwrap".into());
            }
            Ok(())
        }
        "has_format_content" => {
            let complete = args.get("complete").and_then(|c| c.as_bool()).unwrap_or(false);
            let got = has_format_content(input, complete);
            if Some(got) == case["expect"].as_bool() { Ok(()) } else { Err(format!("got {got}")) }
        }
        "wrap" => {
            let parts = parts_from_json(&case["parts"]);
            let out = wrap(&parts);
            if let Some(e) = case.get("expect_output").and_then(|e| e.as_str()) {
                if out != e {
                    return Err(format!("got {out:?}, expect {e:?}"));
                }
            }
            if let Some(Value::Array(frags)) = case.get("expect_contains") {
                for f in frags {
                    if !out.contains(f.as_str().unwrap()) {
                        return Err(format!("{f} not in {out:?}"));
                    }
                }
            }
            if case.get("expect_round_trip").and_then(|r| r.as_bool()) == Some(true) && unwrap(&out) != parts {
                return Err("round trip changed the parts".into());
            }
            Ok(())
        }
        "parse_response" => {
            let compiler = make_compiler(args).map_err(|e| format!("catalog: {e}"))?;
            let wrapped = args.get("wrapped").and_then(|w| w.as_bool()).unwrap_or(true);
            let res = parse_response(&compiler, input, wrapped, "default_surface");
            if let Some(expect_err) = case.get("expect_error") {
                return match res {
                    Ok(_) => Err("expected an error".into()),
                    Err(e) => error_matches(&e, expect_err),
                };
            }
            let got = Value::Array(
                res.map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|p| match p {
                        Part::Text(t) => json!({"text": t}),
                        Part::A2ui(m) => json!({"a2ui": m}),
                    })
                    .collect(),
            );
            if json_eq(&got, &case["expect"]) { Ok(()) } else { Err(format!("\n  got:    {got}\n  expect: {}", case["expect"])) }
        }
        other => Err(format!("unknown action {other}")),
    }
}

fn run_suite(file: &str, runner: fn(&Value) -> Result<(), String>) -> (usize, usize, Vec<String>) {
    let cases = load_yaml(file);
    let (mut pass, mut known) = (0, 0);
    let mut failures = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let result = std::panic::catch_unwind(|| runner(case))
            .unwrap_or_else(|_| Err("panicked".into()));
        match result {
            Ok(()) => pass += 1,
            Err(why) => {
                if let Some((_, reason)) = KNOWN_GAPS.iter().find(|(n, _)| *n == name) {
                    known += 1;
                    println!("  KNOWN GAP {name}: {reason}");
                } else {
                    println!("  FAIL {name}: {why}");
                    failures.push(name.to_string());
                }
            }
        }
    }
    println!("{file}: {pass}/{} passed, {known} known gaps, {} failed", cases.len(), failures.len());
    (pass, cases.len(), failures)
}

#[test]
fn express_compiler_conformance() {
    let (_, _, failures) = run_suite("agent/express/compiler.yaml", run_compile);
    assert!(failures.is_empty(), "{} compiler cases failed: {failures:?}", failures.len());
}

#[test]
fn express_response_parser_conformance() {
    let (_, _, failures) = run_suite("agent/express/response_parser.yaml", run_response);
    assert!(failures.is_empty(), "{} response parser cases failed: {failures:?}", failures.len());
}

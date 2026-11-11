//! The currency agent's A2UI 1.0 setup: catalogs, server-side macros and the
//! reply pipeline that turns the model's Express blocks into A2UI 1.0 JSON.

use crate::express::macros::{Macro, MacroExpander, MacroParam, UiNode, check_transport};
use crate::express::prompt::express_prompt;
use crate::express::{Catalog, Compiler, Part, basic_catalog_v1_0, parse_response};
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub const BASIC_ID: &str = "https://a2ui.org/specification/v1_0/catalogs/basic/catalog.json";
pub const CHARTS_ID: &str = "https://a2ui.org/catalog/currency-charts/v1/catalog.json";

/// Surface id used when a block names none.
pub const DEFAULT_SURFACE: &str = "main";

fn frankfurter_url() -> String {
    std::env::var("FRANKFURTER_URL").unwrap_or_else(|_| "https://api.frankfurter.dev/v1".into())
}

pub fn charts_catalog() -> Catalog {
    let doc: Value = serde_json::from_str(include_str!("charts_catalog_v1_0.json")).expect("charts catalog JSON");
    Catalog::from_json(&doc).expect("charts catalog loads")
}

/// Model output is untrusted: accept only a three-letter ISO 4217 code.
fn code(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    let raw = args.get(key).and_then(|v| v.as_str()).ok_or(format!("{key} is required"))?;
    let c = raw.trim().to_uppercase();
    if c.len() == 3 && c.chars().all(|ch| ch.is_ascii_uppercase()) {
        Ok(c)
    } else {
        Err(format!("not an ISO 4217 currency code: {raw:?}"))
    }
}

fn get_json(path: &str, from: &str, to: &str) -> Result<Value, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{}/{path}", frankfurter_url()))
        .query(&[("from", from), ("to", to)])
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())
}

fn text(t: impl Into<String>, variant: Option<&str>) -> UiNode {
    let n = UiNode::new("Text", BASIC_ID).prop("text", t.into());
    match variant {
        Some(v) => n.prop("variant", v),
        None => n,
    }
}

fn thousands(x: f64) -> String {
    let s = format!("{x:.2}");
    let (int, frac) = s.split_once('.').unwrap();
    let (sign, digits) = int.strip_prefix('-').map(|d| ("-", d)).unwrap_or(("", int));
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{sign}{grouped}.{frac}")
}

fn rate_card(args: &Map<String, Value>) -> Result<UiNode, String> {
    let (src, dst) = (code(args, "currency_from")?, code(args, "currency_to")?);
    let amount = args.get("amount").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let data = get_json("latest", &src, &dst)?;
    let rate = data["rates"][&dst].as_f64().ok_or("rate missing from API response")?;
    let date = data["date"].as_str().unwrap_or("latest");
    Ok(UiNode::new("Card", BASIC_ID).child(
        "child",
        UiNode::new("Column", BASIC_ID).children(
            "children",
            vec![
                text(format!("### {} {src} = {} {dst}", thousands(amount), thousands(amount * rate)), None),
                text(format!("1 {src} = {rate:.5} {dst}"), None),
                UiNode::new("Divider", BASIC_ID),
                text(format!("ECB reference rate for {date} (Frankfurter API)"), Some("caption")),
            ],
        ),
    ))
}

fn rate_history_chart(args: &Map<String, Value>) -> Result<UiNode, String> {
    let (src, dst) = (code(args, "currency_from")?, code(args, "currency_to")?);
    let days = args.get("days").and_then(|v| v.as_f64()).unwrap_or(30.0).clamp(7.0, 365.0) as i64;
    let start = (chrono::Utc::now().date_naive() - chrono::Duration::days(days)).to_string();
    let data = get_json(&format!("{start}.."), &src, &dst)?;
    let rates = data["rates"].as_object().ok_or("rates missing from API response")?;
    let mut series: Vec<(&String, f64)> = rates
        .iter()
        .filter_map(|(d, r)| r[&dst].as_f64().map(|v| (d, v)))
        .collect();
    series.sort_by(|a, b| a.0.cmp(b.0));
    if series.is_empty() {
        return Err("no rates in range".into());
    }
    let labels: Vec<Value> = series.iter().map(|(d, _)| Value::from(&d[5..])).collect();
    let values: Vec<Value> = series.iter().map(|(_, v)| Value::from(*v)).collect();
    let lo = series.iter().map(|s| s.1).fold(f64::INFINITY, f64::min);
    let hi = series.iter().map(|s| s.1).fold(f64::NEG_INFINITY, f64::max);
    let last = series.last().unwrap().1;
    let title = format!(
        "{src}/{dst} ({} to {})",
        data["start_date"].as_str().unwrap_or(""),
        data["end_date"].as_str().unwrap_or("")
    );
    Ok(UiNode::new("Card", BASIC_ID).child(
        "child",
        UiNode::new("Column", BASIC_ID).children(
            "children",
            vec![
                text(format!("### {src} → {dst}, last {days} days"), None),
                UiNode::new("LineChart", CHARTS_ID)
                    .prop("title", title)
                    .prop("labels", labels)
                    .prop("values", values),
                text(format!("Low {lo:.5} · High {hi:.5} · Latest {last:.5}"), Some("caption")),
            ],
        ),
    ))
}

pub fn macros() -> Vec<Macro> {
    vec![
        Macro {
            name: "RateCard",
            description: "Card with the live exchange rate and a converted amount. The server looks up the rate, so do not pass rate figures.",
            params: vec![
                MacroParam { name: "currency_from", json_type: "string", description: "ISO 4217 code to convert from, such as \"USD\".", required: true },
                MacroParam { name: "currency_to", json_type: "string", description: "ISO 4217 code to convert to, such as \"EUR\".", required: true },
                MacroParam { name: "amount", json_type: "number", description: "Amount of currency_from to convert. Defaults to 1.", required: false },
            ],
            expand: rate_card,
        },
        Macro {
            name: "RateHistoryChart",
            description: "Card with a line chart of the exchange rate history. The server fetches the history, so do not pass dates or rate values.",
            params: vec![
                MacroParam { name: "currency_from", json_type: "string", description: "ISO 4217 code of the base currency, such as \"USD\".", required: true },
                MacroParam { name: "currency_to", json_type: "string", description: "ISO 4217 code of the quote currency, such as \"EUR\".", required: true },
                MacroParam { name: "days", json_type: "integer", description: "Number of calendar days of history, from 7 to 365. Defaults to 30.", required: false },
            ],
            expand: rate_history_chart,
        },
    ]
}

pub struct A2uiPipeline {
    /// Catalogs the renderer implements.
    pub base: Vec<Catalog>,
    /// Base catalogs plus the macro sidecar: what the model sees.
    pub compiler: Compiler,
    pub expander: MacroExpander,
}

pub static PIPELINE: LazyLock<A2uiPipeline> = LazyLock::new(|| {
    let base = vec![basic_catalog_v1_0(), charts_catalog()];
    let expander = MacroExpander::new(macros());
    let mut inference = base.clone();
    inference.push(expander.sidecar_catalog());
    A2uiPipeline { base, compiler: Compiler::new(inference), expander }
});

/// The A2UI part of the system prompt.
pub fn a2ui_prompt() -> String {
    express_prompt(PIPELINE.compiler.catalogs())
}

/// Compiles one Express block, expands its macros, and checks the result
/// against the renderer's catalogs.
pub fn compile_block(raw: &str) -> Result<Vec<Value>, crate::express::ExpressError> {
    let messages = PIPELINE.compiler.compile(raw, DEFAULT_SURFACE)?;
    let expanded = PIPELINE.expander.expand(messages)?;
    check_transport(&expanded, &PIPELINE.base)?;
    Ok(expanded)
}

/// Rewrites a model reply: each `<a2ui>` Express block becomes an
/// `<a2ui-json>` block holding the A2UI 1.0 messages, which is the form the
/// React UI and A2A clients read. A block that fails to compile is replaced
/// by a short note, and the error is returned for logging. Blocking: runs
/// macros that call the rates API.
pub fn render_reply(reply: &str) -> (String, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let mut errors = Vec::new();
    for part in crate::express::unwrap(reply) {
        match part {
            crate::express::RawPart::Text(t) => out.push(t),
            crate::express::RawPart::A2ui { raw, .. } => match compile_block(&raw) {
                Ok(msgs) => out.push(format!("<a2ui-json>{}</a2ui-json>", Value::Array(msgs))),
                Err(e) => {
                    errors.push(e.to_string());
                    out.push("(The view could not be rendered.)".into());
                }
            },
        }
    }
    (out.join("\n"), errors)
}

/// For tests and tools: parse a whole response with the inference catalogs.
pub fn parse(reply: &str) -> Result<Vec<Part>, crate::express::ExpressError> {
    parse_response(&PIPELINE.compiler, reply, true, DEFAULT_SURFACE)
}

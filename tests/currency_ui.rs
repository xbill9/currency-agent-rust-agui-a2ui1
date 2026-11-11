//! Checks for the agent's A2UI pipeline. `live_*` tests call the Frankfurter
//! API and are ignored by default: `cargo test --test currency_ui -- --ignored`.

use currency_agent::currency_ui::{a2ui_prompt, compile_block, render_reply};

#[test]
fn prompt_lists_macros_and_charts() {
    let p = a2ui_prompt();
    assert!(p.contains("• RateCard(currency_from (static), currency_to (static), amount? (static))"), "{p}");
    assert!(p.contains("• LineChart("));
    assert!(p.contains("https://a2ui.org/catalogs/macros"));
}

#[test]
fn plain_basic_layout_compiles_to_v1_0() {
    let msgs = compile_block(r#"root = Card(Column([Text("Hello"), Text("World", "caption")]))"#).unwrap();
    let create = &msgs[0]["createSurface"];
    assert_eq!(msgs[0]["version"], "1.0");
    assert!(create.get("catalogId").is_none(), "several catalogs: surface names none");
    let comps = create["components"].as_array().unwrap();
    assert_eq!(comps.len(), 4);
    assert!(comps.iter().all(|c| c["catalogId"] == "https://a2ui.org/specification/v1_0/catalogs/basic/catalog.json"));
}

#[test]
fn bad_currency_code_is_rejected_before_any_request() {
    let err = compile_block(r#"root = RateCard("dollars", "EUR")"#).unwrap_err();
    assert!(err.to_string().contains("ISO 4217"), "{err}");
}

#[test]
#[ignore]
fn live_rate_card_expands_into_basic_components() {
    let msgs = compile_block(r#"root = RateCard("usd", "GBP", 100)"#).unwrap();
    let comps = msgs[0]["createSurface"]["components"].as_array().unwrap();
    let names: Vec<&str> = comps.iter().map(|c| c["component"].as_str().unwrap()).collect();
    assert_eq!(names, ["Text", "Text", "Divider", "Text", "Column", "Card"]);
    assert_eq!(comps.last().unwrap()["id"], "root");
    println!("{}", serde_json::to_string_pretty(&msgs).unwrap());
}

#[test]
#[ignore]
fn live_reply_rewrites_blocks() {
    let (out, errs) = render_reply("Here it is.\n<a2ui>\nroot = RateHistoryChart(\"USD\", \"JPY\", 14)\n</a2ui>");
    assert!(errs.is_empty(), "{errs:?}");
    assert!(out.starts_with("Here it is.\n<a2ui-json>[{\"version\":\"1.0\""), "{out}");
    assert!(out.contains("\"component\":\"LineChart\""));
}

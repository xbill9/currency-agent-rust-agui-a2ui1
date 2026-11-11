#![allow(clippy::collapsible_if)]

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

const AGENT_CARD_JSON: &str = include_str!("agent_card.json");

/// The agent's role. The A2UI 1.0 Express contract, generated from the
/// catalogs (Basic 1.0, currency charts and the macro sidecar), is appended
/// at request time by `system_instruction()`.
const ROLE_INSTRUCTION: &str = r###"You are a specialized assistant for currency conversions and exchange rate visualization. Today's date is {TODAYS_DATE}.

Use the 'get_exchange_rate' tool when you need a rate in your text answer. If the user asks about anything other than currency conversion or exchange rates, politely say you can only help with currency questions.

Show results as A2UI in an <a2ui> Express block. Prefer the server-side macros: RateCard for a rate or a conversion, RateHistoryChart for trends or history over time. These macros look up the figures on the server, so pass only currency codes and the amount or day count; never invent or copy rate figures into them. Use Basic catalog components (Card, Column, Row, Text) or BarChart only for layouts the macros do not cover, such as comparing several currencies, and then only with rates you fetched with the tool. Keep any text outside the block to one short sentence."###;

fn system_instruction() -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    format!(
        "{}\n\n{}",
        ROLE_INSTRUCTION.replace("{TODAYS_DATE}", &today),
        currency_agent::currency_ui::a2ui_prompt()
    )
}

/// Earlier assistant turns hold rendered `<a2ui-json>` blocks; the model only
/// needs to know a view was shown.
fn strip_rendered_views(text: &str) -> String {
    let re = regex::Regex::new(r"(?s)<a2ui-json>.*?</a2ui-json>").unwrap();
    re.replace_all(text, "[A2UI view shown to the user]").into_owned()
}

/// Compiles the model's Express blocks to A2UI 1.0 JSON (runs the macros,
/// which call the rates API, off the async runtime).
async fn render_a2ui(reply: String) -> String {
    match tokio::task::spawn_blocking(move || currency_agent::currency_ui::render_reply(&reply)).await {
        Ok((text, errors)) => {
            for e in &errors {
                error!("A2UI block rejected: {}", e);
            }
            text
        }
        Err(e) => {
            error!("A2UI render task failed: {:?}", e);
            "(The view could not be rendered.)".to_string()
        }
    }
}

// A2A Protocol Models in Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct A2AJSONRPCRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct A2AJSONRPCResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

// Stateful Task structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Part {
    kind: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    parts: Vec<Part>,
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    context_id: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStatus {
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: String,
    #[serde(rename = "contextId")]
    context_id: String,
    status: TaskStatus,
    artifacts: Vec<Value>,
    history: Vec<Message>,
    metadata: HashMap<String, Value>,
    kind: String,
}

// Active task memory storage
struct AppState {
    tasks: RwLock<HashMap<String, Task>>,
    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let state = Arc::new(AppState {
        tasks: RwLock::new(HashMap::new()),
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/.well-known/agent-card.json", get(handle_agent_card))
        .route("/health", get(handle_health))
        .route("/", post(handle_rpc))
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(10000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Rust A2A Agent Server starting on 0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_agent_card() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    (StatusCode::OK, headers, AGENT_CARD_JSON).into_response()
}

async fn handle_health() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    (StatusCode::OK, headers, "{\"status\": \"ok\"}").into_response()
}

async fn handle_rpc(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(value): Json<Value>,
) -> Response {
    info!(
        "Incoming request raw payload size: {}",
        value.to_string().len()
    );

    if value.get("jsonrpc").is_some() {
        // Legacy JSON-RPC A2A flow
        info!("Handling via legacy JSON-RPC A2A flow");
        let body: A2AJSONRPCRequest = match serde_json::from_value(value.clone()) {
            Ok(b) => b,
            Err(e) => {
                error!(
                    "JSON-RPC Deserialization failed: {}. Raw payload snippet: {}",
                    e,
                    &value.to_string()[..std::cmp::min(1000, value.to_string().len())]
                );
                let id = value.get("id").cloned().unwrap_or(Value::Null);
                let err_response = A2AJSONRPCResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(serde_json::json!({
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    })),
                };
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("application/json"),
                );
                return (
                    StatusCode::BAD_REQUEST,
                    headers,
                    serde_json::to_string(&err_response).unwrap(),
                )
                    .into_response();
            }
        };

        info!("Incoming JSON-RPC request method: {}", body.method);

        let mut response = A2AJSONRPCResponse {
            jsonrpc: "2.0".to_string(),
            id: body.id.clone(),
            result: None,
            error: None,
        };

        match body.method.as_str() {
            "message/send" => {
                if let Some(ref params) = body.params {
                    if let Some(msg_val) = params.get("message") {
                        if let Ok(user_msg) = serde_json::from_value::<Message>(msg_val.clone()) {
                            let mut tasks = state.tasks.write().await;
                            let task_id = user_msg
                                .task_id
                                .clone()
                                .unwrap_or_else(|| Uuid::new_v4().to_string());
                            let context_id = user_msg
                                .context_id
                                .clone()
                                .unwrap_or_else(|| Uuid::new_v4().to_string());

                            // Retrieve or create task
                            let mut task = tasks.get(&task_id).cloned().unwrap_or_else(|| Task {
                                id: task_id.clone(),
                                context_id: context_id.clone(),
                                status: TaskStatus {
                                    state: "working".to_string(),
                                    message: None,
                                },
                                artifacts: Vec::new(),
                                history: Vec::new(),
                                metadata: HashMap::new(),
                                kind: "task".to_string(),
                            });

                            task.history.push(user_msg.clone());

                            // Direct call to Gemini
                            let reply_text =
                                execute_gemini_query(&state.http_client, &task.history).await;

                            let agent_msg = Message {
                                role: "agent".to_string(),
                                parts: vec![Part {
                                    kind: "text".to_string(),
                                    text: reply_text,
                                    data: None,
                                    metadata: None,
                                }],
                                message_id: Uuid::new_v4().to_string(),
                                context_id: Some(task.context_id.clone()),
                                task_id: Some(task.id.clone()),
                            };

                            task.history.push(agent_msg.clone());
                            task.status.state = "completed".to_string();
                            task.status.message = Some(agent_msg);

                            tasks.insert(task.id.clone(), task.clone());

                            response.result = Some(serde_json::to_value(&task).unwrap());
                        } else {
                            response.error = Some(serde_json::json!({
                                "code": -32602,
                                "message": "Invalid message parameter format"
                            }));
                        }
                    } else {
                        response.error = Some(serde_json::json!({
                            "code": -32602,
                            "message": "Missing message parameter"
                        }));
                    }
                } else {
                    response.error = Some(serde_json::json!({
                        "code": -32602,
                        "message": "Missing parameters"
                    }));
                }
            }
            "tasks/get" => {
                if let Some(ref params) = body.params {
                    if let Some(id_val) = params.get("id").and_then(|v| v.as_str()) {
                        let tasks = state.tasks.read().await;
                        if let Some(task) = tasks.get(id_val) {
                            response.result = Some(serde_json::to_value(task).unwrap());
                        } else {
                            response.error = Some(serde_json::json!({
                                "code": -32001,
                                "message": "Task not found"
                            }));
                        }
                    } else {
                        response.error = Some(serde_json::json!({
                            "code": -32602,
                            "message": "Missing task id"
                        }));
                    }
                } else {
                    response.error = Some(serde_json::json!({
                        "code": -32602,
                        "message": "Missing parameters"
                    }));
                }
            }
            _ => {
                response.error = Some(serde_json::json!({
                    "code": -32601,
                    "message": format!("Method not found: {}", body.method)
                }));
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );

        let serialized_response = serde_json::to_string(&response).unwrap();
        let sse_payload = format!("event: message\ndata: {}\n\n", serialized_response);

        (StatusCode::OK, headers, sse_payload).into_response()
    } else {
        // Modern AG-UI flow
        info!("Handling via modern AG-UI flow");

        let thread_id = value
            .get("threadId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let run_id = value
            .get("runId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let mut history = Vec::new();
        if let Some(msgs_arr) = value.get("messages").and_then(|v| v.as_array()) {
            for msg_val in msgs_arr {
                let role = msg_val
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user")
                    .to_string();
                let content = msg_val
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let id = msg_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let parts = vec![Part {
                    kind: "text".to_string(),
                    text: content,
                    data: None,
                    metadata: None,
                }];

                history.push(Message {
                    role,
                    parts,
                    message_id: id,
                    context_id: Some(thread_id.clone()),
                    task_id: Some(run_id.clone()),
                });
            }
        }

        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::convert::Infallible>>(100);
        let http_client = state.http_client.clone();
        let thread_id_clone = thread_id.clone();
        let run_id_clone = run_id.clone();

        tokio::spawn(async move {
            // 1. Send RunStarted
            let run_started = serde_json::json!({
                "type": "RUN_STARTED",
                "threadId": thread_id_clone,
                "runId": run_id_clone
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    run_started
                ))))
                .await;

            // 2. Call Gemini
            let reply_text = execute_gemini_query(&http_client, &history).await;

            // 3. Send TextMessageStart
            let assistant_message_id = Uuid::new_v4().to_string();
            let text_message_start = serde_json::json!({
                "type": "TEXT_MESSAGE_START",
                "messageId": assistant_message_id,
                "role": "assistant"
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    text_message_start
                ))))
                .await;

            // 4. Stream reply_text in chunks
            let chars: Vec<char> = reply_text.chars().collect();
            let chunk_size = 30;
            let mut idx = 0;
            while idx < chars.len() {
                let end = std::cmp::min(idx + chunk_size, chars.len());
                let chunk: String = chars[idx..end].iter().collect();
                idx = end;

                let text_message_content = serde_json::json!({
                    "type": "TEXT_MESSAGE_CONTENT",
                    "messageId": assistant_message_id,
                    "delta": chunk
                });
                let _ = tx
                    .send(Ok(axum::body::Bytes::from(format!(
                        "data: {}\n\n",
                        text_message_content
                    ))))
                    .await;

                tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
            }

            // 5. Send TextMessageEnd
            let text_message_end = serde_json::json!({
                "type": "TEXT_MESSAGE_END",
                "messageId": assistant_message_id
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    text_message_end
                ))))
                .await;

            // 6. Send RunFinished
            let run_finished = serde_json::json!({
                "type": "RUN_FINISHED",
                "threadId": thread_id_clone,
                "runId": run_id_clone
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    run_finished
                ))))
                .await;
        });

        let body_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );

        (
            StatusCode::OK,
            headers,
            axum::body::Body::from_stream(body_stream),
        )
            .into_response()
    }
}

// Direct interface with Gemini API and tool routing
async fn execute_gemini_query(client: &reqwest::Client, history: &[Message]) -> String {
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .unwrap_or_default();

    if api_key.is_empty() {
        return "Error: Gemini API Key is missing in environment.".to_string();
    }

    let model = std::env::var("GENAI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    // Build model content structures from message history
    let mut contents = Vec::new();
    for msg in history {
        let role = match msg.role.as_str() {
            "user" => "user",
            "agent" | "model" => "model",
            _ => "user",
        };

        let mut parts_vals = Vec::new();
        for part in &msg.parts {
            let text = if role == "model" {
                strip_rendered_views(&part.text)
            } else {
                part.text.clone()
            };
            parts_vals.push(serde_json::json!({
                "text": text
            }));
        }

        contents.push(serde_json::json!({
            "role": role,
            "parts": parts_vals
        }));
    }

    let dynamic_system_instruction = system_instruction();

    let mut turn_count = 0;
    const MAX_TURNS: usize = 15;

    while turn_count < MAX_TURNS {
        turn_count += 1;
        info!(
            "Executing Gemini query loop turn {}/{}...",
            turn_count, MAX_TURNS
        );

        // Include system instruction and declaration for the get_exchange_rate tool
        let payload = serde_json::json!({
            "contents": contents,
            "systemInstruction": {
                "parts": [{ "text": dynamic_system_instruction }]
            },
            "tools": [{
                "functionDeclarations": [
                    {
                        "name": "get_exchange_rate",
                        "description": "Use this to get the exchange rate for a specific date or 'latest'. To get historical trend data (e.g. for building a LineChart), call this tool multiple times sequentially or in parallel with different historical dates.",
                        "parameters": {
                            "type": "OBJECT",
                            "properties": {
                                "currency_from": {
                                    "type": "STRING",
                                    "description": "The currency to convert from (e.g., \"USD\")."
                                },
                                "currency_to": {
                                    "type": "STRING",
                                    "description": "The currency to convert to (e.g., \"EUR\")."
                                },
                                "currency_date": {
                                    "type": "STRING",
                                    "description": "The date for the exchange rate or \"latest\". Defaults to \"latest\"."
                                }
                            },
                            "required": ["currency_from", "currency_to"]
                        }
                    }
                ]
            }]
        });

        info!(
            "Sending request to Gemini generateContent (turn {})...",
            turn_count
        );
        let res = client.post(&url).json(&payload).send().await;

        match res {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                        info!("Gemini response (turn {}): {}", turn_count, data);
                        if let Some(candidates) = data.get("candidates").and_then(|c| c.as_array())
                        {
                            if let Some(first) = candidates.first() {
                                if let Some(content) = first.get("content") {
                                    if let Some(parts) =
                                        content.get("parts").and_then(|p| p.as_array())
                                    {
                                        let mut has_function_call = false;
                                        let mut function_responses = Vec::new();

                                        for part in parts {
                                            if let Some(func_call) = part.get("functionCall") {
                                                has_function_call = true;
                                                let func_name = func_call
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or_default();
                                                if func_name == "get_exchange_rate" {
                                                    let args = func_call
                                                        .get("args")
                                                        .cloned()
                                                        .unwrap_or(serde_json::json!({}));
                                                    info!(
                                                        "Gemini requested function call (turn {}): get_exchange_rate with args {:?}",
                                                        turn_count, args
                                                    );

                                                    // Invoke the local MCP toolset
                                                    let mcp_result = call_mcp_tool(
                                                        client,
                                                        "get_exchange_rate",
                                                        args.clone(),
                                                    )
                                                    .await;

                                                    // Construct functionResponse part
                                                    function_responses.push(serde_json::json!({
                                                        "functionResponse": {
                                                            "name": "get_exchange_rate",
                                                            "response": {
                                                                "name": "get_exchange_rate",
                                                                "content": mcp_result
                                                            }
                                                        }
                                                    }));
                                                }
                                            }
                                        }

                                        if has_function_call {
                                            if function_responses.is_empty() {
                                                return "Error: Unsupported tool call requested."
                                                    .to_string();
                                            }

                                            // Push model turn and function responses turn to contents
                                            contents.push(content.clone());
                                            contents.push(serde_json::json!({
                                                "role": "user",
                                                "parts": function_responses
                                            }));

                                            // Go back to the loop to let the model process the function responses
                                            continue;
                                        }

                                        // No function calls, look for text
                                        let mut final_text = String::new();
                                        for part in parts {
                                            if let Some(text) =
                                                part.get("text").and_then(|t| t.as_str())
                                            {
                                                final_text.push_str(text);
                                            }
                                        }

                                        if !final_text.is_empty() {
                                            return render_a2ui(final_text).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let status = resp.status();
                    let err_body = resp.text().await.unwrap_or_default();
                    error!(
                        "Gemini API responded with error (turn {}): {:?}",
                        turn_count, err_body
                    );
                    return format!(
                        "Error: Gemini API responded with error status {}: {}",
                        status, err_body
                    );
                }
            }
            Err(err) => {
                error!(
                    "HTTP request to Gemini API failed (turn {}): {:?}",
                    turn_count, err
                );
                return format!("Error: HTTP request to Gemini API failed: {:?}", err);
            }
        }
    }

    "Error: Exceeded maximum tool execution turns.".to_string()
}

// Call local MCP Server
async fn call_mcp_tool(client: &reqwest::Client, name: &str, arguments: Value) -> Value {
    let mcp_url =
        std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/mcp".to_string());

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    });

    info!("Calling MCP server tools/call at {}...", mcp_url);
    let res = client.post(&mcp_url).json(&payload).send().await;

    match res {
        Ok(resp) => {
            let status = resp.status();
            info!("MCP server responded with status: {}", status);
            if status.is_success() {
                match resp.text().await {
                    Ok(body_text) => {
                        info!("MCP server raw response body: {}", body_text);
                        let json_str = if body_text.contains("data: ") {
                            body_text
                                .lines()
                                .find(|line| line.starts_with("data: "))
                                .map(|line| line["data: ".len()..].trim())
                                .unwrap_or(&body_text)
                        } else {
                            &body_text
                        };

                        match serde_json::from_str::<Value>(json_str) {
                            Ok(mcp_response) => {
                                info!("MCP server parsed JSON response: {:?}", mcp_response);
                                if let Some(result) = mcp_response.get("result") {
                                    return result.clone();
                                } else {
                                    error!(
                                        "MCP response did not contain 'result' field: {:?}",
                                        mcp_response
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to parse MCP response JSON: {:?}. Raw string: {}",
                                    e, json_str
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read MCP response body text: {:?}", e);
                    }
                }
            } else {
                error!(
                    "MCP server returned error status: {}, body: {:?}",
                    status,
                    resp.text().await
                );
            }
        }
        Err(err) => {
            error!("MCP tool invocation HTTP request failed: {:?}", err);
        }
    }

    serde_json::json!({ "error": "MCP server call failed" })
}

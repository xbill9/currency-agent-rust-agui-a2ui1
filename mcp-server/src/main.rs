#![allow(clippy::collapsible_if)]

use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct GenericRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CallParams {
    name: String,
    arguments: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ExchangeRateArgs {
    #[serde(default = "default_from")]
    currency_from: String,
    #[serde(default = "default_to")]
    currency_to: String,
    #[serde(default = "default_date")]
    currency_date: String,
}

fn default_from() -> String {
    "USD".to_string()
}
fn default_to() -> String {
    "EUR".to_string()
}
fn default_date() -> String {
    "latest".to_string()
}

// Active session stream management
struct Session {
    sender: Option<mpsc::UnboundedSender<axum::response::sse::Event>>,
}

struct AppState {
    sessions: RwLock<HashMap<String, Session>>,
    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        sessions: RwLock::new(HashMap::new()),
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/mcp", get(handle_mcp_get).post(handle_mcp_post))
        .with_state(state);

    let port = std::env::var("MCP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 MCP server starting on 0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handle GET /mcp
async fn handle_mcp_get(
    headers: HeaderMap,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Response {
    // Validate Accept header
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !accept.is_empty() && !accept.contains("text/event-stream") && !accept.contains("*/*") {
        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "server-error",
            "error": {
                "code": -32600,
                "message": "Not Acceptable: Client must accept text/event-stream"
            }
        });
        return (
            StatusCode::NOT_ACCEPTABLE,
            response_headers,
            body.to_string(),
        )
            .into_response();
    }

    // Extract session ID
    let session_id = match headers.get("mcp-session-id").and_then(|h| h.to_str().ok()) {
        Some(sid) => sid.to_string(),
        None => {
            let mut response_headers = HeaderMap::new();
            response_headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": "server-error",
                "error": {
                    "code": -32600,
                    "message": "Bad Request: Missing session ID"
                }
            });
            return (StatusCode::BAD_REQUEST, response_headers, body.to_string()).into_response();
        }
    };

    let (tx, rx) = mpsc::unbounded_channel();

    // Register active session GET stream
    state
        .sessions
        .write()
        .await
        .insert(session_id.clone(), Session { sender: Some(tx) });

    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(Ok::<_, Infallible>);

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "mcp-session-id",
        HeaderValue::from_str(&session_id).unwrap(),
    );

    axum::response::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

// Handle POST /mcp
async fn handle_mcp_post(
    headers: HeaderMap,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    // Determine/Generate Session ID
    let session_id = headers
        .get("mcp-session-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string().replace("-", ""));

    // Check Accept header
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let is_acceptable = accept.is_empty()
        || accept.contains("*/*")
        || accept.contains("text/event-stream")
        || accept.contains("application/json");

    if !is_acceptable {
        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        response_headers.insert(
            "mcp-session-id",
            HeaderValue::from_str(&session_id).unwrap(),
        );
        let err_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "server-error",
            "error": {
                "code": -32600,
                "message": "Not Acceptable: Client must accept both application/json and text/event-stream"
            }
        });
        return (
            StatusCode::NOT_ACCEPTABLE,
            response_headers,
            err_body.to_string(),
        )
            .into_response();
    }

    // Parse request
    let req: GenericRequest = match serde_json::from_value(body.clone()) {
        Ok(r) => r,
        Err(_) => {
            let mut response_headers = HeaderMap::new();
            response_headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            let err_body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": "server-error",
                "error": {
                    "code": -32700,
                    "message": "Parse error"
                }
            });
            return (
                StatusCode::BAD_REQUEST,
                response_headers,
                err_body.to_string(),
            )
                .into_response();
        }
    };

    let response_val = match req.method.as_str() {
        "initialize" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": req.id.unwrap_or(serde_json::Value::Null),
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "experimental": {},
                        "logging": {},
                        "prompts": { "listChanged": true },
                        "resources": { "subscribe": false, "listChanged": true },
                        "tools": { "listChanged": true },
                        "extensions": { "io.modelcontextprotocol/ui": {} }
                    },
                    "serverInfo": {
                        "name": "Currency MCP Server 💵",
                        "version": "3.4.2"
                    }
                }
            })
        }
        "initialized" | "notifications/initialized" => {
            // Notification, no response needed
            return Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("mcp-session-id", &session_id)
                .body(Body::empty())
                .unwrap();
        }
        "tools/list" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": req.id.unwrap_or(serde_json::Value::Null),
                "result": {
                    "tools": [
                        {
                            "name": "get_exchange_rate",
                            "description": "Use this to get current exchange rate.",
                            "inputSchema": {
                                "additionalProperties": false,
                                "properties": {
                                    "currency_from": {
                                        "default": "USD",
                                        "type": "string",
                                        "description": "The currency to convert from (e.g., \"USD\")."
                                    },
                                    "currency_to": {
                                        "default": "EUR",
                                        "type": "string",
                                        "description": "The currency to convert to (e.g., \"EUR\")."
                                    },
                                    "currency_date": {
                                        "default": "latest",
                                        "type": "string",
                                        "description": "The date for the exchange rate or \"latest\". Defaults to \"latest\"."
                                    }
                                },
                                "type": "object"
                            },
                            "_meta": {
                                "fastmcp": {
                                    "tags": []
                                }
                            }
                        }
                    ]
                }
            })
        }
        "tools/call" => {
            let call_params: CallParams =
                match req.params.and_then(|p| serde_json::from_value(p).ok()) {
                    Some(p) => p,
                    None => {
                        let mut response_headers = HeaderMap::new();
                        response_headers.insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/json"),
                        );
                        let err_body = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": req.id.unwrap_or(serde_json::Value::Null),
                            "error": {
                                "code": -32602,
                                "message": "Invalid params"
                            }
                        });
                        return (
                            StatusCode::BAD_REQUEST,
                            response_headers,
                            err_body.to_string(),
                        )
                            .into_response();
                    }
                };

            if call_params.name == "get_exchange_rate" {
                let args: ExchangeRateArgs = match call_params.arguments {
                    Some(args_val) => {
                        serde_json::from_value(args_val).unwrap_or_else(|_| ExchangeRateArgs {
                            currency_from: default_from(),
                            currency_to: default_to(),
                            currency_date: default_date(),
                        })
                    }
                    None => ExchangeRateArgs {
                        currency_from: default_from(),
                        currency_to: default_to(),
                        currency_date: default_date(),
                    },
                };

                info!(
                    "--- 🛠️ Tool: get_exchange_rate called for converting {} to {} ---",
                    args.currency_from, args.currency_to
                );

                let url = format!("https://api.frankfurter.dev/v1/{}", args.currency_date);
                let res = state
                    .http_client
                    .get(&url)
                    .query(&[("from", &args.currency_from), ("to", &args.currency_to)])
                    .send()
                    .await;

                let result_json = match res {
                    Ok(resp) => {
                        let status = resp.status();
                        let url_str = resp.url().clone();
                        if status.is_success() {
                            match resp.json::<serde_json::Value>().await {
                                Ok(data) => {
                                    if data.get("rates").is_none() {
                                        error!("❌ rates not found in response: {}", data);
                                        serde_json::json!({ "error": "Invalid API response format." })
                                    } else {
                                        info!("✅ API response: {}", data);
                                        data
                                    }
                                }
                                Err(_) => {
                                    error!("❌ Invalid JSON response from API");
                                    serde_json::json!({ "error": "Invalid JSON response from API." })
                                }
                            }
                        } else {
                            let err_msg = format!(
                                "API request failed: Client error '{} {}' for url '{}'\nFor more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{}",
                                status.as_u16(),
                                status.canonical_reason().unwrap_or(""),
                                url_str,
                                status.as_u16()
                            );
                            error!("❌ {}", err_msg);
                            serde_json::json!({ "error": err_msg })
                        }
                    }
                    Err(err) => {
                        let err_msg = format!("API request failed: {}", err);
                        error!("❌ {}", err_msg);
                        serde_json::json!({ "error": err_msg })
                    }
                };

                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req.id.unwrap_or(serde_json::Value::Null),
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string(&result_json).unwrap()
                            }
                        ],
                        "structuredContent": result_json,
                        "isError": false
                    }
                })
            } else {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req.id.unwrap_or(serde_json::Value::Null),
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", call_params.name)
                    }
                })
            }
        }
        _ => {
            if req.id.is_none() || req.id.as_ref().unwrap().is_null() {
                return Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .header("mcp-session-id", &session_id)
                    .body(Body::empty())
                    .unwrap();
            }
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": req.id.unwrap_or(serde_json::Value::Null),
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", req.method)
                }
            })
        }
    };

    // Serialize result to push to SSE
    let payload = serde_json::to_string(&response_val).unwrap();
    let sse_msg = format!("event: message\ndata: {}\n\n", payload);

    // Optionally push to active GET stream if there is one registered for this session
    let sessions = state.sessions.read().await;
    if let Some(session) = sessions.get(&session_id) {
        if let Some(ref sender) = session.sender {
            let event = axum::response::sse::Event::default()
                .event("message")
                .data(payload.clone());
            let _ = sender.send(event);
        }
    }

    // Build immediate SSE response chunk for the POST request
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache, no-transform")
        .header(header::CONNECTION, "keep-alive")
        .header("mcp-session-id", &session_id)
        .header("x-accel-buffering", "no")
        .body(Body::from(sse_msg))
        .unwrap()
}

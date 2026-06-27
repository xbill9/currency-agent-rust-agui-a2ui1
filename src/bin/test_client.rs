use serde_json::json;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent_url = env::var("AGENT_URL").unwrap_or_else(|_| "http://127.0.0.1:10000".to_string());
    println!("--- 🔄 Connecting to agent at {}... ---", agent_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // 1. Fetch Agent Card (.well-known/agent-card.json)
    let card_url = format!("{}/.well-known/agent-card.json", agent_url);
    match client.get(&card_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let card_json: serde_json::Value = resp.json().await?;
                println!("--- ✅ Connection successful. Agent Card retrieved: ---");
                println!(
                    "Agent Name: {}\n",
                    card_json
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unknown")
                );
            } else {
                eprintln!(
                    "--- ❌ Failed to fetch agent card: HTTP {} ---",
                    resp.status()
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("--- ❌ Connection failed: {} ---", e);
            eprintln!("Ensure the agent server is running.");
            std::process::exit(1);
        }
    }

    // 2. Run Single Turn Test
    run_single_turn_test(&client, &agent_url).await?;

    // 3. Run Multi-Turn Test
    run_multi_turn_test(&client, &agent_url).await?;

    Ok(())
}

fn create_send_message_payload(
    text: &str,
    task_id: Option<&str>,
    context_id: Option<&str>,
) -> serde_json::Value {
    let mut message = json!({
        "role": "user",
        "parts": [{"kind": "text", "text": text}],
        "messageId": Uuid::new_v4().simple().to_string(),
    });

    if let Some(tid) = task_id {
        message["taskId"] = json!(tid);
    }
    if let Some(cid) = context_id {
        message["contextId"] = json!(cid);
    }

    message
}

async fn run_single_turn_test(
    client: &reqwest::Client,
    agent_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- ✉️  Single Turn Request ---");

    let message_payload = create_send_message_payload("how much is 100 USD in CAD?", None, None);
    let rpc_request = json!({
        "jsonrpc": "2.0",
        "id": Uuid::new_v4().to_string(),
        "method": "message/send",
        "params": {
            "message": message_payload
        }
    });

    let resp = client.post(agent_url).json(&rpc_request).send().await?;
    let resp_text = resp.text().await?;
    let json_str = if resp_text.contains("data: ") {
        resp_text
            .lines()
            .find(|line| line.starts_with("data: "))
            .map(|line| line["data: ".len()..].trim())
            .unwrap_or(&resp_text)
    } else {
        &resp_text
    };
    let resp_json: serde_json::Value = serde_json::from_str(json_str)?;

    println!("--- 📥 Single Turn Request Response ---");
    println!("{}\n", serde_json::to_string_pretty(&resp_json)?);

    // Extract Task ID to run Query Task
    if let Some(task_id) = resp_json
        .get("result")
        .and_then(|r| r.get("id"))
        .and_then(|id| id.as_str())
    {
        println!("--- ❔ Query Task ---");
        let query_request = json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": "tasks/get",
            "params": {
                "id": task_id
            }
        });

        let query_resp = client.post(agent_url).json(&query_request).send().await?;
        let query_text = query_resp.text().await?;
        let query_json_str = if query_text.contains("data: ") {
            query_text
                .lines()
                .find(|line| line.starts_with("data: "))
                .map(|line| line["data: ".len()..].trim())
                .unwrap_or(&query_text)
        } else {
            &query_text
        };
        let query_json: serde_json::Value = serde_json::from_str(query_json_str)?;

        println!("--- 📥 Query Task Response ---");
        println!("{}\n", serde_json::to_string_pretty(&query_json)?);
    }

    Ok(())
}

async fn run_multi_turn_test(
    client: &reqwest::Client,
    agent_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 📝 Multi-Turn Request ---");

    // --- First Turn ---
    let first_payload = create_send_message_payload("how much is 100 USD?", None, None);
    let rpc_request1 = json!({
        "jsonrpc": "2.0",
        "id": Uuid::new_v4().to_string(),
        "method": "message/send",
        "params": {
            "message": first_payload
        }
    });

    let resp1 = client.post(agent_url).json(&rpc_request1).send().await?;
    let resp1_text = resp1.text().await?;
    let json_str1 = if resp1_text.contains("data: ") {
        resp1_text
            .lines()
            .find(|line| line.starts_with("data: "))
            .map(|line| line["data: ".len()..].trim())
            .unwrap_or(&resp1_text)
    } else {
        &resp1_text
    };
    let resp1_json: serde_json::Value = serde_json::from_str(json_str1)?;

    println!("--- 📥 Multi-Turn: First Turn Response ---");
    println!("{}\n", serde_json::to_string_pretty(&resp1_json)?);

    let result = resp1_json.get("result");
    let context_id = result
        .and_then(|r| r.get("contextId"))
        .and_then(|c| c.as_str());
    let task_id = result.and_then(|r| r.get("id")).and_then(|id| id.as_str());
    let state = result
        .and_then(|r| r.get("status"))
        .and_then(|s| s.get("state"))
        .and_then(|s| s.as_str());

    if let (Some(cid), Some(tid), Some(st)) = (context_id, task_id, state) {
        if st == "input_required" {
            println!("--- 📝 Multi-Turn: Second Turn (Input Required) ---");
            let second_payload = create_send_message_payload("in GBP", Some(tid), Some(cid));
            let rpc_request2 = json!({
                "jsonrpc": "2.0",
                "id": Uuid::new_v4().to_string(),
                "method": "message/send",
                "params": {
                    "message": second_payload
                }
            });

            let resp2 = client.post(agent_url).json(&rpc_request2).send().await?;
            let resp2_text = resp2.text().await?;
            let json_str2 = if resp2_text.contains("data: ") {
                resp2_text
                    .lines()
                    .find(|line| line.starts_with("data: "))
                    .map(|line| line["data: ".len()..].trim())
                    .unwrap_or(&resp2_text)
            } else {
                &resp2_text
            };
            let resp2_json: serde_json::Value = serde_json::from_str(json_str2)?;

            println!("--- 📥 Multi-Turn: Second Turn Response ---");
            println!("{}\n", serde_json::to_string_pretty(&resp2_json)?);
        } else {
            println!(
                "--- 🚀 First turn completed, no further input required for this test case. ---"
            );
        }
    } else {
        println!(
            "--- ⚠️ Warning: Could not extract task ID, context ID, or state from first turn response. ---"
        );
    }

    Ok(())
}

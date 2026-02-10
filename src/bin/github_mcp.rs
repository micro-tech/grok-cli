use anyhow::Result;
use reqwest::header::USER_AGENT;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = stdout();

    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse JSON: {}", e);
                continue;
            }
        };

        if let Err(e) = handle_request(&request, &mut stdout).await {
            eprintln!("Error handling request: {}", e);
        }
    }
    Ok(())
}

async fn handle_request(request: &Value, stdout: &mut tokio::io::Stdout) -> Result<()> {
    let id = request.get("id");
    let method = request.get("method").and_then(|v| v.as_str());

    match method {
        Some("initialize") => {
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "0.1.0",
                    "capabilities": {},
                    "serverInfo": {
                        "name": "github-mcp",
                        "version": "0.1.0"
                    }
                }
            });
            send_response(stdout, &response).await?;
        }
        Some("notifications/initialized") => {
            // No response needed for notifications
        }
        Some("tools/list") => {
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "search_repos",
                            "description": "Search for public GitHub repositories",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string", "description": "The search query (e.g. 'rust cli')" }
                                },
                                "required": ["query"]
                            }
                        }
                    ]
                }
            });
            send_response(stdout, &response).await?;
        }
        Some("tools/call") => {
            let params = request
                .get("params")
                .ok_or_else(|| anyhow::anyhow!("Missing params"))?;
            let name = params
                .get("name")
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
            let default_args = json!({});
            let args = params.get("arguments").unwrap_or(&default_args);

            if name == "search_repos" {
                let query = args.get("query").and_then(|s| s.as_str()).unwrap_or("rust");

                let client = reqwest::Client::new();
                let url = format!(
                    "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page=5",
                    query
                );

                let api_response = client
                    .get(&url)
                    .header(USER_AGENT, "grok-cli-mcp")
                    .send()
                    .await?
                    .json::<Value>()
                    .await?;

                let mut text = String::new();
                if let Some(items) = api_response.get("items").and_then(|v| v.as_array()) {
                    for item in items {
                        let full_name = item
                            .get("full_name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("unknown");
                        let desc = item
                            .get("description")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        let stars = item
                            .get("stargazers_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        text.push_str(&format!("- **{}** (★ {}): {}\n", full_name, stars, desc));
                    }
                } else {
                    text = "No repositories found or API rate limit exceeded.".to_string();
                }

                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": text
                            }
                        ],
                        "isError": false
                    }
                });
                send_response(stdout, &response).await?;
            } else {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Tool not found: {}", name)
                    }
                });
                send_response(stdout, &response).await?;
            }
        }
        _ => {
            // Ignore unknown methods or notifications
        }
    }

    Ok(())
}

async fn send_response(stdout: &mut tokio::io::Stdout, response: &Value) -> Result<()> {
    let json_str = serde_json::to_string(response)?;
    stdout.write_all(json_str.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}

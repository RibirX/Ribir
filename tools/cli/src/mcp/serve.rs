use std::sync::atomic::{AtomicU16, Ordering};

use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Port being used by the current MCP session (set at startup)
static CURRENT_PORT: AtomicU16 = AtomicU16::new(2333);

pub async fn mcp_serve(port: u16) -> Result<()> {
  // Store the port for use in fallback messages
  CURRENT_PORT.store(port, Ordering::Relaxed);

  log::info!("Starting MCP stdio server, connecting to debug server on port {}", port);

  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;

  let mcp_url = format!("http://127.0.0.1:{}/mcp/message", port);

  let stdin = tokio::io::stdin();
  let mut stdout = tokio::io::stdout();
  let mut reader = BufReader::new(stdin);

  while let Some(message) = read_message(&mut reader).await? {
    let request: Value = match serde_json::from_slice(&message) {
      Ok(r) => r,
      Err(e) => {
        log::error!("Failed to parse JSON-RPC request: {}", e);
        continue;
      }
    };

    log::debug!("MCP request: method={}", request["method"]);

    if is_notification(&request) {
      continue;
    }

    let response = handle_request(&client, &mcp_url, request).await;

    let response_str = serde_json::to_string(&response)?;
    write_response(&mut stdout, response_str.as_bytes()).await?;
  }

  Ok(())
}

async fn read_message(reader: &mut BufReader<tokio::io::Stdin>) -> Result<Option<Vec<u8>>> {
  loop {
    let mut line = Vec::new();
    let bytes = reader.read_until(b'\n', &mut line).await?;
    if bytes == 0 {
      return Ok(None);
    }

    let trimmed = trim_line(&line);
    if trimmed.is_empty() {
      continue;
    }

    return Ok(Some(trimmed.to_vec()));
  }
}

fn trim_line(line: &[u8]) -> &[u8] {
  let mut end = line.len();
  while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
    end -= 1;
  }
  &line[..end]
}

async fn write_response(stdout: &mut tokio::io::Stdout, payload: &[u8]) -> Result<()> {
  stdout.write_all(payload).await?;
  stdout.write_all(b"\n").await?;
  stdout.flush().await?;
  Ok(())
}

fn is_notification(request: &Value) -> bool {
  let id = request.get("id");
  if id.is_none() || id.is_some_and(|v| v.is_null()) {
    return true;
  }
  request["method"] == "notifications/initialized"
}

async fn handle_request(client: &reqwest::Client, url: &str, request: Value) -> Value {
  let resp = match client.post(url).json(&request).send().await {
    Ok(resp) => resp,
    Err(e) if is_connection_error(&e) => return handle_fallback(request),
    Err(e) => {
      log::error!("HTTP request failed: {}", e);
      return json_rpc_error(request["id"].clone(), -32603, format!("Request failed: {}", e));
    }
  };

  let mut json = match resp.json::<Value>().await {
    Ok(json) => json,
    Err(e) => {
      log::error!("Failed to parse server response: {}", e);
      return json_rpc_error(request["id"].clone(), -32603, format!("Parse error: {}", e));
    }
  };

  if request["method"] == "initialize" {
    if let Some(server_version) = json["result"]["serverInfo"]["version"].as_str() {
      let server_version = server_version.to_string();
      check_version_compatibility(&mut json, &server_version);
    }
  }

  json
}

fn handle_fallback(request: Value) -> Value {
  let method = request["method"].as_str().unwrap_or("");
  let id = request["id"].clone();

  match method {
    "initialize" => json_rpc_success(id, super::schema::get_fallback_init_result()),
    "tools/list" => json_rpc_success(id, json!({ "tools": super::schema::get_tools() })),
    "resources/list" => {
      json_rpc_success(id, json!({ "resources": super::schema::get_resources() }))
    }
    _ => {
      let port = CURRENT_PORT.load(Ordering::Relaxed);
      let hint = format!(
        "Cannot connect to Ribir debug server at port {}. Make sure your Ribir app is running \
         with the debug feature enabled: cargo run --features debug. Use 'ribir mcp list' to see \
         active debug sessions.",
        port
      );
      json_rpc_error(id, -32000, hint)
    }
  }
}

fn check_version_compatibility(response: &mut Value, server_version: &str) {
  let cli_version = super::schema::MCP_VERSION;
  if server_version == cli_version {
    return;
  }

  let warning = format!(
    "Version mismatch detected: CLI MCP version is {}, but debug server is running {}. Consider \
     upgrading to ensure compatibility.",
    cli_version, server_version
  );

  if let Some(result) = response["result"].as_object_mut() {
    result.insert("_warning".to_string(), Value::String(warning));
  }
}

fn is_connection_error(err: &reqwest::Error) -> bool { err.is_connect() || err.is_timeout() }

fn json_rpc_success(id: Value, result: Value) -> Value {
  json!({
      "jsonrpc": "2.0",
      "id": id,
      "result": result
  })
}

fn json_rpc_error(id: Value, code: i32, message: String) -> Value {
  json!({
      "jsonrpc": "2.0",
      "id": id,
      "error": {
          "code": code,
          "message": message
      }
  })
}

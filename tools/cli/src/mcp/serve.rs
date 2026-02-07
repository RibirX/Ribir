use std::{
  process::Stdio,
  sync::atomic::{AtomicU8, AtomicU16, Ordering},
  time::Duration,
};

use anyhow::Result;
use serde_json::{Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
  process::Child,
  sync::Mutex,
};

/// Port being used by the current MCP session (set at startup)
static CURRENT_PORT: AtomicU16 = AtomicU16::new(2333);
static CURRENT_PORT_SOURCE: AtomicU8 = AtomicU8::new(PortSource::Unknown as u8);
const MAX_RETRIES: usize = 600; // 60 seconds total
const STDERR_TIMEOUT_SECS: u64 = 5;
const STDERR_MAX_SIZE: usize = 1024 * 1024; // 1MB max
const READY_CHECK_RETRIES: usize = 50; // 5 seconds total
const READY_CHECK_INTERVAL_MS: u64 = 100;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortSource {
  Unknown = 0,
  ExplicitArg = 1,
  AutoDiscovered = 2,
  StartApp = 3,
}

impl PortSource {
  pub(crate) fn as_label(self) -> &'static str {
    match self {
      PortSource::Unknown => "unknown",
      PortSource::ExplicitArg => "--port",
      PortSource::AutoDiscovered => "auto-discovered",
      PortSource::StartApp => "start_app",
    }
  }
}

fn load_port_source() -> PortSource {
  match CURRENT_PORT_SOURCE.load(Ordering::Relaxed) {
    1 => PortSource::ExplicitArg,
    2 => PortSource::AutoDiscovered,
    3 => PortSource::StartApp,
    _ => PortSource::Unknown,
  }
}

// Use OnceCell for the global child process handle to avoid lazy_static macro
// complexities
static APP_CHILD: tokio::sync::OnceCell<Mutex<Option<Child>>> = tokio::sync::OnceCell::const_new();

async fn get_app_child() -> &'static Mutex<Option<Child>> {
  APP_CHILD
    .get_or_init(|| async { Mutex::new(None) })
    .await
}

pub async fn mcp_serve(port: u16, port_source: PortSource) -> Result<()> {
  CURRENT_PORT.store(port, Ordering::Relaxed);
  CURRENT_PORT_SOURCE.store(port_source as u8, Ordering::Relaxed);

  log::info!(
    "Starting MCP stdio server, connecting to debug server on port {} (source: {})",
    port,
    port_source.as_label()
  );

  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(10))
    .build()?;

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

    if is_notification(&request) {
      continue;
    }

    let response = handle_request(&client, request).await;
    let response_str = serde_json::to_string(&response)?;
    write_response(&mut stdout, response_str.as_bytes()).await?;
  }

  // Cleanup: ensure the app is killed when MCP server exits
  stop_current_app().await;

  Ok(())
}

async fn stop_current_app() {
  let mut lock = get_app_child().await.lock().await;
  if let Some(mut child) = lock.take() {
    log::info!("Cleaning up: killing the Ribir application process.");
    let _ = child.kill().await;
  }
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

async fn handle_request(client: &reqwest::Client, request: Value) -> Value {
  let method = request["method"].as_str().unwrap_or("");
  if method == "tools/call" {
    let tool_name = request["params"]
      .get("name")
      .and_then(|n| n.as_str());
    match tool_name {
      Some("start_app") => return handle_start_app(request).await,
      Some("stop_app") => return handle_stop_app(request).await,
      _ => {}
    }
  }

  let port = CURRENT_PORT.load(Ordering::Relaxed);
  let port_source = load_port_source();
  let url = format!("http://127.0.0.1:{}/mcp/message", port);

  match client.post(&url).json(&request).send().await {
    Ok(resp) => match resp.json::<Value>().await {
      Ok(mut json) => {
        if method == "initialize" {
          let version = json["result"]["serverInfo"]["version"]
            .as_str()
            .map(|s| s.to_string());
          if let Some(v) = version {
            check_version_compatibility(&mut json, &v);
          }
        }
        json
      }
      Err(e) => {
        log::error!("Failed to parse response: {}", e);
        json_rpc_error(request["id"].clone(), -32603, format!("Parse error: {}", e))
      }
    },
    Err(e) if is_connection_error(&e) => handle_fallback(request, port, port_source).await,
    Err(e) => {
      log::error!("Request failed: {}", e);
      json_rpc_error(request["id"].clone(), -32603, format!("Request failed: {}", e))
    }
  }
}

async fn handle_fallback(request: Value, port: u16, port_source: PortSource) -> Value {
  let method = request["method"].as_str().unwrap_or("");
  let id = request["id"].clone();

  match method {
    "initialize" => json_rpc_success(id, super::schema::get_fallback_init_result()),
    "tools/list" => json_rpc_success(id, json!({ "tools": super::schema::get_tools() })),
    "resources/list" => {
      json_rpc_success(id, json!({ "resources": super::schema::get_resources() }))
    }
    _ => {
      let message = fallback_error_message(port, port_source);
      json_rpc_error(id, -32000, message)
    }
  }
}

async fn handle_stop_app(request: Value) -> Value {
  stop_current_app().await;
  json_rpc_success(
    request["id"].clone(),
    json!({ "content": [{ "type": "text", "text": "Application stopped." }] }),
  )
}

async fn handle_start_app(request: Value) -> Value {
  let id = request["id"].clone();

  // Parse arguments
  let args = request["params"].get("arguments");
  let bin = args
    .and_then(|v| v.get("bin"))
    .and_then(|v| v.as_str())
    .map(ToString::to_string);
  let example = args
    .and_then(|v| v.get("example"))
    .and_then(|v| v.as_str())
    .map(ToString::to_string);
  let package = args
    .and_then(|v| v.get("package"))
    .and_then(|v| v.as_str())
    .map(ToString::to_string);

  if bin.is_none() && example.is_none() && package.is_none() {
    return json_rpc_error(
      id,
      -32602,
      "start_app requires an explicit target. Provide at least one of: 'package', 'bin', or \
       'example'. Examples: {\"package\":\"counter\"} or {\"example\":\"counter\"}."
        .to_string(),
    );
  }

  // Stop existing before starting a new one
  stop_current_app().await;

  let mut features = vec!["debug".to_string()];
  if let Some(args) = args {
    if let Some(f) = args.get("features").and_then(|v| v.as_str()) {
      for feature in f.split(',').map(str::trim) {
        if !features.contains(&feature.to_string()) {
          features.push(feature.to_string());
        }
      }
    }
  }

  let cwd = match std::env::current_dir() {
    Ok(dir) => dir,
    Err(e) => return json_rpc_error(id, -32000, format!("Failed to get current directory: {e}")),
  };

  let mut cmd = tokio::process::Command::new("cargo");
  cmd
    .arg("run")
    .arg("--features")
    .arg(features.join(","))
    .current_dir(&cwd)
    .stdout(Stdio::null())
    .stderr(Stdio::piped()) // Capture error output
    .stdin(Stdio::null());

  if let Some(bin) = &bin {
    cmd.arg("--bin").arg(bin);
  }
  if let Some(example) = &example {
    cmd.arg("--example").arg(example);
  }
  if let Some(package) = &package {
    cmd.arg("-p").arg(package);
  }

  if let Some(args) = args {
    // Support additional cargo arguments
    if let Some(cargo_args) = args.get("cargo_args") {
      match cargo_args {
        Value::String(s) => {
          // Split string by whitespace, respecting quotes
          for arg in shell_words::split(s).unwrap_or_default() {
            cmd.arg(arg);
          }
        }
        Value::Array(arr) => {
          for arg in arr {
            if let Some(s) = arg.as_str() {
              cmd.arg(s);
            }
          }
        }
        _ => {}
      }
    }
  }

  log::info!("Launching: {:?}", cmd);

  match cmd.spawn() {
    Ok(mut child) => {
      let mut stderr = child.stderr.take().unwrap();
      let registry = super::port_discovery::PortRegistry::new();

      for _ in 0..MAX_RETRIES {
        // Check if process has exited early (e.g., compile error)
        match child.try_wait() {
          Ok(Some(status)) => {
            let mut err_msg = String::new();
            // Read stderr with timeout and size limit
            match tokio::time::timeout(
              Duration::from_secs(STDERR_TIMEOUT_SECS),
              read_stderr_limited(&mut stderr, STDERR_MAX_SIZE),
            )
            .await
            {
              Ok(Ok(output)) => err_msg = output,
              Ok(Err(e)) => log::warn!("Failed to read stderr: {}", e),
              Err(_) => log::warn!("Timeout reading stderr"),
            }
            // Explicitly clean up the child process
            drop(child);
            let error_preview = if err_msg.len() > 500 {
              format!("{}... (truncated)", &err_msg[..500])
            } else {
              err_msg
            };
            return json_rpc_error(
              id,
              -32001,
              format!("Process exited with {status}. Error:\n{error_preview}"),
            );
          }
          Ok(None) => {} // Still running
          Err(e) => return json_rpc_error(id, -32000, format!("Error waiting for process: {e}")),
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Some(entry) = registry.discover_best_for_path(&cwd) {
          // Wait until the debug HTTP server is responsive to avoid immediate
          // post-start tool calls failing with connection errors.
          let server_ready = wait_for_debug_server_ready(entry.port).await;

          // Acquire lock first to prevent race condition with stop_current_app
          let mut lock = get_app_child().await.lock().await;
          // Store port and child atomically
          CURRENT_PORT.store(entry.port, Ordering::Relaxed);
          CURRENT_PORT_SOURCE.store(PortSource::StartApp as u8, Ordering::Relaxed);
          *lock = Some(child);

          let message = if server_ready {
            format!("App started on port {} and debug server is ready.", entry.port)
          } else {
            format!(
              "App started on port {}. Debug server may still be warming up; if the next MCP call \
               fails, retry after 1-2 seconds.",
              entry.port
            )
          };

          return json_rpc_success(id, json!({ "content": [{ "type": "text", "text": message }] }));
        }
      }

      let _ = child.kill().await;
      json_rpc_error(
        id,
        -32000,
        format!(
          "Startup timeout while waiting for debug port registration in {}. Verify the app \
           started with '--features debug' and that this worktree is the one running the target \
           app.",
          cwd.display()
        ),
      )
    }
    Err(e) => json_rpc_error(id, -32000, format!("Failed to spawn: {e}")),
  }
}

fn check_version_compatibility(response: &mut Value, server_version: &str) {
  let cli_version = super::schema::MCP_VERSION;
  if server_version != cli_version {
    let warning = format!("Version mismatch: CLI is {cli_version}, Server is {server_version}.");
    if let Some(result) = response["result"].as_object_mut() {
      result.insert("_warning".to_string(), Value::String(warning));
    }
  }
}

fn is_connection_error(err: &reqwest::Error) -> bool { err.is_connect() || err.is_timeout() }

async fn wait_for_debug_server_ready(port: u16) -> bool {
  let client = match reqwest::Client::builder()
    .timeout(Duration::from_millis(800))
    .build()
  {
    Ok(c) => c,
    Err(e) => {
      log::warn!("Failed to create readiness-check client: {}", e);
      return false;
    }
  };

  let url = format!("http://127.0.0.1:{port}/status");
  for _ in 0..READY_CHECK_RETRIES {
    if let Ok(resp) = client.get(&url).send().await {
      if resp.status().is_success() {
        return true;
      }
    }
    tokio::time::sleep(Duration::from_millis(READY_CHECK_INTERVAL_MS)).await;
  }
  false
}

fn fallback_error_message(port: u16, port_source: PortSource) -> String {
  let warmup_hint = if port_source == PortSource::StartApp {
    " The app was launched by 'start_app' and may still be initializing. Wait 1-2 seconds and \
     retry."
  } else {
    ""
  };
  let cwd = std::env::current_dir()
    .map(|p| p.display().to_string())
    .unwrap_or_else(|_| "<unknown cwd>".to_string());

  format!(
    "Cannot connect to Ribir debug server at port {port} (source: {}) for current directory \
     {cwd}.{} Recommended next step: call 'start_app' with an explicit target \
     (package/bin/example). Other options: run 'ribir-cli mcp list' to inspect active sessions, \
     pass '--port <PORT>' to override, or start manually with 'cargo run --features debug \
     --example <name>' or 'cargo run --features debug -p <package>'.",
    port_source.as_label(),
    warmup_hint
  )
}

fn json_rpc_success(id: Value, result: Value) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "result": result })
}
fn json_rpc_error(id: Value, code: i32, message: String) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

async fn read_stderr_limited(
  stderr: &mut tokio::process::ChildStderr, max_size: usize,
) -> Result<String> {
  let mut buffer = Vec::with_capacity(4096);
  let mut total_read = 0;

  loop {
    let mut chunk = vec![0u8; 4096];
    match stderr.read(&mut chunk).await {
      Ok(0) => break, // EOF
      Ok(n) => {
        total_read += n;
        if total_read > max_size {
          buffer.extend_from_slice(&chunk[..n]);
          buffer.extend_from_slice(b"\n... (output truncated, exceeded size limit)");
          break;
        }
        buffer.extend_from_slice(&chunk[..n]);
      }
      Err(e) => return Err(e.into()),
    }
  }

  Ok(String::from_utf8_lossy(&buffer).to_string())
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn fallback_message_for_start_app_contains_retry_hint() {
    let msg = fallback_error_message(2333, PortSource::StartApp);
    assert!(msg.contains("may still be initializing"));
    assert!(msg.contains("Wait 1-2 seconds and retry"));
    assert!(msg.contains("call 'start_app' with an explicit target"));
  }

  #[test]
  fn fallback_message_for_non_start_app_has_no_retry_hint() {
    let msg = fallback_error_message(2333, PortSource::AutoDiscovered);
    assert!(!msg.contains("Wait 1-2 seconds and retry"));
  }

  #[test]
  fn start_app_requires_explicit_target() {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "start_app",
        "arguments": {}
      }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("runtime");
    let response = runtime.block_on(handle_start_app(request));

    assert_eq!(response["error"]["code"], -32602);
    let msg = response["error"]["message"]
      .as_str()
      .expect("error message");
    assert!(msg.contains("requires an explicit target"));
  }
}

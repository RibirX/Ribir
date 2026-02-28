use std::{
  path::{Path, PathBuf},
  process::Stdio,
  sync::{
    Arc, Mutex as StdMutex,
    atomic::{AtomicU8, AtomicU16, Ordering},
  },
  time::Duration,
};

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use tokio::{
  io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
  process::Child,
  sync::Mutex,
};

use crate::util::cargo_settings::CargoSettings;

/// Port being used by the current MCP session (set at startup)
static CURRENT_PORT: AtomicU16 = AtomicU16::new(0);
static CURRENT_PORT_SOURCE: AtomicU8 = AtomicU8::new(PortSource::Unknown as u8);
const START_POLL_INTERVAL_MS: u64 = 100;
const START_IDLE_TIMEOUT_SECS: u64 = 8 * 60;
const STDERR_PROGRESS_PREVIEW_MAX_CHARS: usize = 4096;
const READY_CHECK_RETRIES: usize = 50; // 5 seconds total
const READY_CHECK_INTERVAL_MS: u64 = 100;
const MANUAL_LAUNCH_HINT: &str =
  "If you manually launch, use a non-blocking command because GUI apps block until exit.";
const MANUAL_LAUNCH_TEMPLATES: &[&str] = &[
  "cd <project_path> && nohup cargo run --features debug > /tmp/ribir-debug.log 2>&1 &",
  "Start-Process cargo -ArgumentList 'run --features debug' -WorkingDirectory '<project_path>'",
];

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortSource {
  Unknown = 0,
  ExplicitArg = 1,
  AutoDiscovered = 2,
  StartApp = 3,
  AttachApp = 4,
}

impl PortSource {
  pub(crate) fn as_label(self) -> &'static str {
    match self {
      PortSource::Unknown => "unknown",
      PortSource::ExplicitArg => "--port",
      PortSource::AutoDiscovered => "auto-discovered",
      PortSource::StartApp => "start_app",
      PortSource::AttachApp => "attach_app",
    }
  }
}

fn load_port_source() -> PortSource {
  match CURRENT_PORT_SOURCE.load(Ordering::Relaxed) {
    1 => PortSource::ExplicitArg,
    2 => PortSource::AutoDiscovered,
    3 => PortSource::StartApp,
    4 => PortSource::AttachApp,
    _ => PortSource::Unknown,
  }
}

#[derive(Debug)]
struct AdoptedProcess {
  pid: u32,
  project_path: PathBuf,
  started_at: u64,
  port: u16,
}

impl From<super::port_discovery::PortEntry> for AdoptedProcess {
  fn from(entry: super::port_discovery::PortEntry) -> Self {
    Self {
      pid: entry.pid,
      project_path: entry.project_path,
      started_at: entry.started_at,
      port: entry.port,
    }
  }
}

#[derive(Debug)]
enum ManagedProcess {
  Child(Child),
  Adopted(AdoptedProcess),
}

static MANAGED_PROCESS: tokio::sync::OnceCell<Mutex<Option<ManagedProcess>>> =
  tokio::sync::OnceCell::const_new();

async fn get_managed_process() -> &'static Mutex<Option<ManagedProcess>> {
  MANAGED_PROCESS
    .get_or_init(|| async { Mutex::new(None) })
    .await
}

fn bind_current_port(port: u16, source: PortSource) {
  CURRENT_PORT.store(port, Ordering::Relaxed);
  CURRENT_PORT_SOURCE.store(source as u8, Ordering::Relaxed);
}

#[derive(Debug)]
struct StopOutcome {
  stopped: bool,
  error_code: Option<&'static str>,
  message: String,
}

#[derive(Debug, Clone, Copy)]
enum StartProjectValidationKind {
  InvalidProjectPath,
  ProjectPathNotRunnable,
}

#[derive(Debug, Clone, Copy)]
struct ErrorSpec {
  rpc_code: i32,
  error_code: &'static str,
  next_action: &'static str,
}

#[derive(Debug)]
enum ToolError {
  SessionRequired { port: u16, port_source: PortSource },
  AttachMissingUrl,
  InvalidAttachUrl { reason: String },
  AttachConnectFailed { base_url: String },
  StartMissingProjectPath,
  StartProjectPathNotAbs { path: PathBuf },
  StartCargoTomlMissing { path: PathBuf },
  StartProjectValidation { kind: StartProjectValidationKind, message: String },
  StartSpawnFailed { source: String },
  StartStderrUnavailable,
  StartExitedEarly { status: String, stderr_preview: String },
  StartWaitError { source: String },
  StartTimeout { path: PathBuf, idle_ms: u64, last_progress: Option<String> },
}

impl ToolError {
  fn spec(&self) -> ErrorSpec {
    match self {
      Self::SessionRequired { .. } => ErrorSpec {
        rpc_code: -32000,
        error_code: "SESSION_REQUIRED",
        next_action: "call_start_or_attach",
      },
      Self::AttachMissingUrl => {
        ErrorSpec { rpc_code: -32602, error_code: "INVALID_ARGS", next_action: "provide_url" }
      }
      Self::InvalidAttachUrl { .. } => {
        ErrorSpec { rpc_code: -32602, error_code: "INVALID_URL", next_action: "provide_url" }
      }
      Self::AttachConnectFailed { .. } => ErrorSpec {
        rpc_code: -32000,
        error_code: "ATTACH_CONNECT_FAILED",
        next_action: "start_debug_app_then_attach",
      },
      Self::StartMissingProjectPath => ErrorSpec {
        rpc_code: -32602,
        error_code: "INVALID_ARGS",
        next_action: "provide_project_path",
      },
      Self::StartProjectPathNotAbs { .. }
      | Self::StartCargoTomlMissing { .. }
      | Self::StartProjectValidation {
        kind: StartProjectValidationKind::InvalidProjectPath, ..
      } => ErrorSpec {
        rpc_code: -32602,
        error_code: "INVALID_PROJECT_PATH",
        next_action: "provide_project_path",
      },
      Self::StartProjectValidation {
        kind: StartProjectValidationKind::ProjectPathNotRunnable,
        ..
      } => ErrorSpec {
        rpc_code: -32602,
        error_code: "PROJECT_PATH_NOT_RUNNABLE",
        next_action: "provide_project_path_or_attach_url",
      },
      Self::StartSpawnFailed { .. } => ErrorSpec {
        rpc_code: -32000,
        error_code: "LAUNCH_FAILED",
        next_action: "retry_or_manual_launch",
      },
      Self::StartStderrUnavailable | Self::StartWaitError { .. } => {
        ErrorSpec { rpc_code: -32000, error_code: "LAUNCH_FAILED", next_action: "retry" }
      }
      Self::StartExitedEarly { .. } => ErrorSpec {
        rpc_code: -32001,
        error_code: "LAUNCH_FAILED",
        next_action: "check_project_or_command",
      },
      Self::StartTimeout { .. } => ErrorSpec {
        rpc_code: -32000,
        error_code: "IDLE_TIMEOUT",
        next_action: "retry_or_manual_launch",
      },
    }
  }

  fn message(&self) -> String {
    match self {
      Self::SessionRequired { port, port_source } => fallback_error_message(*port, *port_source),
      Self::AttachMissingUrl => "attach_app requires argument 'url'.".to_string(),
      Self::InvalidAttachUrl { reason } => reason.clone(),
      Self::AttachConnectFailed { base_url } => {
        format!("Cannot connect to debug server at {base_url}.")
      }
      Self::StartMissingProjectPath => "start_app requires argument 'project_path'.".to_string(),
      Self::StartProjectPathNotAbs { path } => {
        format!("project_path must be an absolute path, got '{}'.", path.display())
      }
      Self::StartCargoTomlMissing { path } => {
        format!("No Cargo.toml found in '{}'.", path.display())
      }
      Self::StartProjectValidation { message, .. } => message.clone(),
      Self::StartSpawnFailed { source } => format!("Failed to spawn: {source}"),
      Self::StartStderrUnavailable => "Failed to capture child stderr.".to_string(),
      Self::StartExitedEarly { status, stderr_preview } => {
        format!("Process exited with {status}. Error:\n{stderr_preview}")
      }
      Self::StartWaitError { source } => format!("Error waiting for process: {source}"),
      Self::StartTimeout { path, idle_ms, .. } => format!(
        "start_app became idle for {}s while waiting for debug port registration in {}.",
        idle_ms / 1000,
        path.display()
      ),
    }
  }

  fn extra(&self) -> Value {
    match self {
      Self::SessionRequired { .. } => json!({
        "hint": "Call start_app(project_path) to launch/attach by project, or attach_app(url) if you already know the debug URL."
      }),
      Self::AttachConnectFailed { .. } => json!({
        "launch_steps": [
          "Start the app with debug enabled in a non-blocking way.",
          "Parse RIBIR_DEBUG_URL from app output.",
          "Call attach_app(url) again."
        ],
        "command_templates": MANUAL_LAUNCH_TEMPLATES,
      }),
      Self::StartSpawnFailed { .. } => json!({
        "hint": MANUAL_LAUNCH_HINT,
        "command_templates": MANUAL_LAUNCH_TEMPLATES,
      }),
      Self::StartTimeout { idle_ms, last_progress, .. } => json!({
        "hint": MANUAL_LAUNCH_HINT,
        "command_templates": MANUAL_LAUNCH_TEMPLATES,
        "idle_seconds": idle_ms / 1000,
        "last_progress": last_progress,
      }),
      _ => Value::Object(Map::new()),
    }
  }
}

#[derive(Debug)]
enum ToolSuccess {
  StopOutcome(StopOutcome),
  AttachOutcome {
    base_url: String,
    mcp_url: String,
    port: u16,
    adopt_requested: bool,
    adopted: bool,
  },
  StartAttached {
    port: u16,
    project_path: PathBuf,
    adopted: bool,
  },
  StartLaunched {
    port: u16,
    project_path: PathBuf,
    adopted: bool,
  },
}

impl ToolSuccess {
  fn status(&self) -> &'static str {
    match self {
      Self::StopOutcome(outcome) => {
        if outcome.stopped {
          "stopped"
        } else {
          "noop"
        }
      }
      Self::AttachOutcome { .. } | Self::StartAttached { .. } => "attached",
      Self::StartLaunched { .. } => "started",
    }
  }

  fn message(&self) -> String {
    match self {
      Self::StopOutcome(outcome) => outcome.message.clone(),
      Self::AttachOutcome { base_url, adopt_requested, adopted, .. } => {
        if *adopt_requested && !*adopted {
          format!(
            "Attached to {base_url}, but ownership could not be adopted (pid not found in \
             registry)."
          )
        } else {
          format!("Attached to {base_url}.")
        }
      }
      Self::StartAttached { port, project_path, .. } => format!(
        "Attached to existing debug session on port {} for {}.",
        port,
        project_path.display()
      ),
      Self::StartLaunched { port, .. } => {
        format!("App started on port {} and debug server is ready.", port)
      }
    }
  }

  fn data(&self) -> Value {
    match self {
      Self::StopOutcome(outcome) => json!({
        "stopped": outcome.stopped,
        "error_code": outcome.error_code,
      }),
      Self::AttachOutcome { mcp_url, port, adopted, .. } => json!({
        "url": mcp_url,
        "port": port,
        "adopted": adopted,
      }),
      Self::StartAttached { port, project_path, adopted } => json!({
        "port": port,
        "project_path": project_path.display().to_string(),
        "source": "registry",
        "adopted": adopted,
      }),
      Self::StartLaunched { port, project_path, adopted } => json!({
        "port": port,
        "project_path": project_path.display().to_string(),
        "source": "launched",
        "adopted": adopted,
      }),
    }
  }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StepCode {
  FallbackReq,
  FallbackSessionRequired,
  StopReq,
  StopDone,
  StopNoopOrFail,
  AttachReq,
  AttachArgMissingUrl,
  AttachUrlInvalid,
  AttachUrlParsed,
  AttachStatusOk,
  AttachStatusFailed,
  AttachPortBound,
  AttachAdopted,
  AttachAdoptNotFound,
  AttachDone,
  StartReq,
  StartArgMissingProjectPath,
  StartProjectPathNotAbs,
  StartCargoTomlMissing,
  StartCargoTomlOk,
  StartProjectNotRunnable,
  StartProjectValidated,
  StartAttachLookup,
  StartAttachFound,
  StartAttachNotReady,
  StartAttachReady,
  StartAttachMiss,
  StartLaunchCmdReady,
  StartSpawnFailed,
  StartLaunchSpawned,
  StartLaunchExitedEarly,
  StartLaunchWaitError,
  StartLaunchDiscovered,
  StartLaunchReady,
  StartLaunchNotReady,
  StartLaunchIdleTimeout,
  StartTimeout,
}

#[derive(Debug, Clone, serde::Serialize)]
struct StepEvent {
  code: StepCode,
  at_ms: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  detail: Option<Value>,
}

struct ToolFlowCtx {
  tool: &'static str,
  id: Value,
  steps: Vec<StepEvent>,
}

impl ToolFlowCtx {
  fn from_request(tool: &'static str, request: &Value, first: StepCode) -> Self {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let mut out = Self { tool, id, steps: Vec::new() };
    out.step(first);
    out
  }

  fn step(&mut self, code: StepCode) -> &mut Self {
    self
      .steps
      .push(StepEvent { code, at_ms: now_unix_ms(), detail: None });
    self
  }

  fn step_with_detail(&mut self, code: StepCode, detail: Value) -> &mut Self {
    self
      .steps
      .push(StepEvent { code, at_ms: now_unix_ms(), detail: Some(detail) });
    self
  }

  fn last_step_code(&self) -> Option<StepCode> { self.steps.last().map(|s| s.code) }

  fn tool_meta(&self) -> Value {
    json!({
      "name": self.tool,
      "last_step": self.last_step_code(),
      "steps": self.steps,
    })
  }

  fn ok(self, success: ToolSuccess) -> Value {
    let mut result = Map::new();
    result.insert("status".to_string(), Value::String(success.status().to_string()));
    result.insert("content".to_string(), json!([{ "type": "text", "text": success.message() }]));
    result.insert("tool".to_string(), self.tool_meta());
    result.insert("data".to_string(), object_or_empty(success.data()));
    json_rpc_success(self.id, Value::Object(result))
  }

  fn fail(self, err: ToolError) -> Value {
    let spec = err.spec();
    let data = json!({
      "error_code": spec.error_code,
      "next_action": spec.next_action,
      "tool": self.tool_meta(),
      "extra": object_or_empty(err.extra()),
    });
    json_rpc_error_with_data(self.id, spec.rpc_code, err.message(), data)
  }

  fn fail_at(mut self, step: StepCode, err: ToolError) -> Value {
    self.step(step);
    self.fail(err)
  }

  fn fail_with_step_detail(mut self, step: StepCode, detail: Value, err: ToolError) -> Value {
    self.step_with_detail(step, detail);
    self.fail(err)
  }
}

pub async fn mcp_serve(port: u16, port_source: PortSource) -> Result<()> {
  bind_current_port(port, port_source);

  tracing::info!(
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
        tracing::error!("Failed to parse JSON-RPC request: {}", e);
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

  // Cleanup: ensure the managed app is killed when MCP server exits
  let _ = stop_managed_process().await;

  Ok(())
}

async fn stop_managed_process() -> StopOutcome {
  let mut lock = get_managed_process().await.lock().await;
  let Some(process) = lock.take() else {
    return StopOutcome {
      stopped: false,
      error_code: Some("NO_MANAGED_PROCESS"),
      message: "No managed process found; nothing to stop.".to_string(),
    };
  };

  match process {
    ManagedProcess::Child(mut child) => match child.kill().await {
      Ok(()) => StopOutcome {
        stopped: true,
        error_code: None,
        message: "Stopped managed application process.".to_string(),
      },
      Err(e) => StopOutcome {
        stopped: false,
        error_code: Some("STOP_FAILED"),
        message: format!("Failed to stop managed application process: {e}"),
      },
    },
    ManagedProcess::Adopted(meta) => {
      if !ownership_still_matches(&meta) {
        return StopOutcome {
          stopped: false,
          error_code: Some("OWNERSHIP_STALE"),
          message: "Managed process ownership is stale; refusing to stop unknown process."
            .to_string(),
        };
      }
      match kill_pid(meta.pid) {
        Ok(()) => StopOutcome {
          stopped: true,
          error_code: None,
          message: format!("Stopped adopted process pid {}.", meta.pid),
        },
        Err(e) => StopOutcome {
          stopped: false,
          error_code: Some("STOP_FAILED"),
          message: format!("Failed to stop adopted process pid {}: {e}", meta.pid),
        },
      }
    }
  }
}

fn ownership_still_matches(meta: &AdoptedProcess) -> bool {
  let registry = super::port_discovery::PortRegistry::new();
  registry.list_all().into_iter().any(|entry| {
    entry.pid == meta.pid
      && entry.started_at == meta.started_at
      && entry.port == meta.port
      && normalize_path(&entry.project_path) == normalize_path(&meta.project_path)
  })
}

#[cfg(unix)]
fn kill_pid(pid: u32) -> std::io::Result<()> {
  let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
  if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}

#[cfg(windows)]
fn kill_pid(pid: u32) -> std::io::Result<()> {
  use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess},
  };

  unsafe {
    let handle: HANDLE = OpenProcess(PROCESS_TERMINATE, 0, pid);
    if handle == 0 {
      return Err(std::io::Error::last_os_error());
    }

    let ok = TerminateProcess(handle, 1) != 0;
    let close_result = CloseHandle(handle);
    if close_result == 0 {
      return Err(std::io::Error::last_os_error());
    }

    if ok { Ok(()) } else { Err(std::io::Error::last_os_error()) }
  }
}

#[cfg(not(any(unix, windows)))]
fn kill_pid(_pid: u32) -> std::io::Result<()> {
  Err(std::io::Error::new(std::io::ErrorKind::Other, "kill_pid is not supported on this platform"))
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
  line
    .strip_suffix(b"\r\n")
    .or_else(|| line.strip_suffix(b"\n"))
    .or_else(|| line.strip_suffix(b"\r"))
    .unwrap_or(line)
}

async fn write_response(stdout: &mut tokio::io::Stdout, payload: &[u8]) -> Result<()> {
  stdout.write_all(payload).await?;
  stdout.write_all(b"\n").await?;
  stdout.flush().await?;
  Ok(())
}

fn is_notification(request: &Value) -> bool {
  let id = request.get("id");
  id.is_none()
    || id.is_some_and(|v| v.is_null())
    || request["method"] == "notifications/initialized"
}

fn tool_call_name(request: &Value) -> Option<&str> {
  if request["method"] != "tools/call" {
    return None;
  }
  request
    .get("params")
    .and_then(|p| p.get("name"))
    .and_then(Value::as_str)
}

async fn handle_request(client: &reqwest::Client, request: Value) -> Value {
  if let Some(tool_name) = tool_call_name(&request) {
    match tool_name {
      "start_app" => return handle_start_app(request).await,
      "attach_app" => return handle_attach_app(client, request).await,
      "stop_app" => return handle_stop_app(request).await,
      _ => {}
    };
  }

  let method = request["method"].as_str().unwrap_or("");
  let port = CURRENT_PORT.load(Ordering::Relaxed);
  let port_source = load_port_source();
  let url = format!("http://127.0.0.1:{}/mcp/message", port);

  match client.post(&url).json(&request).send().await {
    Ok(resp) => match resp.json::<Value>().await {
      Ok(mut json) => {
        if method == "initialize" {
          if let Some(v) = json["result"]["serverInfo"]["version"]
            .as_str()
            .map(str::to_owned)
          {
            check_version_compatibility(&mut json, &v);
          }
        }
        json
      }
      Err(e) => {
        tracing::error!("Failed to parse response: {}", e);
        json_rpc_error(request["id"].clone(), -32603, format!("Parse error: {}", e))
      }
    },
    Err(e) if is_connection_error(&e) => handle_fallback(request, port, port_source).await,
    Err(e) => {
      tracing::error!("Request failed: {}", e);
      json_rpc_error(request["id"].clone(), -32603, format!("Request failed: {}", e))
    }
  }
}

async fn handle_fallback(request: Value, port: u16, port_source: PortSource) -> Value {
  let method = request["method"].as_str().unwrap_or("");

  match method {
    "initialize" => {
      json_rpc_success(request["id"].clone(), super::schema::get_fallback_init_result())
    }
    "tools/list" => {
      json_rpc_success(request["id"].clone(), json!({ "tools": super::schema::get_tools() }))
    }
    "resources/list" => json_rpc_success(
      request["id"].clone(),
      json!({ "resources": super::schema::get_resources() }),
    ),
    _ => ToolFlowCtx::from_request("fallback", &request, StepCode::FallbackReq)
      .fail_with_step_detail(
        StepCode::FallbackSessionRequired,
        json!({ "port": port, "source": port_source.as_label() }),
        ToolError::SessionRequired { port, port_source },
      ),
  }
}

async fn handle_stop_app(request: Value) -> Value {
  let mut ctx = ToolFlowCtx::from_request("stop_app", &request, StepCode::StopReq);
  let outcome = stop_managed_process().await;
  ctx.step(if outcome.stopped { StepCode::StopDone } else { StepCode::StopNoopOrFail });
  ctx.ok(ToolSuccess::StopOutcome(outcome))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct AttachAppArgs {
  url: Option<String>,
  adopt: Option<bool>,
}

impl AttachAppArgs {
  fn adopt(&self) -> bool { self.adopt.unwrap_or(false) }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct StartAppArgs {
  project_path: Option<String>,
  features: Option<String>,
  cargo_args: Option<CargoArgsInput>,
  adopt: Option<bool>,
}

impl StartAppArgs {
  fn adopt(&self) -> bool { self.adopt.unwrap_or(true) }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum CargoArgsInput {
  String(String),
  Array(Vec<String>),
}

#[derive(Debug)]
struct StepError {
  step: StepCode,
  err: ToolError,
}

fn parse_tool_args_or_default<T>(request: &Value) -> T
where
  T: DeserializeOwned + Default,
{
  let args = request
    .get("params")
    .and_then(|v| v.get("arguments"))
    .cloned()
    .unwrap_or_else(|| Value::Object(Map::new()));
  match serde_json::from_value(args) {
    Ok(v) => v,
    Err(e) => {
      tracing::warn!("Invalid tool arguments, falling back to defaults: {}", e);
      T::default()
    }
  }
}

async fn handle_attach_app(client: &reqwest::Client, request: Value) -> Value {
  let mut ctx = ToolFlowCtx::from_request("attach_app", &request, StepCode::AttachReq);
  let args: AttachAppArgs = parse_tool_args_or_default(&request);
  let Some(url_input) = args
    .url
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  else {
    return ctx.fail_at(StepCode::AttachArgMissingUrl, ToolError::AttachMissingUrl);
  };
  let adopt = args.adopt();

  let (base_url, port) = match normalize_debug_base_url(url_input) {
    Ok(v) => v,
    Err(msg) => {
      return ctx.fail_at(StepCode::AttachUrlInvalid, ToolError::InvalidAttachUrl { reason: msg });
    }
  };
  ctx.step_with_detail(StepCode::AttachUrlParsed, json!({ "port": port }));

  let status_url = format!("{base_url}/status");
  let mcp_url = format!("{base_url}/mcp/message");
  let status_ok = client
    .get(&status_url)
    .send()
    .await
    .is_ok_and(|resp| resp.status().is_success());
  ctx.step(if status_ok { StepCode::AttachStatusOk } else { StepCode::AttachStatusFailed });

  if !status_ok {
    return ctx.fail(ToolError::AttachConnectFailed { base_url });
  }

  bind_current_port(port, PortSource::AttachApp);
  ctx.step(StepCode::AttachPortBound);

  let mut adopted = false;
  if adopt {
    let registry = super::port_discovery::PortRegistry::new();
    if let Some(entry) = registry
      .list_all()
      .into_iter()
      .find(|e| e.port == port)
    {
      let mut lock = get_managed_process().await.lock().await;
      *lock = Some(ManagedProcess::Adopted(entry.into()));
      adopted = true;
      ctx.step(StepCode::AttachAdopted);
    } else {
      ctx.step(StepCode::AttachAdoptNotFound);
    }
  }
  ctx.step(StepCode::AttachDone);

  ctx.ok(ToolSuccess::AttachOutcome { base_url, mcp_url, port, adopt_requested: adopt, adopted })
}

fn validate_start_project_path(
  project_path: Option<&str>,
) -> std::result::Result<PathBuf, StepError> {
  let Some(project_path) = project_path
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(PathBuf::from)
  else {
    return Err(StepError {
      step: StepCode::StartArgMissingProjectPath,
      err: ToolError::StartMissingProjectPath,
    });
  };

  if !project_path.is_absolute() {
    return Err(StepError {
      step: StepCode::StartProjectPathNotAbs,
      err: ToolError::StartProjectPathNotAbs { path: project_path },
    });
  }

  let cwd = normalize_path(&project_path);
  let cargo_toml = cwd.join("Cargo.toml");
  if !cargo_toml.exists() {
    return Err(StepError {
      step: StepCode::StartCargoTomlMissing,
      err: ToolError::StartCargoTomlMissing { path: cwd },
    });
  }

  Ok(cwd)
}

async fn try_attach_existing_session(
  ctx: &mut ToolFlowCtx, cwd: &Path, adopt: bool,
) -> std::result::Result<Option<ToolSuccess>, StepError> {
  let registry = super::port_discovery::PortRegistry::new();
  ctx.step(StepCode::StartAttachLookup);

  let Some(entry) = registry.discover_for_path(cwd) else {
    ctx.step(StepCode::StartAttachMiss);
    return Ok(None);
  };

  ctx.step(StepCode::StartAttachFound);
  if !wait_for_debug_server_ready(entry.port).await {
    ctx.step(StepCode::StartAttachNotReady);
    // If the port is registered but unresponsive, fallback to launching a new
    // session. The new session will overwrite the stale registry entry.
    return Ok(None);
  }

  ctx.step(StepCode::StartAttachReady);
  bind_current_port(entry.port, PortSource::AutoDiscovered);

  let port = entry.port;
  let project_path = entry.project_path.clone();
  if adopt {
    let mut lock = get_managed_process().await.lock().await;
    *lock = Some(ManagedProcess::Adopted(entry.into()));
  }

  Ok(Some(ToolSuccess::StartAttached { port, project_path, adopted: adopt }))
}

async fn launch_new_session(
  ctx: &mut ToolFlowCtx, cwd: &Path, args: &StartAppArgs, adopt: bool,
) -> std::result::Result<ToolSuccess, StepError> {
  // Starting a new managed process replaces previous ownership.
  if adopt {
    let _ = stop_managed_process().await;
  }

  let registry = super::port_discovery::PortRegistry::new();
  let mut cmd = tokio::process::Command::new("cargo");
  cmd
    .arg("run")
    .arg("--features")
    .arg(merged_features(args).join(","))
    .current_dir(cwd)
    .stdout(Stdio::null())
    .stderr(Stdio::piped())
    .stdin(Stdio::null());
  append_cargo_args(&mut cmd, args.cargo_args.as_ref());

  tracing::info!("Launching: {:?}", cmd);
  ctx.step(StepCode::StartLaunchCmdReady);

  let mut child = match cmd.spawn() {
    Ok(child) => child,
    Err(e) => {
      return Err(StepError {
        step: StepCode::StartSpawnFailed,
        err: ToolError::StartSpawnFailed { source: e.to_string() },
      });
    }
  };
  ctx.step(StepCode::StartLaunchSpawned);

  let Some(mut stderr) = child.stderr.take() else {
    return Err(StepError {
      step: StepCode::StartSpawnFailed,
      err: ToolError::StartStderrUnavailable,
    });
  };

  let stderr_progress = Arc::new(StdMutex::new(StderrProgress::new()));
  let stderr_progress_reader = Arc::clone(&stderr_progress);
  let stderr_task = tokio::spawn(async move {
    drain_stderr_progress(&mut stderr, stderr_progress_reader).await;
  });

  let idle_timeout_ms = START_IDLE_TIMEOUT_SECS * 1000;

  loop {
    match child.try_wait() {
      Ok(Some(status)) => {
        let _ = tokio::time::timeout(Duration::from_millis(150), stderr_task).await;
        let error_preview = stderr_progress
          .lock()
          .ok()
          .map(|p| p.preview())
          .unwrap_or_default();
        drop(child);
        return Err(StepError {
          step: StepCode::StartLaunchExitedEarly,
          err: ToolError::StartExitedEarly {
            status: status.to_string(),
            stderr_preview: error_preview,
          },
        });
      }
      Ok(None) => {}
      Err(e) => {
        return Err(StepError {
          step: StepCode::StartLaunchWaitError,
          err: ToolError::StartWaitError { source: e.to_string() },
        });
      }
    }

    tokio::time::sleep(Duration::from_millis(START_POLL_INTERVAL_MS)).await;
    let Some(entry) = registry.discover_for_path(cwd) else {
      let idle_ms = stderr_progress
        .lock()
        .ok()
        .map(|p| p.idle_ms())
        .unwrap_or(0);
      if idle_ms >= idle_timeout_ms {
        let _ = child.kill().await;
        let last_progress = stderr_progress
          .lock()
          .ok()
          .and_then(|p| p.last_progress_line.clone());
        ctx.step_with_detail(
          StepCode::StartLaunchIdleTimeout,
          json!({ "idle_ms": idle_ms, "idle_timeout_ms": idle_timeout_ms }),
        );
        return Err(StepError {
          step: StepCode::StartTimeout,
          err: ToolError::StartTimeout { path: cwd.to_path_buf(), idle_ms, last_progress },
        });
      }
      continue;
    };

    ctx.step(StepCode::StartLaunchDiscovered);
    let server_ready = wait_for_debug_server_ready(entry.port).await;
    ctx.step(if server_ready { StepCode::StartLaunchReady } else { StepCode::StartLaunchNotReady });
    if !server_ready {
      continue;
    }

    bind_current_port(entry.port, PortSource::StartApp);
    if adopt {
      let mut lock = get_managed_process().await.lock().await;
      *lock = Some(ManagedProcess::Child(child));
    }

    return Ok(ToolSuccess::StartLaunched {
      port: entry.port,
      project_path: cwd.to_path_buf(),
      adopted: adopt,
    });
  }
}

async fn handle_start_app(request: Value) -> Value {
  let mut ctx = ToolFlowCtx::from_request("start_app", &request, StepCode::StartReq);
  let args: StartAppArgs = parse_tool_args_or_default(&request);
  let cwd = match validate_start_project_path(args.project_path.as_deref()) {
    Ok(path) => path,
    Err(e) => return ctx.fail_at(e.step, e.err),
  };
  ctx.step(StepCode::StartCargoTomlOk);

  if let Err((kind, message)) = validate_runnable_project(&cwd) {
    return ctx.fail_at(
      StepCode::StartProjectNotRunnable,
      ToolError::StartProjectValidation { kind, message },
    );
  }
  ctx.step(StepCode::StartProjectValidated);

  let adopt = args.adopt();

  match try_attach_existing_session(&mut ctx, &cwd, adopt).await {
    Ok(Some(success)) => return ctx.ok(success),
    Ok(None) => {}
    Err(e) => return ctx.fail_at(e.step, e.err),
  }

  match launch_new_session(&mut ctx, &cwd, &args, adopt).await {
    Ok(success) => ctx.ok(success),
    Err(e) => ctx.fail_at(e.step, e.err),
  }
}

fn merged_features(args: &StartAppArgs) -> Vec<String> {
  let mut features = vec!["debug".to_string()];
  let Some(raw) = args.features.as_deref() else {
    return features;
  };

  for feature in raw
    .split(',')
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    let feature = feature.to_string();
    if !features.contains(&feature) {
      features.push(feature);
    }
  }

  features
}

fn append_cargo_args(cmd: &mut tokio::process::Command, cargo_args: Option<&CargoArgsInput>) {
  let Some(cargo_args) = cargo_args else {
    return;
  };

  match cargo_args {
    CargoArgsInput::String(s) => {
      match shell_words::split(s) {
        Ok(args) => {
          for arg in args {
            cmd.arg(arg);
          }
        }
        Err(e) => {
          // Preserve user intent instead of silently dropping args on parse errors.
          tracing::warn!(
            "Failed to parse cargo_args string as shell words: {}. Using raw string.",
            e
          );
          cmd.arg(s);
        }
      }
    }
    CargoArgsInput::Array(arr) => {
      for arg in arr {
        cmd.arg(arg);
      }
    }
  }
}

fn truncate_preview(input: &str, max_chars: usize) -> String {
  if input.chars().count() <= max_chars {
    return input.to_string();
  }

  let end = input
    .char_indices()
    .nth(max_chars)
    .map(|(idx, _)| idx)
    .unwrap_or(input.len());
  format!("{}... (truncated)", &input[..end])
}

fn validate_runnable_project(
  path: &Path,
) -> std::result::Result<(), (StartProjectValidationKind, String)> {
  let manifest_path = path.join("Cargo.toml");
  let settings = match CargoSettings::load(&manifest_path) {
    Ok(v) => v,
    Err(e) => {
      return Err((
        StartProjectValidationKind::InvalidProjectPath,
        format!("Failed to parse Cargo.toml in '{}': {e}", path.display()),
      ));
    }
  };

  if settings.package.is_none() {
    return Err((
      StartProjectValidationKind::ProjectPathNotRunnable,
      format!(
        "'{}' is not a runnable crate path. Provide a crate directory (not workspace root), or \
         call attach_app(url).",
        path.display()
      ),
    ));
  }

  Ok(())
}

fn normalize_debug_base_url(input: &str) -> std::result::Result<(String, u16), String> {
  let candidate = input.trim();
  let parsed = reqwest::Url::parse(candidate).map_err(|e| format!("Invalid URL '{input}': {e}"))?;
  if !matches!(parsed.scheme(), "http" | "https") {
    return Err(format!("Invalid URL '{input}': only http:// or https:// is supported."));
  }

  let host = parsed
    .host_str()
    .ok_or_else(|| format!("Invalid URL '{input}': missing host."))?;

  let port = parsed.port_or_known_default().ok_or_else(|| {
    format!("Invalid URL '{input}': missing port and no known default is available.")
  })?;

  let host_fmt = if host.contains(':') && !host.starts_with('[') {
    format!("[{host}]")
  } else {
    host.to_string()
  };
  let base = format!("{}://{}:{port}", parsed.scheme(), host_fmt);
  Ok((base, port))
}

fn normalize_path(path: &Path) -> PathBuf {
  path
    .canonicalize()
    .unwrap_or_else(|_| path.to_path_buf())
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

fn object_or_empty(value: Value) -> Value {
  if value.is_object() { value } else { Value::Object(Map::new()) }
}

async fn wait_for_debug_server_ready(port: u16) -> bool {
  let client = match reqwest::Client::builder()
    .timeout(Duration::from_millis(800))
    .build()
  {
    Ok(c) => c,
    Err(e) => {
      tracing::warn!("Failed to create readiness-check client: {}", e);
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
  format!(
    "Cannot connect to Ribir debug server at port {port} (source: {}). Recommended next step: \
     call 'start_app' with a runnable crate 'project_path', or call 'attach_app' with an explicit \
     debug URL. If launching manually, run in background because GUI apps block until exit.",
    port_source.as_label(),
  )
}

fn now_unix_ms() -> u64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64
}

#[derive(Debug)]
struct StderrProgress {
  last_activity_ms: u64,
  preview: String,
  last_progress_line: Option<String>,
}

impl StderrProgress {
  fn new() -> Self {
    Self { last_activity_ms: now_unix_ms(), preview: String::new(), last_progress_line: None }
  }

  fn record(&mut self, chunk: &str) {
    let trimmed = chunk.trim();
    self.last_activity_ms = now_unix_ms();
    if !trimmed.is_empty() {
      self.last_progress_line = Some(trimmed.to_string());
    }
    self.preview.push_str(chunk);
    if self.preview.chars().count() > STDERR_PROGRESS_PREVIEW_MAX_CHARS {
      self.preview = keep_last_chars(&self.preview, STDERR_PROGRESS_PREVIEW_MAX_CHARS);
    }
  }

  fn idle_ms(&self) -> u64 { now_unix_ms().saturating_sub(self.last_activity_ms) }

  fn preview(&self) -> String { truncate_preview(&self.preview, 500) }
}

fn keep_last_chars(input: &str, max_chars: usize) -> String {
  let char_count = input.chars().count();
  if char_count <= max_chars {
    return input.to_string();
  }
  let skip = char_count.saturating_sub(max_chars);
  let tail = input.chars().skip(skip).collect::<String>();
  format!("...{}", tail)
}

async fn drain_stderr_progress(
  stderr: &mut tokio::process::ChildStderr, progress: Arc<StdMutex<StderrProgress>>,
) {
  let mut reader = BufReader::new(stderr);
  loop {
    let mut line = Vec::new();
    let read = reader.read_until(b'\n', &mut line).await;
    let Ok(bytes) = read else { break };
    if bytes == 0 {
      break;
    }
    let text = String::from_utf8_lossy(&line);
    if let Ok(mut guard) = progress.lock() {
      guard.record(&text);
    }
  }
}

fn json_rpc_success(id: Value, result: Value) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn json_rpc_error(id: Value, code: i32, message: String) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn json_rpc_error_with_data(id: Value, code: i32, message: String, data: Value) -> Value {
  json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message, "data": data } })
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn normalize_base_url_accepts_explicit_http_url() {
    let (base, port) = normalize_debug_base_url("http://127.0.0.1:2333").expect("url parsed");
    assert_eq!(base, "http://127.0.0.1:2333");
    assert_eq!(port, 2333);
  }

  #[test]
  fn normalize_base_url_accepts_https_remote_with_default_port() {
    let (base, port) = normalize_debug_base_url("https://example.com").expect("url parsed");
    assert_eq!(base, "https://example.com:443");
    assert_eq!(port, 443);
  }

  #[test]
  fn normalize_base_url_rejects_unsupported_scheme() {
    let err = normalize_debug_base_url("ftp://example.com:21").expect_err("must fail");
    assert!(err.contains("http:// or https://"));
  }

  #[test]
  fn truncate_preview_is_utf8_safe() {
    let input = "错误".repeat(400);
    let preview = truncate_preview(&input, 500);
    assert!(preview.ends_with("... (truncated)"));
    assert!(preview.is_char_boundary(preview.len()));
  }

  #[test]
  fn start_app_requires_project_path() {
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
    assert_eq!(response["error"]["data"]["error_code"], "INVALID_ARGS");
    assert_eq!(response["error"]["data"]["tool"]["name"], "start_app");
    assert_eq!(response["error"]["data"]["tool"]["last_step"], "START_ARG_MISSING_PROJECT_PATH");
    assert_eq!(response["error"]["data"]["tool"]["steps"][0]["code"], "START_REQ");
    assert_eq!(response["error"]["data"]["next_action"], "provide_project_path");
  }

  #[test]
  fn start_app_rejects_relative_project_path() {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "start_app",
        "arguments": {
          "project_path": "relative/path"
        }
      }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("runtime");
    let response = runtime.block_on(handle_start_app(request));

    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["data"]["error_code"], "INVALID_PROJECT_PATH");
    assert_eq!(response["error"]["data"]["tool"]["last_step"], "START_PROJECT_PATH_NOT_ABS");
    assert_eq!(response["error"]["data"]["next_action"], "provide_project_path");
  }

  #[test]
  fn start_app_rejects_blank_project_path() {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "start_app",
        "arguments": {
          "project_path": "   "
        }
      }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("runtime");
    let response = runtime.block_on(handle_start_app(request));

    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["data"]["error_code"], "INVALID_ARGS");
    assert_eq!(response["error"]["data"]["tool"]["last_step"], "START_ARG_MISSING_PROJECT_PATH");
    assert_eq!(response["error"]["data"]["next_action"], "provide_project_path");
  }

  #[test]
  fn attach_app_requires_url_and_reports_steps() {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "attach_app",
        "arguments": {}
      }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("runtime");
    let client = reqwest::Client::new();
    let response = runtime.block_on(handle_attach_app(&client, request));

    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["data"]["error_code"], "INVALID_ARGS");
    assert_eq!(response["error"]["data"]["tool"]["name"], "attach_app");
    assert_eq!(response["error"]["data"]["tool"]["last_step"], "ATTACH_ARG_MISSING_URL");
    assert_eq!(response["error"]["data"]["next_action"], "provide_url");
  }

  #[test]
  fn stop_app_success_shape_contains_structured_steps() {
    let request = json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "tools/call",
      "params": {
        "name": "stop_app",
        "arguments": {}
      }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("runtime");
    let response = runtime.block_on(handle_stop_app(request));

    assert!(response.get("result").is_some());
    assert_eq!(response["result"]["tool"]["name"], "stop_app");
    assert_eq!(response["result"]["tool"]["steps"][0]["code"], "STOP_REQ");
    assert!(
      response["result"]["tool"]["steps"][0]["at_ms"]
        .as_u64()
        .is_some()
    );
    assert!(response["result"]["data"]["next_action"].is_null());
  }

  #[test]
  fn fallback_message_mentions_start_and_attach() {
    let msg = fallback_error_message(2333, PortSource::AutoDiscovered);
    assert!(msg.contains("start_app"));
    assert!(msg.contains("attach_app"));
  }
}

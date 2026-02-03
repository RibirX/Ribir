//! MCP Protocol Implementation
//!
//! This module implements the Model Context Protocol (MCP) for the Ribir debug
//! server. See: https://modelcontextprotocol.io

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::debug_tool::server::DebugServerState;

// === JSON-RPC Types ===

/// JSON-RPC 2.0 Request
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcRequest {
  #[serde(rename = "jsonrpc")]
  pub jsonrpc: String,
  pub method: String,
  #[serde(default)]
  pub params: Option<Value>,
  pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcResponse {
  #[serde(rename = "jsonrpc")]
  pub jsonrpc: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<JsonRpcError>,
  pub id: Option<Value>,
}

impl JsonRpcResponse {
  pub fn result(id: Option<Value>, result: Value) -> Self {
    Self { jsonrpc: "2.0".to_string(), result: Some(result), error: None, id }
  }

  pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
    Self {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError { code, message: message.into(), data: None }),
      id,
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcError {
  pub code: i32,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitializeResult {
  #[serde(rename = "protocolVersion")]
  pub protocol_version: String,
  pub capabilities: ServerCapabilities,
  #[serde(rename = "serverInfo")]
  pub server_info: ServerInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerCapabilities {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resources: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerInfo {
  pub name: String,
  pub version: String,
  pub description: String,
}

// === Tools ===

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
  pub name: String,
  pub description: String,
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListToolsResult {
  pub tools: Vec<Tool>,
}

#[derive(Deserialize, Debug)]
pub struct CallToolParams {
  pub name: String,
  #[serde(default)]
  pub arguments: Option<Value>,
}

#[derive(Serialize, Debug)]
pub struct CallToolResult {
  pub content: Vec<ToolContent>,
  #[serde(default)]
  #[serde(rename = "isError")]
  pub is_error: bool,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum ToolContent {
  #[serde(rename = "text")]
  Text { text: String },
  #[serde(rename = "image")]
  Image {
    data: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
  },
}

// === Resources ===

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resource {
  pub uri: String,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "mimeType")]
  pub mime_type: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ListResourcesResult {
  pub resources: Vec<Resource>,
}

#[derive(Deserialize, Debug)]
pub struct ReadResourceParams {
  pub uri: String,
}

#[derive(Serialize, Debug)]
pub struct ReadResourceResult {
  pub contents: Vec<ResourceContent>,
}

#[derive(Serialize, Debug)]
pub struct ResourceContent {
  pub uri: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "mimeType")]
  pub mime_type: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub blob: Option<String>,
}

// === Request Handler ===

pub async fn handle_mcp_request(
  request: JsonRpcRequest, state: Arc<DebugServerState>,
) -> JsonRpcResponse {
  let id = request.id.clone();

  match request.method.as_str() {
    "initialize" => {
      log::info!("MCP: initialize request from client");

      #[derive(Deserialize)]
      struct McpSchemaInit {
        fallback_init_result: InitializeResult,
      }

      let schema_json = include_str!("mcp_schema.json");
      let schema: McpSchemaInit =
        serde_json::from_str(schema_json).expect("Failed to parse mcp_schema.json");

      let mut result = schema.fallback_init_result;
      // Overwrite dynamic fields if necessary (like version if not in JSON)
      result.server_info.version = env!("CARGO_PKG_VERSION").to_string();

      JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
    }

    "notifications/initialized" => {
      log::info!("MCP: client initialized");
      // Notification - return success with no result
      JsonRpcResponse::result(id, Value::Null)
    }

    "tools/list" => {
      log::info!("MCP: tools/list request");

      #[derive(Deserialize)]
      struct McpSchemaTools {
        tools: Vec<Tool>,
      }

      let schema_json = include_str!("mcp_schema.json");
      let schema: McpSchemaTools =
        serde_json::from_str(schema_json).expect("Failed to parse mcp_schema.json");

      JsonRpcResponse::result(
        id,
        serde_json::to_value(ListToolsResult { tools: schema.tools }).unwrap(),
      )
    }

    "tools/call" => handle_tool_call(request.params, state, id).await,

    "resources/list" => {
      log::info!("MCP: resources/list request");

      #[derive(Deserialize)]
      struct McpSchemaResources {
        resources: Vec<Resource>,
      }
      let schema_json = include_str!("mcp_schema.json");
      let schema: McpSchemaResources =
        serde_json::from_str(schema_json).expect("Failed to parse mcp_schema.json");

      JsonRpcResponse::result(
        id,
        serde_json::to_value(ListResourcesResult { resources: schema.resources }).unwrap(),
      )
    }

    "resources/read" => handle_read_resource(request.params, state, id).await,

    _ => {
      log::warn!("MCP: unknown method: {}", request.method);
      JsonRpcResponse::error(id, -32601, format!("Method not found: {}", request.method))
    }
  }
}

async fn handle_tool_call(
  params: Option<Value>, state: Arc<DebugServerState>, id: Option<Value>,
) -> JsonRpcResponse {
  use crate::debug_tool::service::*;

  let params: CallToolParams = match serde_json::from_value(params.unwrap_or(Value::Null)) {
    Ok(p) => p,
    Err(e) => return JsonRpcResponse::error(id, -32602, format!("Invalid params: {}", e)),
  };

  log::info!("MCP: tools/call - {}", params.name);

  // Helper to extract common arguments
  let args = params.arguments.as_ref();
  let get_str = |key: &str| -> Option<&str> {
    args
      .and_then(|a| a.get(key))
      .and_then(|v| v.as_str())
  };
  let get_window_id = || -> Option<crate::window::WindowId> {
    get_str("window_id").and_then(|s| {
      s.parse::<u64>()
        .ok()
        .map(crate::window::WindowId::from)
    })
  };

  match params.name.as_str() {
    "capture_screenshot" => match capture_screenshot_svc(&state).await {
      Ok(img) => {
        let mut png_data = Vec::new();
        if img.write_as_png(&mut png_data).is_ok() {
          use base64::{Engine as _, engine::general_purpose};
          let b64 = general_purpose::STANDARD.encode(&png_data);
          let result = CallToolResult {
            content: vec![ToolContent::Image { data: b64, mime_type: "image/png".to_string() }],
            is_error: false,
          };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        } else {
          JsonRpcResponse::error(id, -32000, "Failed to encode image")
        }
      }
      Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
    },

    "inspect_tree" => {
      let options = parse_options(get_str("options"));
      match inspect_tree_svc(&state, get_window_id(), options).await {
        Ok(tree) => {
          let json_str = serde_json::to_string_pretty(&tree).unwrap_or_default();
          let result =
            CallToolResult { content: vec![ToolContent::Text { text: json_str }], is_error: false };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
      }
    }

    "inspect_widget" => {
      let options = parse_options(get_str("options"));
      let Some(widget_id) = get_str("id").map(String::from) else {
        return JsonRpcResponse::error(id, -32602, "Missing required argument: id");
      };

      match inspect_widget_svc(&state, get_window_id(), widget_id, options).await {
        Ok(info) => {
          let json_str = serde_json::to_string_pretty(&info).unwrap_or_default();
          let result =
            CallToolResult { content: vec![ToolContent::Text { text: json_str }], is_error: false };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(ServiceError::NotFound) => JsonRpcResponse::error(id, -32001, "Widget not found"),
        Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
      }
    }

    "get_overlays" => {
      // If no window_id provided, get the default window
      let w_id = if let Some(wid) = get_window_id() {
        Some(wid)
      } else {
        get_windows_svc(&state)
          .await
          .ok()
          .and_then(|list| list.first().map(|w| w.id))
      };

      if let Some(wid) = w_id {
        let overlays = get_overlays_svc(wid);
        let json_str = serde_json::to_string_pretty(&overlays).unwrap_or_default();
        let result =
          CallToolResult { content: vec![ToolContent::Text { text: json_str }], is_error: false };
        JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
      } else {
        JsonRpcResponse::error(id, -32000, "No active window found")
      }
    }

    "set_log_filter" => {
      let Some(filter) = get_str("filter") else {
        return JsonRpcResponse::error(id, -32602, "Missing filter argument");
      };

      if let Err(e) = crate::logging::update_filter(filter) {
        JsonRpcResponse::error(id, -32000, format!("Failed to update filter: {}", e))
      } else {
        let result = CallToolResult {
          content: vec![ToolContent::Text { text: format!("Log filter set to: {}", filter) }],
          is_error: false,
        };
        JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
      }
    }

    "add_overlay" => {
      let Some(widget_id) = get_str("id").map(String::from) else {
        return JsonRpcResponse::error(id, -32602, "Missing required argument: id");
      };
      let color = get_str("color")
        .unwrap_or("#FF000080")
        .to_string();

      match add_overlay_svc(&state, get_window_id(), widget_id.clone(), color).await {
        Ok(()) => {
          let result = CallToolResult {
            content: vec![ToolContent::Text { text: format!("Overlay added to {}", widget_id) }],
            is_error: false,
          };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Failed to add overlay: {}", e)),
      }
    }

    "remove_overlay" => {
      let Some(widget_id) = get_str("id").map(String::from) else {
        return JsonRpcResponse::error(id, -32602, "Missing required argument: id");
      };

      match remove_overlay_svc(&state, get_window_id(), widget_id.clone()).await {
        Ok(()) => {
          let result = CallToolResult {
            content: vec![ToolContent::Text {
              text: format!("Overlay removed from {}", widget_id),
            }],
            is_error: false,
          };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(e) => JsonRpcResponse::error(id, -32000, format!("Failed to remove overlay: {}", e)),
      }
    }

    "clear_overlays" => match clear_overlays_svc(&state, get_window_id()).await {
      Ok(()) => {
        let result = CallToolResult {
          content: vec![ToolContent::Text { text: "Overlays cleared".to_string() }],
          is_error: false,
        };
        JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
      }
      Err(e) => JsonRpcResponse::error(id, -32000, e.to_string()),
    },

    "start_recording" => {
      use std::sync::atomic::Ordering;
      state.recording.store(true, Ordering::Relaxed);
      let result = CallToolResult {
        content: vec![ToolContent::Text { text: "Recording started".to_string() }],
        is_error: false,
      };
      JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
    }

    "stop_recording" => {
      use std::sync::atomic::Ordering;
      state.recording.store(false, Ordering::Relaxed);
      let capture_dir =
        std::env::var("RIBIR_CAPTURE_DIR").unwrap_or_else(|_| "captures".to_string());
      let abs_path = std::fs::canonicalize(&capture_dir)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(capture_dir);
      let result = CallToolResult {
        content: vec![ToolContent::Text {
          text: format!("Recording stopped. Saved to {}", abs_path),
        }],
        is_error: false,
      };
      JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
    }

    "capture_one_shot" => {
      let req: crate::debug_tool::server::CaptureOneShotRequest =
        match serde_json::from_value(params.arguments.unwrap_or(Value::Null)) {
          Ok(r) => r,
          Err(e) => return JsonRpcResponse::error(id, -32602, format!("Invalid params: {}", e)),
        };

      let settle_ms = req.settle_ms.unwrap_or(150);
      let include_images = req.include.iter().any(|s| s == "images");

      match crate::debug_tool::server::capture_start_inner(
        state.clone(),
        req.include,
        req.pre_ms.unwrap_or(2_000),
        req.post_ms.unwrap_or(1_000),
        req.output_dir,
      )
      .await
      {
        Ok(_) => {}
        Err(code) => {
          return JsonRpcResponse::error(
            id,
            -32000,
            format!("Failed to start capture: status {}", code),
          );
        }
      }

      if include_images {
        // Ask the UI thread to redraw once, then wait for the next frame update.
        let _ = state
          .command_tx
          .send(crate::debug_tool::types::DebugCommand::RequestRedraw { window_id: None })
          .await;
        let mut rx = state.last_frame_rx.clone();

        // Wait until we observe at least one frame update (best-effort).
        let _ = tokio::time::timeout(std::time::Duration::from_millis(800), rx.changed()).await;

        // Optional extra settle time to capture any overlays/layout changes.
        tokio::time::sleep(std::time::Duration::from_millis(settle_ms)).await;
      }

      match crate::debug_tool::server::capture_stop_inner(
        state,
        crate::debug_tool::server::CaptureStopRequest { capture_id: None },
      )
      .await
      {
        Ok(axum::Json(resp)) => {
          let result = CallToolResult {
            content: vec![ToolContent::Text {
              text: format!(
                "Capture saved to {}\nManifest: {}",
                resp.capture_dir, resp.manifest_path
              ),
            }],
            is_error: false,
          };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(code) => {
          JsonRpcResponse::error(id, -32000, format!("Failed to stop capture: status {}", code))
        }
      }
    }

    _ => JsonRpcResponse::error(id, -32601, format!("Tool not found: {}", params.name)),
  }
}

async fn handle_read_resource(
  params: Option<Value>, state: Arc<DebugServerState>, id: Option<Value>,
) -> JsonRpcResponse {
  let params: ReadResourceParams = match serde_json::from_value(params.unwrap_or(Value::Null)) {
    Ok(p) => p,
    Err(e) => return JsonRpcResponse::error(id, -32602, format!("Invalid params: {}", e)),
  };

  match params.uri.as_str() {
    "ribir://logs" => {
      let lines = {
        let ring = state.log_ring.lock().await;
        ring.query_lines(None, None, Some(100))
      };
      let text = lines
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<_>>()
        .join("\n");
      let result = ReadResourceResult {
        contents: vec![ResourceContent {
          uri: params.uri,
          mime_type: Some("text/plain".to_string()),
          text: Some(text),
          blob: None,
        }],
      };
      JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
    }
    "ribir://windows" => {
      use tokio::sync::oneshot;
      let (tx, rx) = oneshot::channel();
      let _ = state
        .command_tx
        .send(crate::debug_tool::types::DebugCommand::GetWindows { reply: tx })
        .await;
      match rx.await {
        Ok(windows) => {
          let text = serde_json::to_string_pretty(&windows).unwrap_or_default();
          let result = ReadResourceResult {
            contents: vec![ResourceContent {
              uri: params.uri,
              mime_type: Some("application/json".to_string()),
              text: Some(text),
              blob: None,
            }],
          };
          JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
        }
        Err(_) => JsonRpcResponse::error(id, -32000, "Failed to retrieve windows"),
      }
    }
    "ribir://status" => {
      let ring_len = state.log_ring.lock().await.len();
      let recording = state
        .recording
        .load(std::sync::atomic::Ordering::Relaxed);
      let capture_root = state.capture_root.to_string_lossy().to_string();

      // Construct status JSON
      let status = serde_json::json!({
        "recording": recording,
        "log_sink_connected": crate::logging::debug_log_sender_installed(),
        "filter": crate::logging::current_filter_string(),
        "ring_len": ring_len,
        "capture_root": capture_root,
        // We could add more fields if needed
      });

      let result = ReadResourceResult {
        contents: vec![ResourceContent {
          uri: params.uri,
          mime_type: Some("application/json".to_string()),
          text: Some(status.to_string()),
          blob: None,
        }],
      };
      JsonRpcResponse::result(id, serde_json::to_value(result).unwrap())
    }
    _ => JsonRpcResponse::error(id, -32002, "Resource not found"),
  }
}

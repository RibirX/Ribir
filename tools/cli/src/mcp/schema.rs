use serde_json::Value;

pub const MCP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON schema embedded from ribir_core
const MCP_SCHEMA_JSON: &str = include_str!("../../../../core/src/debug_tool/mcp_schema.json");

/// Get all available MCP tools.
pub fn get_tools() -> Vec<Value> {
  let schema: Value = serde_json::from_str(MCP_SCHEMA_JSON).expect("Failed to parse schema JSON");
  schema["tools"]
    .as_array()
    .cloned()
    .unwrap_or_default()
}

/// Get all available MCP resources.
pub fn get_resources() -> Vec<Value> {
  let schema: Value = serde_json::from_str(MCP_SCHEMA_JSON).expect("Failed to parse schema JSON");
  schema["resources"]
    .as_array()
    .cloned()
    .unwrap_or_default()
}

/// Get fallback initialization result when debug server is not available.
pub fn get_fallback_init_result() -> Value {
  let schema: Value = serde_json::from_str(MCP_SCHEMA_JSON).expect("Failed to parse schema JSON");
  schema["fallback_init_result"].clone()
}

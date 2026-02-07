# Debug Features

Ribir provides a built-in debug server that enables both AI-powered debugging via the Model Context Protocol (MCP) and manual debugging via a web-based HTTP interface.

## 🚀 Getting Started

To enable debug features in your Ribir application:

1. Install `ribir-cli` (once).

```bash
# From the repo root
cargo install --path tools/cli
```
2. Run your app with the `debug` feature enabled:

```bash
cargo run --features debug
```

When the `debug` feature is active:
1. An HTTP debug server starts on `127.0.0.1` and prints the full URL on startup (`RIBIR_DEBUG_URL=...`).
2. Continuous frame capture and log buffering are enabled.
3. The application can interact with MCP-compatible AI assistants.

### Configuration

The debug server can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `RIBIR_CAPTURE_DIR` | Directory where recorded frames and captures are saved | `captures` |

---

> **Port:** The HTTP debug server tries `127.0.0.1:2333` first, then increments until it finds a free port (default range: `2333..2432`). If none are available, it falls back to a dynamic port. Use the startup log (`RIBIR_DEBUG_URL=...`) or the port registry for discovery.

## 🤖 AI Debugging (MCP)

Ribir supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), allowing AI coding assistants like Claude, Codex, or OpenCode to "see" and interact with your running application.

### Setup

Configure your AI client to use the Ribir MCP server. Add the following to your AI client's MCP configuration:

**For Claude Desktop / Claude CLI** (`~/.claude.json` or `claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "ribir-debug": {
      "command": "ribir-cli",
      "args": ["mcp", "serve"]
    }
  }
}
```

**For Codex CLI** (`~/.codex/config.toml`):
```toml
[[mcp.servers]]
name = "ribir-debug"
command = "ribir-cli"
args = ["mcp", "serve"]
```

For other clients, see `tools/cli/README.md` for configuration examples.

The MCP bridge (`ribir-cli mcp serve`) discovers the debug server via the port registry, so you do not need to hardcode a port. Discovery prefers exact path matches and falls back to the nearest parent/child path match.

If no matching debug session exists:
- `ribir-cli mcp check` fails fast with guidance.
- `ribir-cli mcp serve` starts in fallback mode so MCP initialization and tool/resource listing still work.

For MCP clients, prefer calling `start_app` with an explicit `package`, `bin`, or `example` target.

### Key MCP Tools

When connected, the AI can use tools such as:
- `capture_screenshot`: Get a visual of the current app state.
- `inspect_tree`: Read the full widget tree and layout information.
- `inspect_widget`: Get detailed properties of a specific widget.
- `add_overlay`/`remove_overlay`: Visually highlight widgets in the app.
- `set_log_filter`: Dynamically change log levels (e.g., `ribir_core=debug`).

## 🌐 HTTP Debug Server

The debug server provides a REST API and a built-in web UI for manual inspection.

### Built-in Web UI

Open your browser to the URL printed on startup, for example:
`http://127.0.0.1:<port>/ui`

This UI allows you to:
- View live logs.
- Capture screenshots.
- Inspect the widget tree.
- Manage debug overlays.
- Control frame recording.

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/status` | `GET` | Get server status, log filter, and recording state |
| `/windows` | `GET` | List all active application windows |
| `/inspect/tree` | `GET` | Get the full widget tree with layout/global positions |
| `/screenshot` | `GET` | Download a PNG screenshot of the active window |
| `/logs` | `GET` | Get recent logs in NDJSON format |
| `/logs/stream` | `GET` | Real-time log stream via Server-Sent Events (SSE) |

---

## 📹 Advanced Debugging

### Frame Recording

You can record every frame rendered by the application. This is useful for debugging animations or transient layout issues.

- **Via UI**: Toggle the "Recording" checkbox.
- **Via API**: `POST /recording`
- **Output**: PNG frames are saved to the `captures` directory.

### Captures (One-Shot)

A "Capture" is a bundled set of logs and frames surrounding a specific moment. This is what the AI uses to understand a bug report.

- **One-Shot**: `POST /capture/one_shot` captures a short sequence of frames and logs and saves them with a `manifest.json`.

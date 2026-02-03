# Debug Features

Ribir provides a built-in debug server that enables both AI-powered debugging via the Model Context Protocol (MCP) and manual debugging via a web-based HTTP interface.

## 🚀 Getting Started

To enable debug features in your Ribir application, you must run it with the `debug` feature enabled:

```bash
cargo run --features debug
```

When the `debug` feature is active:
1. An HTTP debug server starts on `127.0.0.1:2333` (by default).
2. Continuous frame capture and log buffering are enabled.
3. The application can interact with MCP-compatible AI assistants.

### Configuration

The debug server can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `RIBIR_DEBUG_PORT` | The port the debug server listens on | `2333` |
| `RIBIR_CAPTURE_DIR` | Directory where recorded frames and captures are saved | `captures` |

---

## 🤖 AI Debugging (MCP)

Ribir supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io), allowing AI coding assistants like Claude, Gemini, or OpenCode to "see" and interact with your running application.

### Installation

Use the Ribir CLI to configure your AI clients:

```bash
# Install the MCP adapter and configure all detected AI clients
cargo run -p cli -- mcp install
```

### Key MCP Tools

When connected, the AI can use tools such as:
- `capture_screenshot`: Get a visual of the current app state.
- `inspect_tree`: Read the full widget tree and layout information.
- `inspect_widget`: Get detailed properties of a specific widget.
- `add_overlay`/`remove_overlay`: Visually highlight widgets in the app.
- `set_log_filter`: Dynamically change log levels (e.g., `ribir_core=debug`).

---

## 🌐 HTTP Debug Server

The debug server provides a REST API and a built-in web UI for manual inspection.

### Built-in Web UI

Open your browser to:
[http://localhost:2333/ui](http://localhost:2333/ui)

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

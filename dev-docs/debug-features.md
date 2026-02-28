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

The MCP bridge (`ribir-cli mcp serve`) can discover debug ports via registry, and also supports explicit attach by URL when the client already knows `RIBIR_DEBUG_URL`.

If no matching debug session exists:
- `ribir-cli mcp check` fails fast with guidance.
- `ribir-cli mcp serve` starts in fallback mode so MCP initialization and tool/resource listing still work.

For MCP clients, prefer:
- `start_app` with absolute runnable crate `project_path` (attach-first, then launch if needed).
- `attach_app` with explicit URL when the client already has `RIBIR_DEBUG_URL`.

If users manually launch GUI apps before attach, recommend non-blocking startup commands; foreground `cargo run` blocks until app exit.

### Key MCP Tools

When connected, the AI can use tools such as:
- `start_app`: Attach-first launch by absolute runnable crate `project_path`.
- `attach_app`: Direct attach to explicit debug URL.
- `stop_app`: Stop only process managed by the MCP bridge.
- `capture_screenshot`: Get a visual of the current app state.
- `inspect_tree`: Read the full widget tree and layout information.
- `inspect_widget`: Get detailed properties of a specific widget.
- `add_overlay`/`remove_overlay`: Visually highlight widgets in the app.
- `set_log_filter`: Dynamically change log levels (e.g., `ribir_core=debug`).
- `inject_events`: Inject serialized UI events (cursor/mouse/wheel/keyboard/chars/modifiers) through the same event loop path.
  - Functional events: `click`, `double_click`, `keyboard_input`, `chars`.
  - Low-level events: `cursor_moved`, `mouse_input` (`pressed`/`released`), `raw_keyboard_input` (`pressed`/`released`).
  - Also supports advanced events: `click` and `double_click` (server expands to canonical mouse down/up sequence).
  - `click`/`double_click` can target by coordinates (`x`,`y`) or widget `id` (uses widget center). If both are provided, coordinates win.
  - `keyboard_input` is the default keyboard API (single key stroke: server does press + release, optional `chars`).
  - `raw_keyboard_input` is low-level keyboard API (`physical_key` / `location` / `is_repeat`).
  - `chars` is the quick text input event.
  - For `mouse_input`, you can pass serializable `device_id`: `{ "type": "dummy" }` or `{ "type": "custom", "value": 1 }`.

Example `inject_events` arguments:

```json
{
  "window_id": "1",
  "events": [
    { "type": "cursor_moved", "x": 80, "y": 40 },
    { "type": "mouse_input", "button": "primary", "state": "pressed", "device_id": { "type": "custom", "value": 7 } },
    { "type": "mouse_input", "button": "primary", "state": "released", "device_id": { "type": "custom", "value": 7 } },
    { "type": "chars", "chars": "hello" },
    { "type": "modifiers_changed", "shift": true, "ctrl": false, "alt": false, "logo": false },
    { "type": "mouse_wheel", "delta_x": 0, "delta_y": -1 }
  ]
}
```

Keyboard input sequence:

```json
{
  "events": [
    { "type": "keyboard_input", "key": "a", "chars": "a" },
    { "type": "keyboard_input", "key": "Enter" }
  ]
}
```

Raw keyboard input sequence:

```json
{
  "events": [
    { "type": "raw_keyboard_input", "key": "a", "physical_key": "KeyA", "state": "pressed", "location": "standard", "is_repeat": false },
    { "type": "raw_keyboard_input", "key": "a", "physical_key": "KeyA", "state": "released", "location": "standard" }
  ]
}
```

Minimal tap-like sequence (let framework derive tap from down/up):

```json
{
  "events": [
    { "type": "cursor_moved", "x": 20, "y": 20 },
    { "type": "mouse_input", "button": "primary", "state": "pressed" },
    { "type": "mouse_input", "button": "primary", "state": "released" }
  ]
}
```

Direct advanced click / double click:

```json
{
  "events": [
    { "type": "click", "id": "3:0" },
    { "type": "double_click", "x": 30, "y": 30, "button": "primary", "device_id": { "type": "custom", "value": 9 } }
  ]
}
```

You can also target a widget by its `debug_name` using the `name:` prefix:

```json
{
  "events": [
    { "type": "click", "id": "name:counter_button" }
  ]
}
```

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

For MCP SSE, use:
`http://127.0.0.1:<port>/mcp/sse`

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/status` | `GET` | Get server status, log filter, and recording state |
| `/windows` | `GET` | List all active application windows |
| `/inspect/tree` | `GET` | Get the full widget tree with layout/global positions |
| `/inspect/{id}` | `GET` | Get details for a specific widget by ID |
| `/screenshot` | `GET` | Download a PNG screenshot of the active window |
| `/logs` | `GET` | Get recent logs in NDJSON format |
| `/logs/stream` | `GET` | Real-time log stream via Server-Sent Events (SSE) |
| `/events/inject` | `POST` | Inject input events (mouse/keyboard/wheel) to simulate user interaction |
| `/overlay` | `POST` | Add a visual debug overlay to a widget |
| `/overlays` | `GET` | List all active debug overlays |
| `/overlays` | `DELETE` | Clear all debug overlays |
| `/overlay/{id}` | `DELETE` | Remove a specific debug overlay |
| `/recording` | `POST` | Toggle frame recording (`{"enable": true}`) |
| `/capture/start` | `POST` | Start a capture session |
| `/capture/stop` | `POST` | Stop the active capture session |
| `/capture/one_shot` | `POST` | One-click capture (start → wait → stop) |

`/inspect/tree` and `/inspect/{id}` accept `options` tokens:
`all,id,layout,global_pos,clamp,props,no_global_pos,no_clamp,no_props`.

### Custom debug names (`debug_name`)

You can assign a stable, human-readable debug name to a widget:

```rust
button! {
  debug_name: "counter_button",
  @{ "+1" }
}
```

This works through the built-in `with_debug_name` capability on `FatObj` and
is effective only when the `debug` feature is enabled.

Behavior summary:
- Empty names are ignored.
- Explicit `debug_name` takes priority over auto-resolved type names.
- If no explicit name is provided, Ribir falls back to type-based name
  resolution with internal filtering (for wrappers/builtin intermediate types).

For widget identifier inputs (`id`) in inspect/overlay APIs, use one of:
- `{"index1": <n>, "stamp": <s>}` (full `WidgetId`)
- `<n>:<s>` (colon shorthand, e.g. `3:0`)
- `<n>` (numeric shorthand, matched by `index1`)

`window_id` is optional. If omitted, the first active window is used.
In multi-window scenarios, query `/windows` first and pass `window_id` explicitly.

#### Event Injection (`POST /events/inject`)

Inject UI events to simulate user interaction. **Note**: For HTTP API, `window_id` must be a number (not string). For MCP, use string format.

**Request Body:**
```json
{
  "window_id": 140375628188560,
  "events": [
    { "type": "cursor_moved", "x": 184, "y": 120 },
    { "type": "mouse_input", "button": "primary", "state": "pressed" },
    { "type": "mouse_input", "button": "primary", "state": "released" }
  ]
}
```

**Using the shortcut `click` event:**
```bash
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d '{"window_id": 140375628188560, "events": [{"type": "click", "id": "3:0"}]}'
```

**Response:**
```json
{"accepted": 3}
```

**Event Types:**
| Type | Parameters | Description |
|------|------------|-------------|
| `cursor_moved` | `x`, `y` | Move cursor to position |
| `mouse_input` | `button`, `state` | Press/release mouse button |
| `click` | `x`, `y`, or `id`; optional `button` | Full click (shortcut) |
| `double_click` | `x`, `y`, or `id`; optional `button` | Double click |
| `keyboard_input` | `key`; optional `chars` | Functional single key stroke (press + release, optional text commit via `chars`) |
| `raw_keyboard_input` | `key`, `state`; optional `physical_key`, `location`, `is_repeat`, `chars` | Low-level keyboard event injection |
| `mouse_wheel` | `delta_x`, `delta_y` | Scroll wheel |
| `chars` | `chars` | Type text |
| `modifiers_changed` | `shift`, `ctrl`, `alt`, `logo` | Modifier keys |

---

## 📹 Advanced Debugging

### Frame Recording

You can record every frame rendered by the application. This is useful for debugging animations or transient layout issues.

- **Via UI**: Toggle the "Recording" checkbox.
- **Via API**: `POST /recording`
- **Output**: PNG frames are saved to the `captures` directory.
- **MCP tools**: `start_recording(include?)` creates a capture session directory. `include` accepts `logs` and/or `images` (default: `images`). `stop_recording` returns absolute `capture_dir` and `manifest_path`.

### Captures (One-Shot)

A "Capture" is a bundled set of logs and frames surrounding a specific moment. This is what the AI uses to understand a bug report.

- **One-Shot**: `POST /capture/one_shot` captures a short sequence of frames and logs and saves them with a `manifest.json`.

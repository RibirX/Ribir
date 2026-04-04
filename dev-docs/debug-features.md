# Debug Features

Ribir provides a built-in debug server that enables both AI-powered debugging (via skills) and manual debugging via a web-based HTTP interface.

## Architecture

Both Native and WASM targets use the same unified bridge architecture:

```
App (Native/WASM) --WebSocket--> CLI Debug Server --HTTP--> Debug Tools/UI
```

The app acts as a WebSocket client, connecting to the CLI debug server. This eliminates duplicate HTTP server code and simplifies maintenance.

## 🚀 Getting Started

To enable debug features in your Ribir application:

1. Install `ribir-cli` (once).

```bash
# From the repo root
cargo install --path tools/cli
```

2. Start the debug server:

```bash
cargo run -p ribir-cli -- debug-server
```

The debug server will print the URL on startup:
```
RIBIR_DEBUG_URL=http://127.0.0.1:2333
```

3. Run your app with the `debug` feature enabled:

**Native apps:**
```bash
# Auto-connects to ws://127.0.0.1:2333/ws by default
cargo run --features debug
```

**WASM apps:**
```bash
# Terminal 1: debug server is already running
# Terminal 2: serve the wasm app
cargo run -p ribir-cli -- run-wasm --package <your_package>
```

Then open the wasm page with the debug server HTTP URL injected as a query parameter:
```
http://127.0.0.1:<wasm_port>/?ribir_debug_server=http://127.0.0.1:2333
```

When the `debug` feature is active:
1. The app connects to the CLI debug server via WebSocket.
2. Continuous frame capture and log buffering are enabled.

### Configuration

The debug server can be configured via CLI arguments:

| Argument | Description | Default |
|----------|-------------|---------|
| `--host` | Host to bind | `127.0.0.1` |
| `--port` | Port to bind | `2333` |

Native apps automatically connect to the debug server using the `RIBIR_DEBUG_URL` environment variable (printed on server startup).

WASM apps use the `ribir_debug_server` query parameter with the HTTP URL (automatically converted to WebSocket).

## 🤖 AI Debugging (Skills)

For AI assistants, use the **ribir-debug** skill. The skill provides tools for inspecting, debugging, and interacting with running Ribir applications.

### Prerequisites

Before using the debug skill, make sure:

1. **Debug server is running**: Start it with `cargo run -p ribir-cli -- debug-server`
2. **App is built with debug feature**: Run your app with `--features debug`

### Key Tools

| Category | Tools |
|----------|-------|
| **Lifecycle** | `start_app`, `attach_app`, `stop_app` |
| **Visual** | `capture_screenshot` |
| **Inspection** | `inspect_tree`, `inspect_widget` |
| **Overlays** | `add_overlay`, `remove_overlay`, `clear_overlays`, `get_overlays` |
| **Logging** | `set_log_filter` |
| **Events** | `inject_events` |
| **Recording** | `start_recording`, `stop_recording`, `capture_one_shot` |

#### Widget IDs

Widget identifiers support multiple formats:
- `"3"` - Numeric shorthand (matched by `index1`)
- `"3:0"` - Colon shorthand (`index1:stamp`)
- `'{"index1":3,"stamp":0}'` - Full JSON format

Use `debug_name` prefix to target by name: `"name:counter_button"`

#### Event Injection Examples

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

You can also target a widget by its `debug_name` using the `name:` prefix:

```json
{
  "events": [
    { "type": "click", "id": "name:counter_button" }
  ]
}
```

### Resources

| URI | Description |
|-----|-------------|
| `ribir://logs` | Application logs (NDJSON, ~60s history, 50k cap) |
| `ribir://windows` | Active windows |
| `ribir://status` | Server status |

Read `ribir://windows` to get window IDs for `window_id` parameters.

## 🌐 HTTP Debug Server

The debug server provides a REST API and a built-in web UI for manual inspection.

### Built-in Web UI

Open your browser to the URL printed by the debug server on startup:
```
http://127.0.0.1:<port>/ui
```

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

Inject UI events to simulate user interaction.

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

### Captures (One-Shot)

A "Capture" is a bundled set of logs and frames surrounding a specific moment. This is what the AI uses to understand a bug report.

- **One-Shot**: `POST /capture/one_shot` captures a short sequence of frames and logs and saves them with a `manifest.json`.
- **Debug server**: The debug server writes capture files to disk for both native and WASM sessions, returning `capture_dir` and `manifest_path`.

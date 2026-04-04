---
name: ribir-debug
description: Specialized guide for debugging Ribir applications using the HTTP Debug Tool API. Use this skill when asked to debug, inspect, or interact with running Ribir applications.
---

# Ribir Debug Tool Guide

This guide explains how to use the Ribir Debug Tool HTTP API to debug, inspect, and interact with running Ribir applications.

## 0. Overview

Ribir provides a built-in HTTP debug server that enables:
- **Visual Inspection**: Capture screenshots, inspect widget trees
- **Event Injection**: Simulate user interactions (clicks, keyboard, etc.)
- **Debug Overlays**: Add visual highlights to widgets
- **Logging**: View and filter application logs
- **Frame Recording**: Capture frames for animation/transient issue debugging

### Architecture

```
App (Native/WASM) --WebSocket--> CLI Debug Server --HTTP--> Debug Tools/UI
```

Both Native and WASM targets use the same unified architecture. The app acts as a WebSocket client connecting to the CLI debug server.

## 1. Getting Started

### 1.1 Start the Debug Server

Before running any app with debug feature, start the debug server:

```bash
cargo run -p ribir-cli -- debug-server
```

The debug server prints its URL on startup:
```
RIBIR_DEBUG_URL=http://127.0.0.1:2333
```

**Important:** The actual port may differ if the default port is in use. Always use the printed `RIBIR_DEBUG_URL` value.

### 1.2 Native Debugging

Run your Ribir application with the `debug` feature. The app will automatically connect to the debug server:

```bash
cargo run --features debug

# Or with specific package:
cargo run -p your_package --features debug
```

To use a custom debug server URL:
```bash
RIBIR_DEBUG_URL=http://127.0.0.1:8080 cargo run -p your_package --features debug
```

### 1.3 WASM Debugging

Use `ribir-cli run-wasm` with the `--debug` flag:

```bash
cargo run -p ribir-cli -- run-wasm --package your_package --debug
```

The debug server URL is automatically injected into the HTML as a query parameter:
```
http://127.0.0.1:8000/?ribir_debug_server=http://127.0.0.1:2333
```

Open the browser and the WASM app will automatically connect to the debug server.

### 1.4 Port Assignment Logic

**Debug Server:**
- Default port: 2333
- Tries up to 100 ports (2333-2432)
- Falls back to a dynamic port if all are busy
- Prints the actual URL on startup

**Important:** Always use the printed `RIBIR_DEBUG_URL` rather than assuming a specific port.

### 1.5 Configuration

The debug server can be configured via CLI arguments:

| Argument | Description | Default |
|----------|-------------|---------|
| `--host` | Host to bind | `127.0.0.1` |
| `--port` | Port to bind | `2333` |

## 2. HTTP API Reference

Base URL: `http://127.0.0.1:<port>` (use the printed `RIBIR_DEBUG_URL`)

### 2.1 Status & Windows

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | `GET` | Debug UI page (web interface) |
| `/ui` | `GET` | Debug UI page (web interface) |
| `/status` | `GET` | Server status, log filter, recording state |
| `/windows` | `GET` | List all active windows |

**Example: Get Status**
```bash
curl http://127.0.0.1:2333/status
```

**Response:**
```json
{
  "recording": false,
  "log_sink_connected": true,
  "filter_reload_installed": true,
  "filter": "info",
  "dropped_total": 0,
  "ring_len": 42,
  "capture_root": "captures",
  "active_capture": null,
  "active_macro_recording": null
}
```

**Example: Get Windows**
```bash
curl http://127.0.0.1:2333/windows
```

**Response:**
```json
[
  {
    "id": 140375628188560,
    "title": "My App",
    "width": 800,
    "height": 600
  }
]
```

**Note:** Use the `id` from `/windows` as `window_id` in other endpoints (like `/events/inject`, `/overlay`, etc.) for multi-window apps.

### 2.2 Widget Inspection

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/inspect/tree` | `GET` | Full widget tree with layout info |
| `/inspect/{id}` | `GET` | Details for a specific widget |

**Query Parameters:**
- `options`: Comma-separated tokens: `all`, `id`, `layout`, `global_pos`, `clamp`, `props`, `no_global_pos`, `no_clamp`, `no_props`

**Example: Inspect Widget Tree**
```bash
curl "http://127.0.0.1:2333/inspect/tree?options=all"
```

### 2.3 Screenshots

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/screenshot` | `GET` | PNG screenshot of the active window |

**Example:**
```bash
curl -o screenshot.png http://127.0.0.1:2333/screenshot
```

### 2.4 Debug Overlays

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/overlay` | `POST` | Add visual overlay to a widget |
| `/overlays` | `GET` | List all active overlays |
| `/overlays` | `DELETE` | Clear all overlays |
| `/overlay/{id}` | `DELETE` | Remove specific overlay |

**Request Body for `/overlay`:**
```json
{
  "window_id": null,
  "id": "3:0",
  "color": "#FF000080"
}
```

**Note:** `window_id` is optional. If omitted, the first active window is used. For multi-window apps, query `/windows` first to get the correct window ID.

**Example: Add Overlay**
```bash
curl -X POST http://127.0.0.1:2333/overlay \
  -H "Content-Type: application/json" \
  -d '{"id": "3:0", "color": "#FF000080"}'
```

### 2.5 Event Injection

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/events/inject` | `POST` | Inject UI events |

**Request Body:**
```json
{
  "window_id": null,
  "events": [
    { "type": "click", "id": "3:0" }
  ]
}
```

**Note:** `window_id` is optional (uses first window if omitted). The `accepted` field indicates how many events were actually processed (may be less than sent if some fail).

**Response:**
```json
{"accepted": 1}
```

### 2.6 Logging

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/logs` | `GET` | Recent logs (NDJSON) |
| `/logs/stream` | `GET` | Real-time log stream (SSE) |
| `/logs/filter` | `POST` | Set log filter |

**Query Parameters for `/logs`:**
- `since_ts`: Unix timestamp in milliseconds
- `until_ts`: Unix timestamp in milliseconds
- `limit`: Maximum number of log lines

**Response Headers for `/logs`:**
- `X-Ribir-Log-Dropped`: Number of logs dropped due to ring buffer overflow (indicates data loss)

**Example: Get Logs**
```bash
curl http://127.0.0.1:2333/logs?limit=50
```

**Example: Set Filter**
```bash
curl -X POST http://127.0.0.1:2333/logs/filter \
  -H "Content-Type: application/json" \
  -d '{"filter": "debug,ribir_core=trace"}'
```

### 2.7 Recording & Capture

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/recording` | `POST` | Toggle frame recording |
| `/capture/start` | `POST` | Start capture session |
| `/capture/stop` | `POST` | Stop capture session |
| `/capture/one_shot` | `POST` | One-click capture |

### 2.8 Event Macro Recording

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/events/macro/start` | `POST` | Start recording user interaction events |
| `/events/macro/stop` | `POST` | Stop recording and return the macro |

**Start Recording (Async Mode):**
```bash
# Async mode: starts recording, returns immediately with recording_id
curl -X POST http://127.0.0.1:2333/events/macro/start \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Response (Async Mode):**
```json
{
  "recording_id": "macro_1234567890_0",
  "started_at_ts_unix_ms": 1234567890123
}
```

**Start with Auto-Stop (Sync Mode):**
```bash
# Sync mode: waits for duration_ms, then returns recorded events
curl -X POST http://127.0.0.1:2333/events/macro/start \
  -H "Content-Type: application/json" \
  -d '{"duration_ms": 10000}'
```

**Response (Sync Mode - waits for recording to complete):**
```json
{
  "recording_id": "macro_1234567890_0",
  "events": [
    {"type": "click", "id": "3:0"},
    {"type": "delay", "ms": 100},
    {"type": "chars", "chars": "hello"}
  ],
  "duration_ms": 10000
}
```

**Key Difference:**
- **Async mode** (no `duration_ms`): Returns immediately with `recording_id` and `started_at_ts_unix_ms`. You must call `/events/macro/stop` later.
- **Sync mode** (with `duration_ms`): Waits for the specified duration, then returns the complete recording with `events` and `duration_ms`. No need to call stop.

**Stop Recording (Async Mode Only):**
```bash
curl -X POST http://127.0.0.1:2333/events/macro/stop \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Response:**
```json
{
  "recording_id": "macro_1234567890_0",
  "events": [
    {"type": "click", "id": "3:0"},
    {"type": "delay", "ms": 100},
    {"type": "chars", "chars": "hello"}
  ],
  "duration_ms": 200
}
```

The `events` array contains **replay-ready events** with automatic `delay` insertions to preserve the original timing between user interactions.

`duration_ms` reports the full wall-clock recording duration. For timed auto-stop, it should match the configured duration (plus small scheduling jitter), not the timestamp of the last recorded event.

**Replay:** The `events` array in the response is **replay-ready** and contains `delay` events to preserve the original timing. You can pass it directly to `/events/inject`:

```bash
# Step 1: Record and stop
RESPONSE=$(curl -s -X POST http://127.0.0.1:2333/events/macro/stop \
  -H "Content-Type: application/json" -d '{}')

# Step 2: Extract events and replay directly (includes timing delays)
EVENTS=$(echo "$RESPONSE" | jq '.events')
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d "{\"events\": $EVENTS}"
```

Or with the debug UI: click **Start** → interact with the app → click **Stop** → the events will be displayed with timing information. Click the **Download Events** button to save them as JSON. The downloaded JSON can be passed directly to `/events/inject` for replay with preserved timing.

**Use Cases:**
- Create reproducible test scenarios
- Automate repetitive debugging tasks
- Document bug reproduction steps

## 3. Widget ID Formats

Widget identifiers support multiple formats:

| Format | Example | Description |
|--------|---------|-------------|
| Numeric | `"3"` | Matched by `index1` |
| Colon shorthand | `"3:0"` | `index1:stamp` |
| Full JSON | `'{"index1":3,"stamp":0}'` | Full `WidgetId` |
| Debug name | `"name:counter_button"` | By `debug_name` |

### Using debug_name

Assign stable names to widgets for easier targeting:

```rust
button! {
  debug_name: "counter_button",
  @{ "+1" }
}
```

Then use in API calls:
```bash
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d '{"events": [{"type": "click", "id": "name:counter_button"}]}'
```

## 4. Event Types Reference

### 4.1 Mouse Events

```json
// Move cursor
{ "type": "cursor_moved", "x": 100, "y": 200 }

// Cursor leaves window
{ "type": "cursor_left" }

// Mouse button (button: primary|secondary|auxiliary|fourth|fifth)
{ "type": "mouse_input", "button": "primary", "state": "pressed" }
{ "type": "mouse_input", "button": "primary", "state": "released" }

// Scroll
{ "type": "mouse_wheel", "delta_x": 0, "delta_y": -10 }

// Click shortcut (auto-generates press + release)
{ "type": "click", "id": "3:0" }
{ "type": "click", "x": 100, "y": 200 }

// Double click
{ "type": "double_click", "id": "name:my_button" }
```

### 4.2 Keyboard Events

```json
// Functional keyboard input (press + release + optional chars)
{ "type": "keyboard_input", "key": "a", "chars": "a" }
{ "type": "keyboard_input", "key": "Enter" }

// Raw keyboard input (low-level control)
{ "type": "raw_keyboard_input", "key": "a", "physical_key": "KeyA", "state": "pressed" }
{ "type": "raw_keyboard_input", "key": "a", "physical_key": "KeyA", "state": "released" }

// Quick text input
{ "type": "chars", "chars": "hello world" }

// Modifiers
{ "type": "modifiers_changed", "shift": true, "ctrl": false, "alt": false, "logo": false }
```

**Raw Keyboard Input Fields:**
- `key`: Virtual key name (e.g., "a", "Enter", "Space")
- `physical_key`: Optional W3C physical key code (e.g., "KeyA", "Digit1", "Enter")
- `state`: "pressed" or "released"
- `is_repeat`: Optional, whether this is a key repeat event (default: false)
- `location`: Optional key location: "standard", "left", "right", "numpad" (default: "standard")
- `chars`: Optional characters to receive on press

### 4.3 Utility Events

```json
// Delay between events
{ "type": "delay", "ms": 100 }

// Request redraw
{ "type": "redraw_request", "force": true }
```

## 5. Common Debugging Workflows

### 5.1 Inspect Widget Hierarchy

```bash
# Get the widget tree
curl http://127.0.0.1:2333/inspect/tree | jq .

# Find a specific widget
curl http://127.0.0.1:2333/inspect/3:0 | jq .
```

### 5.2 Visual Debugging with Overlays

```bash
# Highlight a widget
curl -X POST http://127.0.0.1:2333/overlay \
  -H "Content-Type: application/json" \
  -d '{"id": "3:0", "color": "#FF000080"}'

# Take a screenshot to see the overlay
curl -o debug.png http://127.0.0.1:2333/screenshot

# Clear overlays when done
curl -X DELETE http://127.0.0.1:2333/overlays
```

### 5.3 Simulate User Interaction

```bash
# Click a button by debug name
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d '{"events": [{"type": "click", "id": "name:submit_button"}]}'

# Type text into an input field
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d '{"events": [{"type": "chars", "chars": "Hello, World!"}]}'

# Complex interaction sequence
curl -X POST http://127.0.0.1:2333/events/inject \
  -H "Content-Type: application/json" \
  -d '{
    "events": [
      { "type": "cursor_moved", "x": 100, "y": 100 },
      { "type": "delay", "ms": 100 },
      { "type": "mouse_input", "button": "primary", "state": "pressed" },
      { "type": "delay", "ms": 50 },
      { "type": "mouse_input", "button": "primary", "state": "released" }
    ]
  }'
```

### 5.4 Debug Logging

```bash
# Increase log verbosity
curl -X POST http://127.0.0.1:2333/logs/filter \
  -H "Content-Type: application/json" \
  -d '{"filter": "debug,ribir_core=trace"}'

# Stream logs in real-time
curl -N http://127.0.0.1:2333/logs/stream
```

### 5.5 Capture for Bug Reports

```bash
# One-shot capture (logs + frames)
curl -X POST http://127.0.0.1:2333/capture/one_shot \
  -H "Content-Type: application/json" \
  -d '{"include": ["logs", "images"], "pre_ms": 2000, "post_ms": 1000}'

# One-shot capture with settle time (wait for frame to stabilize)
curl -X POST http://127.0.0.1:2333/capture/one_shot \
  -H "Content-Type: application/json" \
  -d '{"include": ["logs", "images"], "pre_ms": 2000, "post_ms": 1000, "settle_ms": 150}'

# Response contains the capture directory
# {"capture_dir": "/path/to/captures/{capture_id}", "manifest_path": "..."}
```

**Capture One-Shot Parameters:**
- `include`: Array of "logs" and/or "images"
- `pre_ms`: Logs captured from (now - pre_ms) (default: 2000)
- `post_ms`: Logs captured until (now + post_ms) (default: 1000)
- `settle_ms`: Extra time to wait after frame update (default: 150)
- `output_dir`: Optional custom output directory

## 6. Tips & Best Practices

### 6.1 Use debug_name Proactively

When debugging, add `debug_name` to target widgets:

```rust
@Button {
  debug_name: "login_button",
  on_tap: move |_| { /* ... */ },
  @{ "Login" }
}
```

This makes event injection much more reliable:
```json
{ "type": "click", "id": "name:login_button" }
```

### 6.2 Verify Widget IDs

Before injecting events, verify the widget exists:

```bash
# Check if widget exists
curl http://127.0.0.1:2333/inspect/name:my_button
```

### 6.3 Use Overlays for Visual Confirmation

Before clicking, add an overlay to confirm the target:

```bash
curl -X POST http://127.0.0.1:2333/overlay \
  -H "Content-Type: application/json" \
  -d '{"id": "name:my_button", "color": "#00FF0080"}'

# Then capture screenshot to verify
curl -o check.png http://127.0.0.1:2333/screenshot
```

### 6.4 Check Server Status

Always verify the debug server is responsive:

```bash
curl http://127.0.0.1:2333/status
```

## 7. Troubleshooting

### 7.1 No Logs Captured

If `ring_len` is 0 in status:
1. Check if `log_sink_connected` is `true`
2. Verify tracing subscriber was initialized (check console for warnings)
3. Try setting a more permissive filter

### 7.2 Event Injection Not Working

1. Verify the widget ID format is correct
2. Check if the widget exists using `/inspect/{id}`
3. Use overlays to visually confirm the target

### 7.3 Screenshot Timeout

If screenshots timeout:
1. The application may be unresponsive
2. Try requesting a redraw first: `{"type": "redraw_request", "force": true}`

### 7.4 Widget Not Found

Widget IDs can change between frames if the widget tree is rebuilt. Use `debug_name` for stable identifiers, or re-query the widget tree before each operation.

### 7.5 Debug Server Not Starting

If the debug server fails to start:
1. Check if port 2333 is already in use
2. Try a different port: `cargo run -p ribir-cli -- debug-server --port 3000`
3. Check the printed `RIBIR_DEBUG_URL` for the actual port

### 7.6 WASM Connection Issues

1. Ensure the debug server is running (`cargo run -p ribir-cli -- debug-server`)
2. Check that the `ribir_debug_server` query parameter is present in the URL
3. Check browser console for connection errors
4. Verify the WebSocket URL matches the debug server's printed `RIBIR_DEBUG_URL`

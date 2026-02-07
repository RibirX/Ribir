## Cli for ribir

### Install

```bash
# From the repo root
cargo install --path tools/cli
```

### SubCommand

#### run-wasm

build the example to wasm

1. Compile to target wasm32-unknown-unknown
2. Use wasm-bindgen to export relative function to js
3. Serve the wasm in 127.0.0.1:8000 by simpl-http-server

#### bundle

Bundle the native app for distribution.

##### Quick Start

```bash
# In your app's directory
cd examples/counter

# Build and bundle (default behavior)
ribir-cli bundle

# With custom config file
ribir-cli bundle -c path/to/bundle.toml
```

##### Subcommands

| Command | Description |
|---------|-------------|
| `bundle` | Build and package the application (default) |
| `bundle build` | Only build the application (do not package) |
| `bundle pack` | Only package the application (assumes already built) |

##### Options

| Option | Description |
|--------|-------------|
| `-c, --config <PATH>` | Path to bundle config file (defaults to Cargo.toml) |
| `--profile <PROFILE>` | Cargo profile to use. Auto-detects `[profile.bundle]` if exists, otherwise uses `release` |
| `--clean` | Clean bundle artifacts before building |
| `-t, --target-dir <PATH>` | Custom target directory |
| `-v, --verbose` | Enable verbose output |

##### Examples

```bash
# Default: build + package with auto-detected profile
ribir-cli bundle

# Use a specific profile
ribir-cli bundle --profile release

# Clean build
ribir-cli bundle --clean

# Only build (for CI/CD pipelines)
ribir-cli bundle build

# Only package (assumes already built)
ribir-cli bundle pack

# Combine options
ribir-cli bundle build --profile dev --clean
ribir-cli bundle pack --profile release
```

##### Profile Detection

The bundle command automatically detects the Cargo profile to use:

1. If `--profile` is specified, use that profile
2. If `[profile.bundle]` exists in Cargo.toml (package or workspace), use `bundle` profile
3. Otherwise, use `release` profile

You can define a custom bundle profile in your Cargo.toml:

```toml
[profile.bundle]
inherits = "release"
lto = true
strip = true
opt-level = "z"  # Optimize for size
```

##### Bundle Config File

The bundle config can be placed in:
- `[package.metadata.bundle]` section in Cargo.toml
- A separate TOML file specified with `--config`

Example configuration (in Cargo.toml):

```toml
[package.metadata.bundle]
productName = "Counter"
version = "1.0.0"
identifier = "com.ribir.counter"
shortDescription = ""
longDescription = ""
copyright = "Copyright (c) You 2021. All rights reserved."
icon = ["../Logo.ico"]
resources = []
externalBin = []
```

Note that this is just an example, and the actual configuration will depend on the specific requirements of your application. For more details, you can refer to the `BundleConfig` struct in the cli crate.

**Path Resolution**: Relative paths in the config file (such as `icon`, `resources`, `licenseFile`) are resolved relative to the config file's directory, not the current working directory. This makes it easier to organize your bundle configuration and assets together.

##### Asset Integration

The bundle command automatically detects and includes assets processed by the `asset!` macro:

1. During build, assets are copied to `target/<profile>/assets/`
2. The bundle command detects this directory and includes it in the package
3. At runtime, assets are loaded relative to the executable

No additional configuration is needed - just use `asset!("path/to/file")` in your code and the bundler will handle the rest.

#### mcp

Start MCP (Model Context Protocol) stdio server for AI coding assistants to debug Ribir applications.

##### What is MCP?

MCP (Model Context Protocol) is a standard that allows AI assistants to interact with external tools and services. The Ribir MCP server provides a native Rust stdio server that enables AI clients to inspect and debug running Ribir applications in real-time.

##### Automatic Port Discovery

The MCP server supports **automatic port discovery** so the CLI can find the running debug server for whichever project you're in:

- When a Ribir app starts with `--features debug`, it binds to `127.0.0.1:2333` and increments until it finds a free port (fallbacks to a dynamic port), then registers the chosen port in `~/.local/state/ribir/debug-ports/`
- The MCP server checks the registry for the best path match relative to the current working directory (exact match first, otherwise nearest parent/child path match)
- `mcp check` fails fast when no matching session is found; `mcp serve` still starts in fallback mode so MCP handshake and tool discovery continue to work

```bash
# Terminal 1: Debug project A (auto-discovered port)
cd ~/projects/app-a
cargo run --features debug
# Output: Debug server listening on http://127.0.0.1:2333

# Terminal 2: Debug project B (auto-discovered port)
cd ~/projects/app-b
cargo run --features debug
# Output: Debug server listening on http://127.0.0.1:2334

# AI clients in each project directory will auto-discover the correct port
```

##### Available Tools

When connected, the AI assistant can use these tools:

| Tool | Description |
|------|-------------|
| `capture_screenshot` | Capture a screenshot of the application window |
| `inspect_tree` | Get the widget tree structure and layout information |
| `inspect_widget` | Get detailed information about a specific widget |
| `get_overlays` | List all active debug overlays |
| `add_overlay` | Highlight a widget with a colored overlay |
| `remove_overlay` | Remove a specific overlay |
| `clear_overlays` | Clear all overlays |
| `set_log_filter` | Set the log filter (e.g., `info,ribir_core=debug`) |
| `start_recording` | Start recording frames to disk |
| `stop_recording` | Stop recording and save to disk |
| `capture_one_shot` | Capture a single sequence of frames (pre/post trigger) |

##### Available Resources

| Resource | URI | Description |
|----------|-----|-------------|
| Application Logs | `ribir://logs` | Recent application logs (last 100 lines) |
| Window List | `ribir://windows` | List of active windows |
| Server Status | `ribir://status` | Debug server status (recording, filter, stats) |

##### Quick Start

```bash
# 1. Run your Ribir app with debug feature enabled
cargo run --features debug

# 2. Configure your AI client to use the MCP server
# The port is auto-discovered - no env var needed!
```

##### Configuration Examples

###### Claude Desktop / Claude CLI

Add to `~/.claude.json` or `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ribir-debug": {
      "command": "cargo",
      "args": ["run", "-p", "cli", "--", "mcp", "serve"]
    }
  }
}
```

###### OpenCode CLI

Add to `~/.config/opencode/opencode.json`:

```json
{
  "mcp": {
    "ribir-debug": {
      "type": "local",
      "command": ["cargo", "run", "-p", "cli", "--", "mcp", "serve"],
      "enabled": true
    }
  }
}
```

###### Codex CLI

Add to `~/.codex/config.toml`:

```toml
[[mcp.servers]]
name = "ribir-debug"
command = "cargo"
args = ["run", "-p", "cli", "--", "mcp", "serve"]
```

##### Subcommands

| Command | Description |
|---------|-------------|
| `mcp serve` | Start MCP stdio server (used by AI clients) |
| `mcp check` | Check connection to the Ribir debug server |
| `mcp list` | List all active debug sessions |

##### Options

**serve options:**

| Option | Description |
|--------|-------------|
| `-p, --port <PORT>` | Override auto-discovered port |

**check options:**

| Option | Description |
|--------|-------------|
| `-p, --port <PORT>` | Override auto-discovered port |

##### Examples

```bash
# List all active debug sessions
ribir-cli mcp list

# Test connection to auto-discovered debug server
ribir-cli mcp check

# Start MCP server with auto-discovery
ribir-cli mcp serve

# Override with a specific port
ribir-cli mcp serve --port=8080
```

##### How It Works

```
┌─────────────────┐     stdio      ┌──────────────────┐     HTTP     ┌─────────────────┐
│   AI Client     │◄──────────────►│  Rust MCP Server │◄───────────►│  Ribir App      │
│ (Claude/Codex)  │               │  (cli mcp serve) │  :auto     │  (debug server) │
└─────────────────┘               └──────────────────┘            └─────────────────┘
```

1. **Run**: Start your Ribir app with `--features debug` - it binds to `127.0.0.1:2333` and increments until a free port is found, then auto-registers that port
2. **Configure**: Add `ribir-debug` server entry to your AI client's config (no port needed)
3. **Connect**: AI client launches `cli mcp serve` which auto-discovers the port
4. **Forward**: The MCP server forwards JSON-RPC requests to the debug HTTP server
5. **Respond**: Responses flow back through the MCP server to the AI client

##### Multi-Project Debugging

When debugging multiple Ribir projects simultaneously:

1. Each project's debug server registers its port with the project path as key
2. The MCP server discovers the port using the best path match (exact first, otherwise nearest parent/child match)
3. No configuration changes needed - just run from the correct directory

```bash
# View all active sessions
cargo run -p ribir-cli -- mcp list

# Output:
# Active debug sessions:
#
#   Port: 2333
#   Path: /Users/you/projects/app-a
#   PID:  12345
#
#   Port: 2334
#   Path: /Users/you/projects/app-b
#   PID:  12346
```

##### Fallback Mode

If the debug server is not running, the MCP server operates in fallback mode:
- Tool/resource discovery still works (via `initialize`, `tools/list`, `resources/list`)
- Tool invocations return helpful error messages with setup instructions
- This allows AI clients to discover available capabilities even when the app isn't running

##### Troubleshooting: Session Not Found or Wrong Port

1. Preferred for MCP clients: call `start_app` with an explicit target (`package`, `bin`, or `example`)
2. Run `ribir-cli mcp list` to inspect which paths and ports are currently registered
3. If needed, force the port via `ribir-cli mcp serve --port <PORT>`

`start_app` now requires an explicit target to avoid ambiguous `cargo run` behavior in workspace roots.

## Cli for ribir

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
cargo run -p cli -- bundle

# With custom config file
cargo run -p cli -- bundle -c path/to/bundle.toml
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
cargo run -p cli -- bundle

# Use a specific profile
cargo run -p cli -- bundle --profile release

# Clean build
cargo run -p cli -- bundle --clean

# Only build (for CI/CD pipelines)
cargo run -p cli -- bundle build

# Only package (assumes already built)
cargo run -p cli -- bundle pack

# Combine options
cargo run -p cli -- bundle build --profile dev --clean
cargo run -p cli -- bundle pack --profile release
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

Configure MCP (Model Context Protocol) for AI coding assistants to debug Ribir applications.

##### What is MCP?

MCP (Model Context Protocol) is a standard that allows AI assistants to interact with external tools and services. The Ribir Debug MCP server enables AI clients to inspect and debug running Ribir applications in real-time.

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

# 2. In another terminal, install and configure MCP
cargo run -p cli -- mcp install

# 3. Use your AI client with MCP tools
# Example: "Use ribir-debug to capture a screenshot"
```

##### Subcommands

| Command | Description |
|---------|-------------|
| `mcp install` | Install adapter to `~/.ribir/` and configure AI clients |
| `mcp upgrade` | Upgrade adapter files in `~/.ribir/` |
| `mcp status` | Show current MCP configuration status |

##### Install Options

| Option | Description |
|--------|-------------|
| `-c, --client <CLIENT>` | Target AI client: `auto`, `claude-cli`, `opencode`, `gemini` |
| `-p, --port <PORT>` | Ribir debug server port (default: `2333`) |
| `--dry-run` | Show what would be written without making changes |
| `--skip-adapter` | Skip copying adapter files to `~/.ribir/` |

##### Supported AI Clients

| Client | Config Path |
|--------|-------------|
| `claude-cli` | Claude CLI (`~/.claude.json`) |
| `opencode` | OpenCode CLI (`~/.config/opencode/opencode.json`) |
| `gemini` | Gemini CLI (`~/.gemini/settings.json`) |

##### Examples

```bash
# Auto-detect and configure all installed AI clients
cargo run -p cli -- mcp install

# Configure only OpenCode
cargo run -p cli -- mcp install --client=opencode

# Use a custom port
cargo run -p cli -- mcp install --port=8080

# Preview what would be written
cargo run -p cli -- mcp install --dry-run --client=opencode
```

##### How It Works

```
┌─────────────────┐     stdio      ┌──────────────────┐     HTTP     ┌─────────────────┐
│   AI Client     │◄──────────────►│   Node Adapter   │◄───────────►│  Ribir App      │
│ (Claude/OpenCode)│               │  (mcp-adapter.js) │  :2333     │  (debug server) │
└─────────────────┘               └──────────────────┘            └─────────────────┘
                                          │
                                          │ loads schema
                                          ▼
                                   ~/.ribir/
                                   ├── mcp-adapter.js
                                   └── mcp_schema.json
```

1. **Install**: Copies `mcp-adapter.js` and `mcp_schema.json` to `~/.ribir/`
2. **Configure**: Adds `ribir-debug` server entry to each AI client's config
3. **Run**: Start your Ribir app with `--features debug` to enable the debug server
4. **Use**: AI client communicates with the adapter via stdio, adapter forwards requests to the debug server
5. **Upgrade**: Run `mcp upgrade` to update adapter files after updating Ribir

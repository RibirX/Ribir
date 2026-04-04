## Cli for ribir

### Install

```bash
# From the repo root
cargo install --path tools/cli
```

### SubCommand

#### run-wasm

Build and serve a Ribir project as WebAssembly.

##### Quick Start

```bash
# Basic usage
cargo run -p ribir-cli -- run-wasm --package counter

# Debug mode (with bridge server)
cargo run -p ribir-cli -- run-wasm --package counter --debug

# With custom template
cargo run -p ribir-cli -- run-wasm --package counter --template path/to/template
```

##### Options

| Option | Description |
|--------|-------------|
| `-p, --package <NAME>` | Package name to build (required) |
| `-n, --name <NAME>` | Output name, default: `web_wasm` |
| `-o, --out-dir <PATH>` | Output directory, default: `target/wasm` |
| `-r, --release` | Build in release mode |
| `--no-server` | Build only, don't start HTTP server |
| `-t, --template <PATH>` | Custom template file/directory |
| `--host <HOST>` | HTTP server host, default: `127.0.0.1` |
| `--port <PORT>` | HTTP server port, default: `8000` |
| `--debug` | Enable debug features and start bridge server |
| `--bridge-host <HOST>` | Bridge server host (debug mode), default: `127.0.0.1` |
| `--bridge-port <PORT>` | Bridge server port (debug mode), default: `2333` |

##### How It Works

1. **Compile**: Builds the package for `wasm32-unknown-unknown` target
2. **Bindgen**: Uses `wasm-bindgen` to generate JS bindings
3. **Template**: Copies user template (if provided) or uses built-in template
4. **Debug Injection** (optional): In `--debug` mode, automatically injects bridge connection script
5. **Serve**: Starts HTTP server with CORS headers for WASM SharedArrayBuffer support

##### HTML Template

The CLI generates `index.html` using this priority:

1. **User Template** (if `--template` provided and contains `index.html`)
2. **Built-in Template** (`tools/cli/template/index.html`)

In debug mode, the bridge WebSocket URL is automatically injected.

##### Debug Mode

When `--debug` is enabled:

- Starts a bridge server for WASM debugging
- Injects connection script into HTML
- Enables debugging tools for browser-based inspection

```bash
# Start with debug bridge
cargo run -p ribir-cli -- run-wasm --package my_app --debug

# App will be available at http://127.0.0.1:8000
# Bridge server prefers ws://127.0.0.1:2333/ws, but the injected page uses the actual bound URL
```

The injected script automatically connects to the bridge, uses the bridge's real runtime URL if the preferred port is unavailable, and sets `RIBIR_DEBUG_URL` for debugging tools.

For wasm debug sessions, `/status`, `/logs/filter`, and capture endpoints are proxied through the bridge to the connected page. Capture artifacts are written by the bridge host and returned as server-side `capture_dir` / `manifest_path` paths.

##### Custom Template

You can provide your own HTML template:

```bash
# Single HTML file
cargo run -p ribir-cli -- run-wasm --package my_app --template custom.html

# Directory with assets
cargo run -p ribir-cli -- run-wasm --package my_app --template ./my-template/
```

The template should include:

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>My App</title>
  <style>
    .ribir_container {
      width: 100vw;
      height: 100vh;
      position: fixed;
      top: 0;
      left: 0;
    }
  </style>
</head>
<body>
  <div class="ribir_container"></div>
  <script type="module">
    import init, { run } from './web_wasm.js';
    await init();
    run();
  </script>
</body>
</html>
```

**Important**: The container element must have `class="ribir_container"` (not `id`) for the WASM runtime to find it.

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

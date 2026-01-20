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

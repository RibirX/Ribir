
# Pomodoro Example

This is a small Pomodoro timer example built with Ribir. It demonstrates a focused work/rest timer with a minimal UI and the ability to package the app into a platform installer using the workspace `cli` bundler.

**What it is:** A simple Pomodoro-style timer app for learning and demonstrating Ribir features (timers, animations, state, and basic app packaging).

**Key features:**
- Work/Break cycles (start, pause, reset)
- Configurable durations for work and break periods
- Minimal, responsive UI built with Ribir components
- Can be bundled into a platform installer using the project `cli` bundle command

**Run (development)**
- From the repository root (recommended):

```powershell
cargo run -p pomodoro
```

- Or from the example directory:

```powershell
cd examples/pomodoro
cargo run
```

This runs the example in debug mode. Use the UI controls to start/pause/reset the timer.

**Build (release)**

To build an optimized release binary:

```powershell
cargo build --package pomodoro --release
```

The release binary will be in the usual `target/release` folder (or the workspace target directory).

**Create an installer / bundle**

The repository includes a small CLI bundler (the `cli` package) that can create platform installers or application bundles. To invoke it from the repository root run:

```powershell
cargo run --package cli -- bundle --verbose
```

- The `--verbose` flag prints detailed logs from the bundling process.
- The exact produced artifact path and format depends on your OS and the bundler configuration; inspect the CLI output to find the generated installer or bundle location (commonly under `target/` or a `dist/`-like folder).
- If the bundler needs additional platform tools (code signing tools, native packagers, etc.), the CLI will log helpful errors — check `cli/README.md` for platform-specific prerequisites.

**Troubleshooting & tips**
- If bundling fails, first build the release binary, then re-run the bundle command:

```powershell
cargo build --package pomodoro --release
cargo run --package cli -- bundle --verbose
```

- Check `cli/README.md` for extra bundler configuration options and platform requirements.
- For development, use `cargo run -p pomodoro` to iterate quickly.

**Where to look next**
- Example source: `examples/pomodoro/src`
- CLI bundler code: `cli/src`
- If you want, I can add a short GIF or screenshot and an example `bundle` configuration snippet to this README.



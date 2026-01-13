# Ribir Docker Environment

Docker images for Ribir development and CI. Images are **automatically managed** based on version numbers in the source code.

## Quick Start

### For Developers (Linux/Windows)

The easiest way to use Docker is via the `ci.rs` script:

```bash
# Start an interactive shell in the Docker environment
./tools/ci.rs docker dev

# Run a specific CI command in Docker
./tools/ci.rs docker lint
./tools/ci.rs docker test
```

### For CI (GitHub Actions)

CI workflows use these images directly via the `container` field in `.yml` files.

## Automatic Version Management

Docker image versions are automatically extracted from source code:

- **Stable version**: From `Cargo.toml` → `workspace.package.rust-version`
- **Nightly version**: From `tools/ci.rs` → `NIGHTLY_VERSION`
- **Image tag**: Format `v{stable}-{nightly}` (e.g., `v1.92.0-2025-12-20`)

## Local Commands

- `./tools/ci.rs check-env`: Verify your local environment (toolchain, docker, etc.).
- `./tools/ci.rs docker dev`: Enter the Docker development environment.
- `./tools/ci.rs docker pull`: Pull the latest images matching your current source code version.

## Architecture

1. **`tools/ci.rs`**: Centralized logic for all development and CI tasks. It handles both local execution and Docker orchestration.
2. **GitHub Actions**: Uses the same images and `ci.rs` script for consistent results.

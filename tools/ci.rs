#!/usr/bin/env -S cargo +nightly -Zscript
---
[dependencies]
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
---
//! Local CI check script - mirrors .github/workflows/ci.yml
//!
//! Usage:
//!   cargo +nightly ci [command]
//!   ./tools/ci.rs [command]
//!
//! Commands:
//!   all       - Run all checks (default)
//!   fmt       - Check code formatting (nightly)
//!   clippy    - Run clippy lints (nightly)
//!   check     - Cargo check (stable)
//!   lint      - Run all lint checks (fmt + clippy + check)
//!   test      - Run tests (uses nextest if available)
//!   doctest   - Run doc tests (stable)
//!   doc       - Compile doc examples (stable)
//!   wasm      - Compile to wasm32 (stable)
//!   bundle    - Bundle counter example (stable)
//!
//! Aliases:
//!   f         - fmt
//!   c         - clippy
//!   t         - test
//!   d         - doctest
//!   w         - wasm
//!   b         - bundle
//!   l         - lint

use std::{
  env, fs,
  process::{Command, ExitCode, Stdio},
};

use serde::Deserialize;

/// Toolchain versions matching ci.yml
const NIGHTLY_VERSION: &str = "nightly-2025-12-20";

fn main() -> ExitCode {
  let args: Vec<String> = env::args().collect();
  let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");

  // Commands that require stable version
  let needs_stable = matches!(
    command,
    "all"
      | "check"
      | "lint"
      | "l"
      | "test"
      | "t"
      | "doctest"
      | "d"
      | "doc"
      | "wasm"
      | "w"
      | "bundle"
      | "b"
  );

  // Get the stable version only if needed
  let stable_version = if needs_stable {
    match get_stable_version() {
      Ok(v) => v,
      Err(e) => {
        eprintln!("❌ Failed to determine stable version: {}", e);
        return ExitCode::FAILURE;
      }
    }
  } else {
    "stable".to_string() // Won't be used
  };

  let result = match command {
    "all" => run_all(&stable_version),
    "fmt" | "f" => run_fmt(),
    "clippy" | "c" => run_clippy(),
    "check" => run_check(&stable_version),
    "lint" | "l" => run_lint(&stable_version),
    "test" | "t" => run_test(&stable_version),
    "doctest" | "d" => run_doctest(&stable_version),
    "doc" => run_doc_examples(&stable_version),
    "wasm" | "w" => run_wasm(&stable_version),
    "bundle" | "b" => run_bundle(&stable_version),
    "config" => {
      print_config();
      Ok(())
    }
    "config-nightly" => {
      // Output just the nightly version for CI to parse
      println!("{}", NIGHTLY_VERSION);
      Ok(())
    }
    "help" | "-h" | "--help" => {
      print_help();
      Ok(())
    }
    _ => {
      eprintln!("Unknown command: {}", command);
      print_help();
      Err(())
    }
  };

  match result {
    Ok(()) => ExitCode::SUCCESS,
    Err(()) => ExitCode::FAILURE,
  }
}

#[derive(Deserialize)]
struct CargoToml {
  workspace: Option<Workspace>,
}

#[derive(Deserialize)]
struct Workspace {
  package: Option<WorkspacePackage>,
}

#[derive(Deserialize)]
struct WorkspacePackage {
  #[serde(rename = "rust-version")]
  rust_version: Option<String>,
}

/// Read MSRV from Cargo.toml and return a stable version that meets the
/// requirement
fn get_stable_version() -> Result<String, String> {
  let cargo_toml =
    fs::read_to_string("Cargo.toml").map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

  let parsed: CargoToml =
    toml::from_str(&cargo_toml).map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

  let msrv = parsed
    .workspace
    .and_then(|w| w.package)
    .and_then(|p| p.rust_version)
    .ok_or_else(|| "No rust-version found in workspace.package".to_string())?;

  // Get current stable version
  let output = Command::new("rustc")
    .args(["+stable", "--version"])
    .output()
    .map_err(|e| format!("Failed to run rustc: {}", e))?;

  let version_str = String::from_utf8_lossy(&output.stdout);
  // Parse version like "rustc 1.88.0 (some hash)"
  let current_stable = version_str
    .split_whitespace()
    .nth(1)
    .ok_or_else(|| "Failed to parse rustc version".to_string())?;

  // Compare versions
  let msrv_parts: Vec<u32> = msrv
    .split('.')
    .filter_map(|s| s.parse().ok())
    .collect();
  let stable_parts: Vec<u32> = current_stable
    .split('.')
    .filter_map(|s| s.parse().ok())
    .collect();

  let msrv_ok = stable_parts >= msrv_parts;

  if !msrv_ok {
    return Err(format!(
      "Current stable ({}) is less than MSRV ({}). Please update rustup.",
      current_stable, msrv
    ));
  }

  println!("📦 Using stable toolchain: {} (MSRV: {})", current_stable, msrv);
  Ok("stable".to_string())
}

fn print_help() {
  println!(
    r#"
Local CI Check Script - mirrors .github/workflows/ci.yml

Usage:
  cargo +nightly ci [command]
  ./tools/ci.rs [command]

Commands:
  all            - Run all checks (default)
  fmt (f)        - Check code formatting      [{}]
  clippy (c)     - Run clippy lints           [{}]
  check          - Cargo check                [stable]
  lint (l)       - Run all lint checks (fmt + clippy + check)
  test (t)       - Run tests (nextest if available)
  doctest (d)    - Run doc tests              [stable]
  doc            - Compile doc examples       [stable]
  wasm (w)       - Compile to wasm32          [stable]
  bundle (b)     - Bundle counter example     [stable]
  config         - Show all configuration
  config-nightly - Output nightly version (for CI)
  help           - Show this help message

Examples:
  cargo +nightly ci       # Run all checks
  cargo +nightly ci f     # Check formatting only
  cargo +nightly ci l     # Run all lints (fmt + clippy + check)
  cargo +nightly ci t     # Run tests
"#,
    NIGHTLY_VERSION, NIGHTLY_VERSION
  );
}

fn print_config() {
  println!("CI Configuration:");
  println!("  NIGHTLY_VERSION: {}", NIGHTLY_VERSION);
}

fn run_all(stable_version: &str) -> Result<(), ()> {
  println!("🚀 Running all CI checks...\n");

  run_lint(stable_version)?;
  run_test(stable_version)?;
  run_doctest(stable_version)?;
  run_doc_examples(stable_version)?;
  run_wasm(stable_version)?;
  run_bundle(stable_version)?;

  println!("\n✅ All CI checks passed!");
  Ok(())
}

fn run_lint(stable_version: &str) -> Result<(), ()> {
  println!("🔍 Running lint checks...\n");
  run_fmt()?;
  run_clippy()?;
  run_check(stable_version)?;
  println!("✅ All lint checks passed!\n");
  Ok(())
}

/// cargo fmt --all -- --check (nightly)
fn run_fmt() -> Result<(), ()> {
  println!("📝 Checking code formatting [{}]...", NIGHTLY_VERSION);
  run_cargo_command(NIGHTLY_VERSION, &["fmt", "--all", "--", "--check"], None, &[])
}

/// cargo clippy --all-targets --all-features -- -D warnings (nightly)
fn run_clippy() -> Result<(), ()> {
  println!("📎 Running clippy [{}]...", NIGHTLY_VERSION);
  run_cargo_command(
    NIGHTLY_VERSION,
    &["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
    None,
    &[],
  )
}

/// cargo check (stable)
fn run_check(stable_version: &str) -> Result<(), ()> {
  println!("✔️  Running cargo check [{}]...", stable_version);
  run_cargo_command(stable_version, &["check"], None, &[])
}

/// Run tests using nextest if available, otherwise fallback to cargo test
fn run_test(stable_version: &str) -> Result<(), ()> {
  if has_cargo_tool("nextest") {
    println!("🧪 Running tests with nextest [{}]...", stable_version);
    run_cargo_command(
      stable_version,
      &["nextest", "run", "--workspace", "--all-features"],
      None,
      &[],
    )
  } else {
    println!("🧪 Running tests [{}]...", stable_version);
    run_cargo_command(
      stable_version,
      &["test", "--workspace", "--all-targets", "--all-features"],
      None,
      &[],
    )
  }
}

/// cargo test --doc --workspace --all-features
fn run_doctest(toolchain: &str) -> Result<(), ()> {
  println!("📚 Running doc tests [{}]...", toolchain);
  run_cargo_command(toolchain, &["test", "--doc", "--workspace", "--all-features"], None, &[])
}

/// Build ribir and compile doc examples with rustdoc (stable)
fn run_doc_examples(stable_version: &str) -> Result<(), ()> {
  println!("📖 Compiling doc examples [{}]...", stable_version);

  // First build the workspace
  run_cargo_command(stable_version, &["build", "--workspace", "--exclude", "pomodoro"], None, &[])?;

  let target_dir = get_toolchain_target_dir(stable_version);
  let deps_dir = target_dir.join("debug/deps");

  // Find the ribir library file, it could be .rlib or .lib depending on platform
  let ribir_lib = if target_dir.join("debug/libribir.rlib").exists() {
    target_dir.join("debug/libribir.rlib")
  } else if target_dir.join("debug/ribir.lib").exists() {
    target_dir.join("debug/ribir.lib")
  } else {
    // Fallback to .rlib if not sure, or try to find it in deps
    target_dir.join("debug/libribir.rlib")
  };

  // Find all markdown files
  let mut md_files = vec!["./README.md".to_string()];

  // Recursively find docs in subdirectories
  fn find_md_files(dir: &std::path::Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
          find_md_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "md") {
          files.push(path.to_string_lossy().to_string());
        }
      }
    }
  }
  find_md_files(std::path::Path::new("./docs"), &mut md_files);

  // Remove duplicates
  md_files.sort();
  md_files.dedup();

  // Compile each markdown file
  for md_file in &md_files {
    println!("   Compiling: {}", md_file);
    let status = Command::new("rustup")
      .args([
        "run",
        stable_version,
        "rustdoc",
        "--test",
        md_file,
        "-L",
        &deps_dir.to_string_lossy(),
        "--edition",
        "2024",
        "--extern",
        &format!("ribir={}", ribir_lib.to_string_lossy()),
      ])
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .status();

    match status {
      Ok(s) if s.success() => {}
      Ok(_) => {
        eprintln!("❌ Failed to compile doc examples in: {}", md_file);
        return Err(());
      }
      Err(e) => {
        eprintln!("❌ Failed to run rustdoc: {}", e);
        return Err(());
      }
    }
  }

  println!("✅ Doc examples compiled successfully!\n");
  Ok(())
}

/// Compile to wasm32-unknown-unknown (stable)
fn run_wasm(stable_version: &str) -> Result<(), ()> {
  println!("🌐 Compiling to wasm32 [{}]...", stable_version);

  run_cargo_command(
    stable_version,
    &[
      "build",
      "--workspace",
      "--target",
      "wasm32-unknown-unknown",
      "--exclude",
      "ribir_dev_helper",
      "--exclude",
      "cli",
      "--exclude",
      "pomodoro",
      "--exclude",
      "ribir-bot",
    ],
    None,
    &[("RUSTFLAGS", r#"--cfg getrandom_backend="wasm_js""#)],
  )
}

fn run_bundle(stable_version: &str) -> Result<(), ()> {
  println!("📦 Bundling gallery example [{}]...", stable_version);

  run_cargo_command(
    stable_version,
    &["run", "-p", "cli", "--", "bundle", "--profile", "dev"],
    Some("examples/gallery"),
    &[("RIBIR_FEATURES", "wgpu")],
  )?;

  println!("🧪 Verifying bundled assets...");
  let mut bundle_assets_path = get_toolchain_target_dir(stable_version);
  bundle_assets_path.push("debug/bundle");

  let asset_manifest = if cfg!(target_os = "macos") {
    let mut p = bundle_assets_path.clone();
    p.push("macos/Gallery.app/Contents/Resources/assets");
    if p.join(".asset_manifest.txt").exists() {
      Some(p.join(".asset_manifest.txt"))
    } else {
      find_assets_manifest(&p)
    }
  } else {
    find_assets_manifest(&bundle_assets_path)
  };

  match asset_manifest {
    Some(path) => {
      println!("✅ Found asset manifest at: {}", path.display());
      let content = std::fs::read_to_string(&path).map_err(|_| ())?;
      if content.trim().is_empty() {
        eprintln!("❌ Asset manifest is empty");
        return Err(());
      }
      println!("✅ Asset manifest content:\n{}", content);
      Ok(())
    }
    None => {
      eprintln!("❌ Could not find asset manifest in bundle");
      Err(())
    }
  }
}

fn find_assets_manifest(path: &std::path::Path) -> Option<std::path::PathBuf> {
  if !path.exists() {
    return None;
  }
  if path.join(".asset_manifest.txt").exists() {
    return Some(path.join(".asset_manifest.txt"));
  }
  if let Ok(entries) = std::fs::read_dir(path) {
    for entry in entries.flatten() {
      if entry.path().is_dir() {
        if let Some(found) = find_assets_manifest(&entry.path()) {
          return Some(found);
        }
      }
    }
  }
  None
}

fn run_cargo_command(
  toolchain: &str, args: &[&str], cwd: Option<&str>, envs: &[(&str, &str)],
) -> Result<(), ()> {
  let target_dir = get_toolchain_target_dir(toolchain);
  let mut cmd = Command::new("cargo");
  cmd.arg(format!("+{}", toolchain));
  cmd.args(args);
  cmd.env("CARGO_TARGET_DIR", target_dir);

  for (key, value) in envs {
    cmd.env(key, value);
  }
  if let Some(cwd) = cwd {
    cmd.current_dir(cwd);
  }

  let status = cmd
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status();

  match status {
    Ok(s) if s.success() => {
      println!("✅ Success!\n");
      Ok(())
    }
    Ok(_) => {
      eprintln!("❌ Command failed: cargo +{} {}", toolchain, args.join(" "));
      Err(())
    }
    Err(e) => {
      eprintln!("❌ Failed to run cargo: {}", e);
      Err(())
    }
  }
}

fn get_toolchain_target_dir(toolchain: &str) -> std::path::PathBuf {
  let mut path = std::env::current_dir().expect("Failed to get current directory");
  path.push("target");

  if !is_default_toolchain(toolchain) {
    path.push(toolchain);
  }
  path
}

fn is_default_toolchain(toolchain: &str) -> bool {
  let default_v = Command::new("rustc")
    .env_remove("RUSTUP_TOOLCHAIN")
    .arg("--version")
    .output()
    .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

  let toolchain_v = Command::new("rustc")
    .arg(format!("+{}", toolchain))
    .arg("--version")
    .output()
    .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

  match (default_v, toolchain_v) {
    (Ok(dv), Ok(tv)) => dv == tv,
    _ => false,
  }
}

fn has_cargo_tool(tool: &str) -> bool {
  Command::new("cargo")
    .args([tool, "--version"])
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .is_ok_and(|s| s.success())
}

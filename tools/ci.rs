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
//!   test      - Run tests (stable)
//!   doctest   - Run doc tests (stable)
//!   doc       - Compile doc examples (stable)
//!   wasm      - Compile to wasm32 (stable)
//!   bundle    - Bundle counter example (stable)
//!   check-env - Verify local environment matches project requirements
//!   docker    - Docker management (dev, pull, run in container)
//!   clean     - Clean build artifacts (use --all to also clean Docker)
//!   status    - Show status of GitHub Actions workflow
//!   watch     - Watch GitHub Actions workflow
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
      | "check-env"
      | "docker"
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
    "check-env" => {
      println!("🌙 Nightly version: {}", NIGHTLY_VERSION);
      if has_toolchain(NIGHTLY_VERSION) {
          println!("✅ Nightly toolchain is available.");
      } else {
          eprintln!("❌ Nightly toolchain ({}) not found. Please run: rustup install {}", NIGHTLY_VERSION, NIGHTLY_VERSION);
      }
      check_tool("docker", "Docker is required for containerized development (Linux/Windows).");
      check_tool("gh", "GitHub CLI is required for checking CI status.");
      Ok(())
    },
    "docker" => {
      let sub = args.get(2).map(|s| s.as_str()).unwrap_or("help");
      match sub {
        "dev" => run_in_docker(&[] as &[&str], true),
        "pull" => run_pull(),
        "help" => {
            print_docker_help();
            Ok(())
        },
        _ => {
            let docker_args: Vec<&str> = args.iter().skip(2).map(|s| s.as_str()).collect();
            run_in_docker(&docker_args, false)
        }
      }
    }
    "clean" => {
      let all = args.get(2).map(|s| s == "--all").unwrap_or(false);
      run_clean(all)
    }
    "status" => run_gh(&["run", "list", "--workflow=ci.yml", "--limit", "5"]),
    "watch" => run_gh(&["run", "watch", "--workflow=ci.yml"]),
    "config" => {
      print_config();
      Ok(())
    }
    "config-nightly" => {
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

fn get_msrv() -> Result<String, String> {
  let cargo_toml =
    fs::read_to_string("Cargo.toml").map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

  let parsed: CargoToml =
    toml::from_str(&cargo_toml).map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

  parsed
    .workspace
    .and_then(|w| w.package)
    .and_then(|p| p.rust_version)
    .ok_or_else(|| "No rust-version found in workspace.package".to_string())
}

fn get_stable_version() -> Result<String, String> {
  let msrv = get_msrv()?;
  let output = Command::new("rustc")
    .args(["+stable", "--version"])
    .output()
    .map_err(|e| format!("Failed to run rustc: {}", e))?;

  let version_str = String::from_utf8_lossy(&output.stdout);
  let current_stable = version_str
    .split_whitespace()
    .nth(1)
    .ok_or_else(|| "Failed to parse rustc version".to_string())?;

  let msrv_parts: Vec<u32> = msrv.split('.').filter_map(|s| s.parse().ok()).collect();
  let stable_parts: Vec<u32> = current_stable.split('.').filter_map(|s| s.parse().ok()).collect();

  if stable_parts < msrv_parts {
    return Err(format!(
      "Current stable ({}) is less than MSRV ({}). Please update rustup.",
      current_stable, msrv
    ));
  }

  println!("📦 Using stable toolchain: {} (MSRV: {})", current_stable, msrv);
  Ok("stable".to_string())
}

fn has_toolchain(toolchain: &str) -> bool {
    Command::new("rustup")
        .args(["run", toolchain, "rustc", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_tool(name: &str, help: &str) {
  if Command::new(name).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok() {
    println!("✅ {} is available.", name);
  } else {
    println!("⚠️  {} not found. {}", name, help);
  }
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
  test (t)       - Run tests with coverage    [{}]
  doctest (d)    - Run doc tests              [{}]
  doc            - Compile doc examples       [stable]
  wasm (w)       - Compile to wasm32          [stable]
  bundle (b)     - Bundle counter example     [stable]
  check-env      - Verify environment
  docker <cmd>   - Docker management (dev, pull, <ci-cmd>)
  clean [--all]  - Clean artifacts (use --all to clean Docker too)
  status         - Show GH Actions status
  watch          - Watch GH Actions status
  config         - Show all configuration
  help           - Show this help message

Examples:
  cargo +nightly ci       # Run all checks
  cargo +nightly ci docker dev   # Start docker dev environment
  cargo +nightly ci docker test  # Run tests inside docker
"#,
    NIGHTLY_VERSION, NIGHTLY_VERSION, NIGHTLY_VERSION, NIGHTLY_VERSION
  );
}

fn print_docker_help() {
    println!(r#"
Docker Management

Usage:
  ci docker <command>

Commands:
  dev     - Start interactive bash session in container
  pull    - Pull latest images from GHCR
  <any>   - Run a CI command inside the container (e.g., ci docker test)

Examples:
  ci docker dev
  ci docker pull
  ci docker lint
"#);
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

fn run_fmt() -> Result<(), ()> {
  println!("📝 Checking code formatting [{}]...", NIGHTLY_VERSION);
  run_cargo_command(NIGHTLY_VERSION, &["fmt", "--all", "--", "--check"], None, &[])
}

fn run_clippy() -> Result<(), ()> {
  println!("📎 Running clippy [{}]...", NIGHTLY_VERSION);
  run_cargo_command(
    NIGHTLY_VERSION,
    &["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"],
    None,
    &[],
  )
}

fn run_check(stable_version: &str) -> Result<(), ()> {
  println!("✔️  Running cargo check [{}]...", stable_version);
  run_cargo_command(stable_version, &["check"], None, &[])
}

fn run_test(stable_version: &str) -> Result<(), ()> {
  if has_cargo_tool("llvm-cov") {
    println!("🧪 Running tests with coverage [{}]...", stable_version);
    run_cargo_command(stable_version, &["llvm-cov", "--workspace", "--all-features"], None, &[])
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

fn run_doctest(toolchain: &str) -> Result<(), ()> {
  println!("📚 Running doc tests [{}]...", toolchain);
  run_cargo_command(toolchain, &["test", "--doc", "--workspace", "--all-features"], None, &[])
}

fn run_doc_examples(stable_version: &str) -> Result<(), ()> {
  println!("📖 Compiling doc examples [{}]...", stable_version);
  run_cargo_command(stable_version, &["build", "--workspace", "--exclude", "pomodoro"], None, &[])?;
  let target_dir = get_toolchain_target_dir(stable_version);
  let deps_dir = target_dir.join("debug/deps");
  let ribir_lib = if target_dir.join("debug/libribir.rlib").exists() {
    target_dir.join("debug/libribir.rlib")
  } else if target_dir.join("debug/ribir.lib").exists() {
    target_dir.join("debug/ribir.lib")
  } else {
    target_dir.join("debug/libribir.rlib")
  };
  let mut md_files = vec!["./README.md".to_string()];
  fn find_md_files(dir: &std::path::Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() { find_md_files(&path, files); } 
        else if path.extension().is_some_and(|e| e == "md") { files.push(path.to_string_lossy().to_string()); }
      }
    }
  }
  find_md_files(std::path::Path::new("./docs"), &mut md_files);
  md_files.sort(); md_files.dedup();
  for md_file in &md_files {
    println!("   Compiling: {}", md_file);
    let status = Command::new("rustup")
      .args(["run", stable_version, "rustdoc", "--test", md_file, "-L", &deps_dir.to_string_lossy(), "--edition", "2024", "--extern", &format!("ribir={}", ribir_lib.to_string_lossy())])
      .stdout(Stdio::inherit()).stderr(Stdio::inherit()).status();
    match status {
      Ok(s) if s.success() => {}
      _ => { eprintln!("❌ Failed to compile doc examples in: {}", md_file); return Err(()); }
    }
  }
  println!("✅ Doc examples compiled successfully!\n");
  Ok(())
}

fn run_wasm(stable_version: &str) -> Result<(), ()> {
  println!("🌐 Compiling to wasm32 [{}]...", stable_version);
  run_cargo_command(stable_version, &["build", "--workspace", "--target", "wasm32-unknown-unknown", "--exclude", "ribir_dev_helper", "--exclude", "cli", "--exclude", "pomodoro"], None, &[("RUSTFLAGS", r#"--cfg getrandom_backend="wasm_js""#)])
}

fn run_bundle(stable_version: &str) -> Result<(), ()> {
  println!("📦 Bundling counter example [{}]...", stable_version);
  let cfg = if cfg!(target_os = "macos") { "ci/bundle-macos.toml" } 
            else if cfg!(target_os = "linux") { "ci/bundle-linux.toml" } 
            else if cfg!(target_os = "windows") { "ci/bundle-windows.toml" } 
            else { eprintln!("❌ Unknown OS for bundling!"); return Err(()); };
  run_cargo_command(stable_version, &["build", "-p", "counter", "--release"], None, &[])?;
  run_cargo_command(stable_version, &["run", "-p", "cli", "--", "bundle", "-c", cfg], Some("examples/counter"), &[])
}

fn run_cargo_command(toolchain: &str, args: &[&str], cwd: Option<&str>, envs: &[(&str, &str)]) -> Result<(), ()> {
  let target_dir = get_toolchain_target_dir(toolchain);
  let mut cmd = Command::new("cargo");
  cmd.arg(format!("+{}", toolchain));
  cmd.args(args);
  cmd.env("CARGO_TARGET_DIR", target_dir);
  for (key, value) in envs { cmd.env(key, value); }
  if let Some(cwd) = cwd { cmd.current_dir(cwd); }
  let status = cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit()).status();
  match status {
    Ok(s) if s.success() => { println!("✅ Success!\n"); Ok(()) }
    _ => { eprintln!("❌ Command failed: cargo +{} {}", toolchain, args.join(" ")); Err(()) }
  }
}

fn get_toolchain_target_dir(toolchain: &str) -> std::path::PathBuf {
  let mut path = std::env::current_dir().expect("Failed to get current directory");
  path.push("target");
  if !is_default_toolchain(toolchain) { path.push(toolchain); }
  path
}

fn is_default_toolchain(toolchain: &str) -> bool {
  let default_v = Command::new("rustc").env_remove("RUSTUP_TOOLCHAIN").arg("--version").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string());
  let toolchain_v = Command::new("rustc").arg(format!("+{}", toolchain)).arg("--version").output().map(|o| String::from_utf8_lossy(&o.stdout).to_string());
  match (default_v, toolchain_v) { (Ok(dv), Ok(tv)) => dv == tv, _ => false }
}

fn has_cargo_tool(tool: &str) -> bool {
  Command::new("cargo").args([tool, "--version"]).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn has_tool(name: &str) -> bool {
  Command::new(name).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()
}

fn get_image_info() -> Result<(String, String), ()> {
    let msrv = get_msrv().map_err(|e| eprintln!("❌ {}", e))?;
    let tag = format!("v{}-{}", msrv, NIGHTLY_VERSION);
    let org = env::var("GITHUB_REPOSITORY_OWNER").unwrap_or_else(|_| "ribir-org".to_string());
    let image = if cfg!(target_os = "macos") { eprintln!("❌ macOS does not support native Docker containers for GUI/GPU development."); return Err(()); } 
                else if cfg!(target_os = "windows") { format!("ghcr.io/{}/ribir-windows-base:{}", org, tag) } 
                else { format!("ghcr.io/{}/ribir-linux:{}", org, tag) };
    Ok((image, tag))
}

fn run_in_docker(args: &[&str], interactive: bool) -> Result<(), ()> {
    if !has_tool("docker") {
        eprintln!("❌ docker not found. Docker is required for containerized development (Linux/Windows).");
        return Err(());
    }
    let (image, _) = get_image_info()?;
    let project_root = env::current_dir().map_err(|e| eprintln!("❌ Failed to get current directory: {}", e))?;
    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm"]);
    if interactive { cmd.arg("-it"); }
    cmd.args(["-v", &format!("{}:/app", project_root.display()), "-v", &format!("{}/target:/app/target", project_root.display()), "-v", &format!("{}/.cargo/registry:/usr/local/cargo/registry", project_root.display()), "-v", &format!("{}/.cargo/git:/usr/local/cargo/git", project_root.display()), "-e", "CARGO_TARGET_DIR=/app/target", "-e", "CARGO_INCREMENTAL=1", "-w", "/app"]);
    if cfg!(target_os = "linux") { cmd.args(["--device", "/dev/dri", "--group-add", "video"]); }
    cmd.arg(image);
    if interactive && args.is_empty() { cmd.arg("bash"); } 
    else { cmd.args(["./tools/ci.rs"]); cmd.args(args); }
    println!("🐳 Running in Docker...");
    let status = cmd.status().map_err(|e| eprintln!("❌ Failed to run docker: {}", e))?;
    if status.success() { Ok(()) } else { Err(()) }
}

fn run_pull() -> Result<(), ()> {
    if !has_tool("docker") {
        eprintln!("❌ docker not found.");
        return Err(());
    }
    let msrv = get_msrv().map_err(|e| eprintln!("❌ {}", e))?;
    let tag = format!("v{}-{}", msrv, NIGHTLY_VERSION);
    let org = env::var("GITHUB_REPOSITORY_OWNER").unwrap_or_else(|_| "ribir-org".to_string());
    let images = vec![format!("ghcr.io/{}/ribir-linux:{}", org, tag), format!("ghcr.io/{}/ribir-windows-base:{}", org, tag)];
    for image in images { println!("📥 Pulling image: {}...", image); let _ = Command::new("docker").args(["pull", &image]).status(); }
    Ok(())
}

fn run_clean(all: bool) -> Result<(), ()> {
    println!("🧹 Cleaning build artifacts...");
    let _ = Command::new("cargo").arg("clean").status();
    if all {
        if !has_tool("docker") {
            eprintln!("⚠️  docker not found, skipping Docker cleanup.");
        } else {
            println!("🧹 Cleaning Docker resources...");
            let _ = Command::new("docker").args(["system", "prune", "-f"]).status();
        }
    }
    println!("✅ Cleanup complete");
    Ok(())
}

fn run_gh(args: &[&str]) -> Result<(), ()> {
    if !has_tool("gh") {
        eprintln!("❌ GitHub CLI (gh) not found. Required for checking CI status.");
        return Err(());
    }
    let status = Command::new("gh").args(args).status().map_err(|e| eprintln!("❌ Failed to run gh: {}", e))?;
    if status.success() { Ok(()) } else { Err(()) }
}

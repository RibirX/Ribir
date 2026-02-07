//! MCP (Model Context Protocol) implementation
//!
//! Provides a native Rust stdio server for AI clients to debug Ribir
//! applications. Supports automatic port discovery based on the current
//! working directory.

mod port_discovery;
mod schema;
mod serve;

use std::path::Path;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use port_discovery::PortRegistry;
use serve::{PortSource, mcp_serve};

use crate::CliCommand;

pub struct McpCommand;

impl CliCommand for McpCommand {
  fn name(&self) -> &str { "mcp" }

  fn command(&self) -> Command {
    Command::new("mcp")
      .about("MCP (Model Context Protocol) for debugging Ribir apps with AI assistants")
      .long_about(
        "MCP enables AI clients (Claude, Gemini, etc.) to inspect and debug running Ribir \
         applications.\n\nThe 'serve' subcommand starts a stdio server that AI clients connect \
         to. The port is automatically discovered based on the current working directory, \
         allowing multiple projects to be debugged simultaneously.",
      )
      .subcommand(
        Command::new("serve")
          .about("Start MCP stdio server (used by AI clients)")
          .long_about(
            "Starts the MCP server that communicates via stdio (standard input/output). This \
             command is typically invoked by AI clients, not directly by users.\n\nThe server \
             automatically discovers the debug server port based on the current working \
             directory. Use --port to override.",
          )
          .arg(
            Arg::new("port")
              .long("port")
              .short('p')
              .help("Ribir debug server port (auto-discovered if not specified)")
              .value_parser(clap::value_parser!(u16)),
          ),
      )
      .subcommand(
        Command::new("check")
          .about("Check connection to the Ribir debug server")
          .arg(
            Arg::new("port")
              .long("port")
              .short('p')
              .help("Ribir debug server port (auto-discovered if not specified)")
              .value_parser(clap::value_parser!(u16)),
          ),
      )
      .subcommand(Command::new("list").about("List all active debug sessions"))
  }

  fn exec(&self, args: &ArgMatches) -> Result<()> {
    match args.subcommand() {
      Some(("serve", sub_args)) => exec_serve(sub_args),
      Some(("check", sub_args)) => exec_check(sub_args),
      Some(("list", _)) => exec_list(),
      _ => {
        println!("MCP Commands:");
        println!("  serve  - Start MCP stdio server for AI clients");
        println!("  check  - Test connection to debug server");
        println!("  list   - List all active debug sessions");
        println!("\nFor configuration examples, see: tools/cli/README.md");
        Ok(())
      }
    }
  }
}

#[derive(Debug, Clone)]
struct ResolvedPort {
  port: u16,
  source: PortSource,
}

/// Resolve the port to use: explicit > discovered.
///
/// Unlike previous behavior, this does not fall back to a default port to avoid
/// accidentally connecting to an unrelated debug session.
fn resolve_port(args: &ArgMatches) -> Result<ResolvedPort> {
  let explicit_port = args.get_one::<u16>("port").copied();
  let cwd = std::env::current_dir()?;
  let registry = PortRegistry::new();
  resolve_port_from_inputs(explicit_port, &cwd, &registry)
}

/// Resolve port for MCP stdio server startup.
///
/// `mcp serve` must always be able to start so MCP clients can complete
/// initialize/tools-list/resources-list handshake even when no debug app is
/// running yet.
fn resolve_port_for_serve(args: &ArgMatches) -> Result<ResolvedPort> {
  let explicit_port = args.get_one::<u16>("port").copied();
  let cwd = std::env::current_dir()?;
  let registry = PortRegistry::new();
  Ok(resolve_port_for_serve_from_inputs(explicit_port, &cwd, &registry))
}

fn resolve_port_from_inputs(
  explicit_port: Option<u16>, cwd: &Path, registry: &PortRegistry,
) -> Result<ResolvedPort> {
  if let Some(port) = explicit_port {
    return Ok(ResolvedPort { port, source: PortSource::ExplicitArg });
  }

  if let Some(entry) = registry.discover_best_for_path(cwd) {
    let cwd_canonical = cwd
      .canonicalize()
      .unwrap_or_else(|_| cwd.to_path_buf());
    let entry_canonical = entry
      .project_path
      .canonicalize()
      .unwrap_or_else(|_| entry.project_path.clone());

    if entry_canonical != cwd_canonical {
      log::info!(
        "Discovered nearest debug server on port {} for {} (current dir: {})",
        entry.port,
        entry.project_path.display(),
        cwd.display()
      );
    } else {
      log::info!(
        "Discovered debug server on port {} for {}",
        entry.port,
        entry.project_path.display()
      );
    }

    return Ok(ResolvedPort { port: entry.port, source: PortSource::AutoDiscovered });
  }

  anyhow::bail!(
    "No Ribir debug session discovered for current directory: {}\nRecommended next step:\n- call \
     MCP tool 'start_app' with an explicit target (one of: package/bin/example)\nOther \
     options:\n- run `ribir-cli mcp list` to inspect active sessions\n- pass an explicit port \
     with `--port <PORT>`\n- or manually run your app in this worktree with `cargo run --features \
     debug --example <name>` or `cargo run --features debug -p <package>`",
    cwd.display()
  );
}

fn resolve_port_for_serve_from_inputs(
  explicit_port: Option<u16>, cwd: &Path, registry: &PortRegistry,
) -> ResolvedPort {
  match resolve_port_from_inputs(explicit_port, cwd, registry) {
    Ok(resolved) => resolved,
    Err(err) => {
      log::info!(
        "No debug session discovered for {}; starting MCP server in fallback mode. Reason: {}",
        cwd.display(),
        err
      );
      ResolvedPort { port: 0, source: PortSource::Unknown }
    }
  }
}

fn exec_serve(args: &ArgMatches) -> Result<()> {
  let resolved = resolve_port_for_serve(args)?;

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?
    .block_on(mcp_serve(resolved.port, resolved.source))
}

fn exec_check(args: &ArgMatches) -> Result<()> {
  let resolved = resolve_port(args)?;
  let port = resolved.port;

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?
    .block_on(async {
      let client = reqwest::Client::new();
      let url = format!("http://127.0.0.1:{}/status", port);

      println!(
        "Checking Ribir debug server at port {} (source: {})...",
        port,
        resolved.source.as_label()
      );

      match client.get(&url).send().await {
        Ok(resp) => {
          if resp.status().is_success() {
            println!("✓ Debug server is running");
            if let Ok(body) = resp.text().await {
              println!("\nServer status:");
              println!("{}", body);
            }
          } else {
            println!("✗ Debug server responded with status: {}", resp.status());
          }
          Ok(())
        }
        Err(e) => {
          println!("✗ Cannot connect to debug server");
          println!("  Error: {}", e);
          println!("\nRecommended next step (for MCP clients):");
          println!("  call MCP tool `start_app` with package/bin/example");
          println!("\nAlternative manual start:");
          println!("  cargo run --features debug --example <name>");
          println!("  cargo run --features debug -p <package>");
          Ok(())
        }
      }
    })
}

fn exec_list() -> Result<()> {
  let registry = PortRegistry::new();
  let entries = registry.list_all();

  if entries.is_empty() {
    println!("No active debug sessions found.");
    println!("\nRecommended next step (for MCP clients):");
    println!("  call MCP tool `start_app` with package/bin/example");
    println!("\nManual start examples:");
    println!("  cargo run --features debug --example <name>");
    println!("  cargo run --features debug -p <package>");
  } else {
    println!("Active debug sessions:\n");
    for entry in entries {
      println!("  Port: {}", entry.port);
      println!("  Path: {}", entry.project_path.display());
      println!("  PID:  {}", entry.pid);
      println!();
    }
  }

  Ok(())
}

pub fn mcp() -> Box<dyn CliCommand> { Box::new(McpCommand) }

#[cfg(test)]
mod tests {
  use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
  };

  use super::*;
  use crate::mcp::port_discovery::{PortEntry, path_to_hash};

  fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn resolve_port_prefers_explicit_arg() {
    let state_dir = temp_dir("ribir-cli-mcp-explicit");
    let registry = PortRegistry::with_state_dir(state_dir.clone());
    let cwd = temp_dir("ribir-cli-mcp-explicit-cwd");

    let resolved = resolve_port_from_inputs(Some(4242), &cwd, &registry).unwrap();
    assert_eq!(resolved.port, 4242);
    assert_eq!(resolved.source, PortSource::ExplicitArg);

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(cwd);
  }

  #[test]
  fn resolve_port_uses_discovered_entry() {
    let state_dir = temp_dir("ribir-cli-mcp-discovered");
    let registry = PortRegistry::with_state_dir(state_dir.clone());
    let cwd = temp_dir("ribir-cli-mcp-discovered-cwd");

    let hash = path_to_hash(&cwd);
    let file_path = state_dir.join(format!("{hash}.json"));
    let entry =
      PortEntry { port: 2444, project_path: cwd.clone(), pid: std::process::id(), started_at: 0 };
    fs::write(file_path, serde_json::to_vec(&entry).unwrap()).unwrap();

    let resolved = resolve_port_from_inputs(None, &cwd, &registry).unwrap();
    assert_eq!(resolved.port, 2444);
    assert_eq!(resolved.source, PortSource::AutoDiscovered);

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(cwd);
  }

  #[test]
  fn resolve_port_uses_nearest_prefix_match() {
    let state_dir = temp_dir("ribir-cli-mcp-prefix");
    let registry = PortRegistry::with_state_dir(state_dir.clone());
    let root = temp_dir("ribir-cli-mcp-prefix-root");
    let project = root.join("examples").join("counter");
    let cwd = root.clone();
    fs::create_dir_all(&project).unwrap();

    let hash = path_to_hash(&project);
    let file_path = state_dir.join(format!("{hash}.json"));
    let entry = PortEntry {
      port: 2555,
      project_path: project.clone(),
      pid: std::process::id(),
      started_at: 1,
    };
    fs::write(file_path, serde_json::to_vec(&entry).unwrap()).unwrap();

    let resolved = resolve_port_from_inputs(None, &cwd, &registry).unwrap();
    assert_eq!(resolved.port, 2555);
    assert_eq!(resolved.source, PortSource::AutoDiscovered);

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(root);
  }

  #[test]
  fn resolve_port_fails_when_not_found() {
    let state_dir = temp_dir("ribir-cli-mcp-not-found");
    let registry = PortRegistry::with_state_dir(state_dir.clone());
    let cwd = temp_dir("ribir-cli-mcp-not-found-cwd");

    let err = resolve_port_from_inputs(None, &cwd, &registry).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("No Ribir debug session discovered for current directory"));
    assert!(msg.contains("call MCP tool 'start_app'"));
    assert!(!msg.contains("2333"));

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(cwd);
  }

  #[test]
  fn resolve_port_for_serve_uses_fallback_mode_when_not_found() {
    let state_dir = temp_dir("ribir-cli-mcp-serve-fallback");
    let registry = PortRegistry::with_state_dir(state_dir.clone());
    let cwd = temp_dir("ribir-cli-mcp-serve-fallback-cwd");

    let serve_resolved = resolve_port_for_serve_from_inputs(None, &cwd, &registry);
    assert_eq!(serve_resolved.port, 0);
    assert_eq!(serve_resolved.source, PortSource::Unknown);

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(cwd);
  }
}

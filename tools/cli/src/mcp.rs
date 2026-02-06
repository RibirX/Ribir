//! MCP (Model Context Protocol) implementation
//!
//! Provides a native Rust stdio server for AI clients to debug Ribir
//! applications. Supports automatic port discovery based on the current
//! working directory.

mod port_discovery;
mod schema;
mod serve;

use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use port_discovery::PortRegistry;
use serve::mcp_serve;

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

/// Resolve the port to use: explicit > discovered > default
fn resolve_port(args: &ArgMatches) -> u16 {
  // 1. Explicit --port argument
  if let Some(&port) = args.get_one::<u16>("port") {
    return port;
  }

  // 2. Try to discover from current directory
  let registry = PortRegistry::new();
  if let Ok(cwd) = std::env::current_dir() {
    if let Some(entry) = registry.discover_for_path(&cwd) {
      log::info!(
        "Discovered debug server on port {} for {}",
        entry.port,
        entry.project_path.display()
      );
      return entry.port;
    }
  }

  // 3. Default fallback
  2333
}

fn exec_serve(args: &ArgMatches) -> Result<()> {
  let port = resolve_port(args);

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?
    .block_on(mcp_serve(port))
}

fn exec_check(args: &ArgMatches) -> Result<()> {
  let port = resolve_port(args);

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()?
    .block_on(async {
      let client = reqwest::Client::new();
      let url = format!("http://127.0.0.1:{}/status", port);

      println!("Checking Ribir debug server at port {}...", port);

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
          println!("\nMake sure your Ribir app is running with:");
          println!("  cargo run --features debug");
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
    println!("\nTo start a debug session, run your Ribir app with:");
    println!("  cargo run --features debug");
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

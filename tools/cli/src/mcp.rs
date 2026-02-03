//! MCP (Model Context Protocol) configuration CLI command
//!
//! Helps users configure MCP for AI clients like Claude Desktop, Claude CLI,
//! Cline, Cursor, OpenCode CLI, and Gemini CLI.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};

use crate::CliCommand;

/// Fixed installation directory for adapter
const RIBIR_DIR: &str = ".ribir";
const ADAPTER_FILENAME: &str = "mcp-adapter.js";
const SCHEMA_FILENAME: &str = "mcp_schema.json";

#[derive(Debug, Clone, Copy, PartialEq)]
enum AiClient {
  ClaudeCli,
  OpenCode,
  GeminiCli,
}

impl AiClient {
  fn from_str(s: &str) -> Option<Self> {
    match s.to_lowercase().as_str() {
      "claude" | "claude-cli" => Some(Self::ClaudeCli),
      "opencode" => Some(Self::OpenCode),
      "gemini" | "gemini-cli" => Some(Self::GeminiCli),
      _ => None,
    }
  }

  fn name(&self) -> &'static str {
    match self {
      Self::ClaudeCli => "Claude CLI",
      Self::OpenCode => "OpenCode CLI",
      Self::GeminiCli => "Gemini CLI",
    }
  }

  fn all() -> &'static [AiClient] { &[Self::ClaudeCli, Self::OpenCode, Self::GeminiCli] }

  /// Get the config file path for this AI client
  fn config_path(&self) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;

    let path = match self {
      Self::ClaudeCli => {
        // Claude CLI uses ~/.claude.json
        home.join(".claude.json")
      }
      Self::OpenCode => {
        // OpenCode CLI uses ~/.config/opencode/opencode.json
        home.join(".config/opencode/opencode.json")
      }
      Self::GeminiCli => {
        // Gemini CLI uses ~/.gemini/settings.json
        home.join(".gemini/settings.json")
      }
    };
    Ok(path)
  }

  /// Check if this client uses nested mcp config (mcpServers)
  fn uses_nested_mcp_config(&self) -> bool { !matches!(self, Self::OpenCode) }
}

/// Try to detect which AI clients are installed
fn detect_installed_clients() -> Vec<AiClient> {
  AiClient::all()
    .iter()
    .copied()
    .filter(|client| {
      client
        .config_path()
        .map(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .unwrap_or(false)
    })
    .collect()
}

/// Get the ribir directory path (~/.ribir)
fn ribir_dir() -> Result<PathBuf> {
  let home = dirs::home_dir().context("Could not find home directory")?;
  Ok(home.join(RIBIR_DIR))
}

/// Get the installed adapter path
fn installed_adapter_path() -> Result<PathBuf> { Ok(ribir_dir()?.join(ADAPTER_FILENAME)) }

/// Find the source adapter files from repo
fn find_source_files() -> Result<(PathBuf, PathBuf)> {
  // Try to find relative to the CLI binary
  let exe_path = std::env::current_exe().context("Could not get current executable path")?;

  for ancestor in exe_path.ancestors() {
    let adapter = ancestor.join("tools/mcp-adapter/mcp-adapter.js");
    let schema = ancestor.join("core/src/debug_tool/mcp_schema.json");
    if adapter.exists() && schema.exists() {
      return Ok((adapter, schema));
    }
  }

  // Fallback: check RIBIR_ROOT env
  if let Ok(root) = std::env::var("RIBIR_ROOT") {
    let root = PathBuf::from(root);
    let adapter = root.join("tools/mcp-adapter/mcp-adapter.js");
    let schema = root.join("core/src/debug_tool/mcp_schema.json");
    if adapter.exists() && schema.exists() {
      return Ok((adapter, schema));
    }
  }

  bail!(
    "Could not find source files. Please set RIBIR_ROOT environment variable to the Ribir \
     repository root."
  )
}

/// Copy adapter and schema to ~/.ribir/
fn install_adapter_files(dry_run: bool) -> Result<PathBuf> {
  let (src_adapter, src_schema) = find_source_files()?;
  let dest_dir = ribir_dir()?;
  let dest_adapter = dest_dir.join(ADAPTER_FILENAME);
  let dest_schema = dest_dir.join(SCHEMA_FILENAME);

  println!("Installing adapter files to: {}", dest_dir.display());

  if dry_run {
    println!("  [DRY RUN] Would copy {} -> {}", src_adapter.display(), dest_adapter.display());
    println!("  [DRY RUN] Would copy {} -> {}", src_schema.display(), dest_schema.display());
  } else {
    // Create destination directory
    std::fs::create_dir_all(&dest_dir).context("Failed to create ~/.ribir directory")?;

    // Copy files
    std::fs::copy(&src_adapter, &dest_adapter).context("Failed to copy mcp-adapter.js")?;
    std::fs::copy(&src_schema, &dest_schema).context("Failed to copy mcp_schema.json")?;

    println!("  ✓ Installed mcp-adapter.js");
    println!("  ✓ Installed mcp_schema.json");
  }

  Ok(dest_adapter)
}

/// Generate the MCP server configuration for Ribir
fn generate_ribir_config(client: AiClient, adapter_path: &std::path::Path, port: u16) -> Value {
  match client {
    AiClient::OpenCode => {
      // OpenCode uses local type with command array and environment
      json!({
        "type": "local",
        "command": [
          "node",
          adapter_path.to_string_lossy()
        ],
        "environment": {
          "RIBIR_DEBUG_PORT": port.to_string()
        },
        "enabled": true
      })
    }
    _ => {
      // Claude CLI and Gemini CLI use stdio format with command/args/env
      json!({
        "command": "node",
        "args": [adapter_path.to_string_lossy()],
        "env": {
          "RIBIR_DEBUG_PORT": port.to_string()
        }
      })
    }
  }
}

/// Read existing config or create empty object
fn read_or_create_config(path: &std::path::Path) -> Result<Value> {
  if path.exists() {
    let content = std::fs::read_to_string(path).context("Failed to read config file")?;
    if content.trim().is_empty() {
      return Ok(json!({}));
    }
    serde_json::from_str(&content).context("Failed to parse config file as JSON")
  } else {
    Ok(json!({}))
  }
}

/// Install MCP configuration for a specific client
fn install_for_client(
  client: AiClient, adapter_path: &std::path::Path, port: u16, dry_run: bool,
) -> Result<()> {
  let config_path = client.config_path()?;
  let ribir_config = generate_ribir_config(client, adapter_path, port);

  println!("\nConfiguring {} at: {}", client.name(), config_path.display());

  let mut config = read_or_create_config(&config_path)?;

  // OpenCode uses nested "mcp" config, others use "mcpServers"
  if client.uses_nested_mcp_config() {
    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
      config["mcpServers"] = json!({});
    }
    // Add or update ribir-debug entry
    config["mcpServers"]["ribir-debug"] = ribir_config;
  } else {
    // OpenCode uses nested "mcp" config
    if config.get("mcp").is_none() {
      config["mcp"] = json!({});
    }
    // Add or update ribir-debug entry
    config["mcp"]["ribir-debug"] = ribir_config;
  }

  let output = serde_json::to_string_pretty(&config)?;

  if dry_run {
    println!("  [DRY RUN] Would write:\n{}", output);
  } else {
    // Create parent directories if needed
    if let Some(parent) = config_path.parent() {
      std::fs::create_dir_all(parent).context("Failed to create config directory")?;
    }
    std::fs::write(&config_path, output).context("Failed to write config file")?;
    println!("  ✓ Configuration written");
  }

  Ok(())
}

pub struct McpCommand;

impl CliCommand for McpCommand {
  fn name(&self) -> &str { "mcp" }

  fn command(&self) -> Command {
    Command::new("mcp")
      .about("Configure MCP (Model Context Protocol) to debug Ribir apps with AI assistants")
      .long_about(
        "MCP enables AI clients (Claude, OpenCode, etc.) to inspect and debug running Ribir \
         applications.\n\nAfter installation, AI assistants can capture screenshots, inspect the \
         widget tree, add debug overlays, view logs, and more.\n\nMake sure your Ribir app is \
         running with '--features debug' before using MCP tools.",
      )
      .subcommand(
        Command::new("install")
          .about("Install adapter and configure MCP for AI clients")
          .long_about(
            "Installs the MCP adapter to ~/.ribir/ and configures AI clients to use it.\n\nYour \
             Ribir app must be running with '--features debug' for the tools to work.",
          )
          .arg(
            Arg::new("client")
              .long("client")
              .short('c')
              .help("Target AI client: claude-cli, opencode, gemini, or auto")
              .value_parser(["auto", "claude", "claude-cli", "opencode", "gemini", "gemini-cli"])
              .default_value("auto"),
          )
          .arg(
            Arg::new("port")
              .long("port")
              .short('p')
              .help("Ribir debug server port")
              .value_parser(clap::value_parser!(u16))
              .default_value("2333"),
          )
          .arg(
            Arg::new("dry-run")
              .long("dry-run")
              .help("Show what would be written without making changes")
              .action(ArgAction::SetTrue),
          )
          .arg(
            Arg::new("skip-adapter")
              .long("skip-adapter")
              .help("Skip copying adapter files to ~/.ribir/")
              .action(ArgAction::SetTrue),
          ),
      )
      .subcommand(Command::new("upgrade").about("Upgrade the adapter files in ~/.ribir/"))
      .subcommand(Command::new("status").about("Show current MCP configuration status"))
  }

  fn exec(&self, args: &ArgMatches) -> Result<()> {
    match args.subcommand() {
      Some(("install", sub_args)) => exec_install(sub_args),
      Some(("upgrade", _)) => exec_upgrade(),
      Some(("status", _)) => exec_status(),
      _ => {
        println!("Use 'mcp install' to install and configure MCP for your AI clients.");
        println!("Use 'mcp upgrade' to upgrade the adapter files.");
        println!("Use 'mcp status' to check current configuration.");
        Ok(())
      }
    }
  }
}

fn exec_install(args: &ArgMatches) -> Result<()> {
  let client_arg = args.get_one::<String>("client").unwrap();
  let port = *args.get_one::<u16>("port").unwrap();
  let dry_run = args.get_flag("dry-run");
  let skip_adapter = args.get_flag("skip-adapter");

  // Step 1: Install adapter files to ~/.ribir/ (unless skipped)
  let adapter_path = if skip_adapter {
    // Try to use existing installed adapter
    let path = installed_adapter_path()?;
    if !path.exists() {
      bail!("No adapter found at {}. Run without --skip-adapter first.", path.display());
    }
    println!("Using existing adapter: {}", path.display());
    path
  } else {
    install_adapter_files(dry_run)?
  };

  // Step 2: Configure AI clients
  if client_arg == "auto" {
    let detected = detect_installed_clients();
    if detected.is_empty() {
      println!("\nNo supported AI clients detected. Please specify --client explicitly.");
      println!("Supported clients: claude, claude-cli, cline, cursor, opencode, gemini");
      return Ok(());
    }

    println!(
      "\nDetected AI clients: {}",
      detected
        .iter()
        .map(|c| c.name())
        .collect::<Vec<_>>()
        .join(", ")
    );

    for client in detected {
      install_for_client(client, &adapter_path, port, dry_run)?;
    }
  } else {
    let client = AiClient::from_str(client_arg).context("Invalid client specified")?;
    install_for_client(client, &adapter_path, port, dry_run)?;
  }

  if !dry_run {
    println!("\n✓ MCP installation complete!");
    println!("  Adapter installed to: ~/.ribir/");
    println!("  Restart your AI client to apply changes.");
    println!("\n  To upgrade later, run: cli mcp upgrade");
  }

  Ok(())
}

fn exec_upgrade() -> Result<()> {
  println!("Upgrading adapter files...\n");
  install_adapter_files(false)?;
  println!("\n✓ Upgrade complete!");
  Ok(())
}

fn exec_status() -> Result<()> {
  println!("MCP Configuration Status\n");

  // Check adapter installation
  let adapter_path = installed_adapter_path()?;
  if adapter_path.exists() {
    println!("Adapter: ✓ Installed at {}", adapter_path.display());
  } else {
    println!("Adapter: ✗ Not installed (run 'mcp install' first)");
  }

  println!();

  // Check each client
  for client in AiClient::all() {
    let config_path = client.config_path()?;
    print!("{}: ", client.name());

    if !config_path.exists() {
      println!("Not configured");
      continue;
    }

    match read_or_create_config(&config_path) {
      Ok(config) => {
        // OpenCode uses "mcp", others use "mcpServers"
        let config_key = if client.uses_nested_mcp_config() { "mcpServers" } else { "mcp" };

        if let Some(servers) = config.get(config_key) {
          if servers.get("ribir-debug").is_some() {
            println!("✓ Configured");
          } else {
            println!("Config exists but ribir-debug not found");
          }
        } else {
          println!("Config exists but no {} section", config_key);
        }
      }
      Err(e) => println!("Error reading config: {}", e),
    }
  }

  Ok(())
}

pub fn mcp() -> Box<dyn CliCommand> { Box::new(McpCommand) }

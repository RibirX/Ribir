//! Ribir Release Bot - Unified CLI for PR, changelog, and release automation.

mod changelog;
mod changelog_merge;

mod commands;
mod external;
mod types;
mod utils;

use clap::Parser;
use types::{Cmd, Config, LogSubCmd, ReleaseCmd};

fn main() {
  if let Err(e) = run() {
    eprintln!("Error: {e}");
    std::process::exit(1);
  }
}

fn run() -> types::Result<()> {
  let mut config = Config::parse();

  // For release commands: default is dry-run unless --execute is passed
  match &config.command {
    Cmd::Release { cmd: ReleaseCmd::Next { execute, .. } }
    | Cmd::Release { cmd: ReleaseCmd::EnterRc { execute, .. } }
    | Cmd::Release { cmd: ReleaseCmd::Stable { execute, .. } } => {
      if !execute {
        config.dry_run = true;
      }
    }
    _ => {}
  }

  if config.dry_run {
    eprintln!("🔍 Dry-run mode enabled");
  }

  match &config.command {
    Cmd::Pr { cmd } => commands::cmd_pr(&config, cmd),
    Cmd::Log { cmd } => match cmd {
      LogSubCmd::Collect { version, write } => {
        commands::cmd_collect(&config, version, *write).map(|_| ())
      }
      LogSubCmd::Merge { version, write } => commands::cmd_merge(&config, version, *write),
      LogSubCmd::Verify => commands::cmd_verify(&config),
    },
    Cmd::Release { cmd } => commands::cmd_release(&config, cmd),
    Cmd::Workflow { cmd } => commands::cmd_workflow(&config, cmd),
  }
}

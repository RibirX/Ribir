//! Ribir Release Bot - Unified CLI for PR, changelog, and release automation.

mod changelog;
mod cli;
mod commands;
mod external;
mod types;
mod utils;

use types::{Cmd, Result};

fn main() {
  if let Err(e) = run() {
    eprintln!("Error: {e}");
    std::process::exit(1);
  }
}

fn run() -> Result<()> {
  let config = cli::parse_args()?;

  if config.dry_run {
    eprintln!("🔍 Dry-run mode enabled");
  }

  match &config.command {
    Cmd::Pr(pr_cmd) => commands::cmd_pr(&config, pr_cmd),
    Cmd::Collect { version, write } => commands::cmd_collect(&config, version, *write).map(|_| ()),
    Cmd::Merge { version, write } => commands::cmd_merge(&config, version, *write),
    Cmd::Verify => commands::cmd_verify(&config),
    Cmd::Release(rel_cmd) => commands::cmd_release(&config, rel_cmd),
  }
}

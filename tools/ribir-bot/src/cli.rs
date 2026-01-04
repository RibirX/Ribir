//! CLI argument parsing and help text.

use crate::types::{Cmd, Config, PrCmd, PrMode, ReleaseCmd, Result};

const HELP: &str = r#"Ribir Bot - Unified CLI for PR, changelog, and release automation.

USAGE:
    ribir-bot <COMMAND> [OPTIONS]

PR COMMANDS (update PR body with AI):
    pr-fill [PR_ID]              Auto-fill placeholders
    pr-regen [PR_ID] [CTX]       Regenerate all content
    pr-summary [PR_ID] [CTX]     Regenerate summary only
    pr-entry [PR_ID] [CTX]       Regenerate changelog entry only

CHANGELOG COMMANDS (update CHANGELOG.md):
    log-collect --version VER    Collect merged PRs into changelog
    log-merge --version VER      Merge pre-release versions
    log-verify                   Verify changelog structure

RELEASE COMMANDS:
    release next <level>             Full release (alpha|rc|patch|minor|major)
    release prepare --version VER    Prepare RC release
    release publish [PR_ID]          Publish GitHub release
    release promote --version VER    Promote RC to stable
    release highlights [CTX]         Regenerate highlights in CHANGELOG.md
    release social-card              Generate social card (coming soon)
    release verify                   Verify release state

GLOBAL OPTIONS:
    --dry-run    Preview without applying changes
    --execute    Execute changes (for release next/promote)
    --write      Write changes (for log-collect/log-merge)
    -h, --help   Show help

EXAMPLES:
    ribir-bot pr-fill                      Auto-fill current PR
    ribir-bot pr-regen 123                 Regenerate PR #123
    ribir-bot pr-summary "be concise"      Regenerate with context
    ribir-bot log-collect --version 0.5.0  Collect merged PRs
    ribir-bot release next alpha           Preview alpha release
    ribir-bot release next alpha --execute Execute alpha release
    ribir-bot release prepare --version 0.5.0
    ribir-bot release highlights           Regenerate highlights
"#;

fn print_release_help() {
  print!(
    r#"ribir-bot release - RC/Stable release automation

USAGE:
    ribir-bot release <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    next <level> Full release (alpha|rc|patch|minor|major)
                 Default: dry-run. Use --execute to apply.
    prepare      Prepare RC release (archive, merge, highlights, PR)
    publish      Publish GitHub release
    promote      Promote RC to stable (default: dry-run)
    highlights   Regenerate highlights in CHANGELOG.md
    social-card  Generate social card from highlights (coming soon)
    verify       Verify release state

OPTIONS:
    --version VER    Target version (required for prepare/promote)
    --execute        Execute changes (required for next/promote)
    --dry-run        Preview without applying

EXAMPLES:
    ribir-bot release next alpha           Preview next alpha release
    ribir-bot release next alpha --execute Execute alpha release
    ribir-bot release promote --version 0.5.0 --execute
"#
  );
}

pub fn parse_args() -> Result<Config> {
  let mut args = pico_args::Arguments::from_env();

  if args.contains(["-h", "--help"]) {
    let cmd: Option<String> = args.opt_free_from_str()?;
    match cmd.as_deref() {
      Some("release") => print_release_help(),
      _ => print!("{HELP}"),
    }
    std::process::exit(0);
  }

  let dry_run = args.contains("--dry-run");
  let execute = args.contains("--execute");

  let cmd_str: Option<String> = args.opt_free_from_str()?;
  let command = match cmd_str.as_deref() {
    // PR commands (update PR body)
    Some("pr-fill") => {
      let pr_id: Option<String> = args.opt_free_from_str()?;
      Cmd::Pr(PrCmd { pr_id, mode: PrMode::Auto })
    }
    Some("pr-regen") => {
      let pr_id: Option<String> = args.opt_free_from_str()?;
      let context: Option<String> = args.opt_free_from_str()?;
      Cmd::Pr(PrCmd { pr_id, mode: PrMode::Regenerate(context) })
    }
    Some("pr-summary") => {
      let pr_id: Option<String> = args.opt_free_from_str()?;
      let context: Option<String> = args.opt_free_from_str()?;
      Cmd::Pr(PrCmd { pr_id, mode: PrMode::SummaryOnly(context) })
    }
    Some("pr-entry") => {
      let pr_id: Option<String> = args.opt_free_from_str()?;
      let context: Option<String> = args.opt_free_from_str()?;
      Cmd::Pr(PrCmd { pr_id, mode: PrMode::ChangelogOnly(context) })
    }
    // Changelog commands (update CHANGELOG.md)
    Some("log-collect") => {
      let version = args
        .opt_value_from_str("--version")?
        .ok_or("--version required for log-collect")?;
      let write = args.contains("--write");
      Cmd::Collect { version, write }
    }
    Some("log-merge") => {
      let version = args
        .opt_value_from_str("--version")?
        .ok_or("--version required for log-merge")?;
      let write = args.contains("--write");
      Cmd::Merge { version, write }
    }
    Some("log-verify") => Cmd::Verify,
    // Release commands
    Some("release") => Cmd::Release(parse_release_args(&mut args)?),
    Some(other) => return Err(format!("Unknown command: {other}").into()),
    None => {
      print!("{HELP}");
      std::process::exit(0);
    }
  };

  let remaining = args.finish();
  if !remaining.is_empty() {
    return Err(format!("Unexpected arguments: {:?}", remaining).into());
  }

  // For release next/promote: default is dry-run, need --execute to run
  // For other commands: default is execute, use --dry-run to preview
  let effective_dry_run = match &command {
    Cmd::Release(ReleaseCmd::Next { .. }) | Cmd::Release(ReleaseCmd::Promote { .. }) => {
      // Default dry-run unless --execute is passed
      !execute
    }
    _ => dry_run,
  };

  Ok(Config { command, dry_run: effective_dry_run })
}

fn parse_release_args(args: &mut pico_args::Arguments) -> Result<ReleaseCmd> {
  let subcmd: Option<String> = args.opt_free_from_str()?;
  let version: Option<String> = args.opt_value_from_str("--version")?;

  match subcmd.as_deref() {
    Some("next") => {
      let level: String = args
        .opt_free_from_str()?
        .ok_or("Level required: alpha, rc, patch, minor, major")?;
      Ok(ReleaseCmd::Next { level })
    }
    Some("prepare") => {
      let version = version.ok_or("--version required for prepare")?;
      Ok(ReleaseCmd::Prepare { version })
    }
    Some("publish") => {
      let pr_number: Option<String> = args.opt_free_from_str()?;
      Ok(ReleaseCmd::Publish { pr_id: pr_number })
    }
    Some("promote") => {
      let version = version.ok_or("--version required for promote")?;
      Ok(ReleaseCmd::Promote { version })
    }
    Some("highlights") => {
      let context: Option<String> = args.opt_free_from_str()?;
      Ok(ReleaseCmd::Highlights { context })
    }
    Some("social-card") => Ok(ReleaseCmd::SocialCard),
    Some("verify") | None => Ok(ReleaseCmd::Verify),
    Some(other) => Err(format!("Unknown release subcommand: {other}").into()),
  }
}

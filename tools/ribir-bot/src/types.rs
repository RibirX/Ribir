//! Shared types for ribir-bot.

use std::error::Error;

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

// ============================================================================
// Core Configuration
// ============================================================================

/// Result type alias for the entire crate.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Ribir Bot - Unified CLI for PR, changelog, and release automation.
#[derive(Debug, Parser)]
#[command(name = "ribir-bot")]
#[command(about = "Unified CLI for PR, changelog, and release automation")]
#[command(after_help = "EXAMPLES:
    ribir-bot pr fill                      Auto-fill current PR
    ribir-bot pr regen 123                 Regenerate PR #123
    ribir-bot pr summary --context \"be concise\"
    ribir-bot log collect --version 0.5.0  Collect merged PRs
    ribir-bot release next alpha           Preview alpha release
    ribir-bot release next alpha --execute Execute alpha release
    ribir-bot release enter-rc --version 0.5.0
    ribir-bot release highlights           Regenerate highlights")]
pub struct Config {
  #[command(subcommand)]
  pub command: Cmd,
  /// Preview without applying changes
  #[arg(long, global = true)]
  pub dry_run: bool,
}

/// Command enumeration.
#[derive(Debug, Subcommand)]
pub enum Cmd {
  /// PR commands (update PR body with AI)
  Pr {
    #[command(subcommand)]
    cmd: PrSubCmd,
  },
  /// Changelog commands
  Log {
    #[command(subcommand)]
    cmd: LogSubCmd,
  },
  /// Release automation commands
  Release {
    #[command(subcommand)]
    cmd: ReleaseCmd,
  },
  /// Workflow commands (for GitHub Actions)
  Workflow {
    #[command(subcommand)]
    cmd: WorkflowCmd,
  },
}

/// PR subcommands
#[derive(Debug, Subcommand)]
pub enum PrSubCmd {
  /// Auto-fill placeholders
  Fill {
    /// PR number (defaults to current PR)
    pr_id: Option<String>,
  },
  /// Regenerate all content
  Regen {
    /// PR number (defaults to current PR)
    pr_id: Option<String>,
    /// Additional context for generation
    #[arg(long)]
    context: Option<String>,
  },
  /// Regenerate summary only
  Summary {
    /// PR number (defaults to current PR)
    pr_id: Option<String>,
    /// Additional context for generation
    #[arg(long)]
    context: Option<String>,
  },
  /// Regenerate changelog entry only
  Entry {
    /// PR number (defaults to current PR)
    pr_id: Option<String>,
    /// Additional context for generation
    #[arg(long)]
    context: Option<String>,
  },
}

/// Changelog subcommands
#[derive(Debug, Subcommand)]
pub enum LogSubCmd {
  /// Collect merged PRs into changelog
  Collect {
    /// Target version
    #[arg(long)]
    version: String,
    /// Write changes to file
    #[arg(long)]
    write: bool,
  },
  /// Merge pre-release versions
  Merge {
    /// Target version
    #[arg(long)]
    version: String,
    /// Write changes to file
    #[arg(long)]
    write: bool,
  },
  /// Verify changelog structure
  Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SectionKind {
  Features,
  Fixed,
  Changed,
  Performance,
  Documentation,
  Breaking,
  Internal,
}

impl SectionKind {
  pub const ALL: &'static [SectionKind] = &[
    Self::Features,
    Self::Fixed,
    Self::Changed,
    Self::Performance,
    Self::Documentation,
    Self::Breaking,
    Self::Internal,
  ];

  pub fn from_str(s: &str) -> Option<Self> {
    match s.trim().to_lowercase().as_str() {
      "feat" | "feature" | "features" => Some(Self::Features),
      "fix" | "fixed" => Some(Self::Fixed),
      "change" | "changed" => Some(Self::Changed),
      "perf" | "performance" => Some(Self::Performance),
      "docs" | "doc" | "documentation" => Some(Self::Documentation),
      "breaking" | "break" => Some(Self::Breaking),
      "internal" | "chore" | "refactor" | "other" => Some(Self::Internal),
      _ => None,
    }
  }

  pub fn header(&self) -> String {
    let (emoji, name) = match self {
      Self::Features => ("🎨", "Features"),
      Self::Fixed => ("🐛", "Fixed"),
      Self::Changed => ("🔄", "Changed"),
      Self::Performance => ("⚡", "Performance"),
      Self::Documentation => ("📚", "Documentation"),
      Self::Breaking => ("💥", "Breaking"),
      Self::Internal => ("🔧", "Internal"),
    };
    format!("{} {}", emoji, name)
  }
}

/// PR data for changelog collection.
#[derive(Deserialize)]
pub struct PR {
  pub number: u32,
  pub title: String,
  pub body: Option<String>,
  pub author: Author,
}

#[derive(Deserialize)]
pub struct Author {
  pub login: String,
}

// ============================================================================
// PR Types
// ============================================================================

pub const SUMMARY_PLACEHOLDER: &str =
  "> 🤖 *Leave this placeholder to let AI generate, or replace with your summary.*";
pub const CHANGELOG_PLACEHOLDER: &str =
  "> 🤖 *Leave this placeholder to let AI generate, or replace with your entries:*";
pub const SKIP_CHANGELOG_CHECKED: &str =
  "- [x] 🔧 No changelog needed (tests, CI, infra, or unreleased fix)";

impl PrSubCmd {
  pub fn pr_id(&self) -> Option<&str> {
    match self {
      Self::Fill { pr_id } => pr_id.as_deref(),
      Self::Regen { pr_id, .. } => pr_id.as_deref(),
      Self::Summary { pr_id, .. } => pr_id.as_deref(),
      Self::Entry { pr_id, .. } => pr_id.as_deref(),
    }
  }

  pub fn context(&self) -> Option<&str> {
    match self {
      Self::Fill { .. } => None,
      Self::Regen { context, .. } => context.as_deref(),
      Self::Summary { context, .. } => context.as_deref(),
      Self::Entry { context, .. } => context.as_deref(),
    }
  }

  pub fn needs(&self, body: &str) -> (bool, bool) {
    match self {
      Self::Fill { .. } => {
        let needs_summary = body.contains(SUMMARY_PLACEHOLDER);
        let needs_changelog = body.contains(CHANGELOG_PLACEHOLDER);
        (needs_summary, needs_changelog)
      }
      Self::Regen { .. } => (true, true),
      Self::Summary { .. } => (true, false),
      Self::Entry { .. } => (false, true),
    }
  }

  pub fn log_status(&self) {
    match self {
      Self::Fill { .. } => {}
      Self::Regen { context: Some(ctx), .. } => eprintln!("⚡ Regenerating all with context: {ctx}"),
      Self::Regen { context: None, .. } => eprintln!("⚡ Regenerating all content"),
      Self::Summary { context: Some(ctx), .. } => {
        eprintln!("📝 Regenerating summary with context: {ctx}")
      }
      Self::Summary { context: None, .. } => eprintln!("📝 Regenerating summary only"),
      Self::Entry { context: Some(ctx), .. } => {
        eprintln!("📋 Regenerating changelog with context: {ctx}")
      }
      Self::Entry { context: None, .. } => eprintln!("📋 Regenerating changelog only"),
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct GeminiResponse {
  pub summary: String,
  pub changelog: String,
  #[serde(default)]
  pub skip_changelog: bool,
}

#[derive(Deserialize)]
pub struct PRView {
  pub title: String,
  pub body: String,
}

#[derive(Deserialize)]
pub struct PRCommits {
  pub commits: Vec<Commit>,
}

#[derive(Deserialize)]
pub struct Commit {
  #[serde(rename = "messageHeadline")]
  pub message_headline: String,
  #[serde(rename = "messageBody")]
  pub message_body: String,
}

// ============================================================================
// Release Types
// ============================================================================

/// Release subcommands
#[derive(Debug, Subcommand)]
pub enum ReleaseCmd {
  /// Full release (alpha|rc|patch|minor|major). Default: dry-run, use --execute
  /// to apply.
  Next {
    /// Release level
    #[arg(value_enum)]
    level: ReleaseLevel,
    /// Execute changes (required to apply)
    #[arg(long)]
    execute: bool,
  },
  /// Enter RC phase (branch + PR + RC.1)
  EnterRc {
    /// Target version
    #[arg(long)]
    version: String,
  },
  /// Publish GitHub release
  Publish {
    /// PR number
    pr_id: Option<String>,
  },
  /// Release stable version (auto-detect from branch). Default: dry-run.
  Stable {
    /// Target version (optional, auto-detect from branch)
    #[arg(long)]
    version: Option<String>,
    /// Execute changes (required to apply)
    #[arg(long)]
    execute: bool,
  },
  /// Regenerate highlights in CHANGELOG.md
  Highlights {
    /// Additional context for generation
    #[arg(long)]
    context: Option<String>,
  },
  /// Generate social card (coming soon)
  SocialCard,
  /// Verify release state
  Verify,
}

/// Release level for the `next` command
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReleaseLevel {
  Alpha,
  Rc,
  Patch,
  Minor,
  Major,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Highlight {
  pub emoji: String,
  pub description: String,
}

#[derive(Deserialize)]
pub struct HighlightsResponse {
  pub highlights: Vec<Highlight>,
}

// ============================================================================
// Workflow Types
// ============================================================================

/// Workflow subcommands (for GitHub Actions)
#[derive(Debug, Subcommand)]
pub enum WorkflowCmd {
  /// Parse GitHub event and execute appropriate action
  HandleEvent {
    /// Path to GitHub event JSON file
    #[arg(long, env = "GITHUB_EVENT_PATH")]
    event_path: std::path::PathBuf,
    /// For workflow_dispatch, the PR number input
    #[arg(long)]
    dispatch_pr: Option<String>,
    /// For workflow_dispatch (rc-bot), the command input
    #[arg(long)]
    dispatch_command: Option<String>,
  },
  /// Post help comment on a PR
  PostHelp {
    /// PR number
    #[arg(long)]
    pr: String,
    /// Bot type (pr or rc)
    #[arg(long, value_enum)]
    bot: BotType,
  },
}

/// Bot type for help messages.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BotType {
  Pr,
  Rc,
}

/// Result from parsing a workflow event.
#[derive(Debug, Serialize)]
pub struct EventResult {
  pub should_run: bool,
  pub command: Option<String>,
  pub pr_id: Option<String>,
  pub context: Option<String>,
  pub branch: Option<String>,
  pub show_help: bool,
}

impl Default for EventResult {
  fn default() -> Self {
    Self {
      should_run: false,
      command: None,
      pr_id: None,
      context: None,
      branch: None,
      show_help: false,
    }
  }
}

impl EventResult {
  /// Output as GitHub Actions outputs format.
  pub fn to_github_output(&self) -> String {
    let mut lines = Vec::new();
    lines.push(format!("should_run={}", self.should_run));
    lines.push(format!("show_help={}", self.show_help));
    if let Some(ref cmd) = self.command {
      lines.push(format!("command={cmd}"));
    }
    if let Some(ref id) = self.pr_id {
      lines.push(format!("pr_id={id}"));
    }
    if let Some(ref ctx) = self.context {
      // Use heredoc syntax for (potentially) multi-line context.
      // IMPORTANT: the delimiter must not be attacker-controllable; otherwise a
      // comment body that contains the delimiter can inject additional GitHub
      // Actions outputs.
      let delimiter = Self::unique_output_delimiter(ctx);
      lines.push(format!("context<<{delimiter}\n{ctx}\n{delimiter}"));
    }
    if let Some(ref branch) = self.branch {
      lines.push(format!("branch={branch}"));
    }
    lines.join("\n")
  }

  fn unique_output_delimiter(content: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let pid = std::process::id();
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_nanos())
      .unwrap_or(0);

    for attempt in 0u32..1000 {
      let delimiter = format!("RIBIR_BOT_CTX_{pid}_{nanos}_{attempt}");
      if !content.contains(&delimiter) {
        return delimiter;
      }
    }

    // Extremely unlikely fallback.
    "RIBIR_BOT_CTX".to_string()
  }
}

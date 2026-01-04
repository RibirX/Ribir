//! Shared types for ribir-bot.

use std::error::Error;

use serde::{Deserialize, Serialize};

// ============================================================================
// Core Configuration
// ============================================================================

/// Result type alias for the entire crate.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Top-level configuration.
#[derive(Debug)]
pub struct Config {
  pub command: Cmd,
  pub dry_run: bool,
}

/// Command enumeration.
#[derive(Debug)]
pub enum Cmd {
  Pr(PrCmd),
  Collect { version: String, write: bool },
  Merge { version: String, write: bool },
  Verify,
  Release(ReleaseCmd),
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
    format!("### {} {}", emoji, name)
  }
}

/// PR data for changelog collection.
#[derive(Deserialize)]
pub struct PR {
  pub number: u32,
  pub title: String,
  pub body: Option<String>,
  pub author: Author,
  pub merged_at: Option<String>,
}

#[derive(Deserialize)]
pub struct Author {
  pub login: String,
}

// ============================================================================
// PR Types
// ============================================================================

#[derive(Debug)]
pub struct PrCmd {
  pub pr_id: Option<String>,
  pub mode: PrMode,
}

#[derive(Debug)]
pub enum PrMode {
  Auto,
  Regenerate(Option<String>),
  SummaryOnly(Option<String>),
  ChangelogOnly(Option<String>),
}

pub const SUMMARY_PLACEHOLDER: &str =
  "> 🤖 *Leave this placeholder to let AI generate, or replace with your summary.*";
pub const CHANGELOG_PLACEHOLDER: &str =
  "> 🤖 *Leave this placeholder to let AI generate, or replace with your entries:*";
pub const SKIP_CHANGELOG_CHECKED: &str =
  "- [x] 🔧 No changelog needed (tests, CI, infra, or unreleased fix)";

impl PrMode {
  pub fn needs(&self, body: &str) -> (bool, bool) {
    match self {
      Self::Auto => {
        let needs_summary = body.contains(SUMMARY_PLACEHOLDER);
        let needs_changelog = body.contains(CHANGELOG_PLACEHOLDER);
        (needs_summary, needs_changelog)
      }
      Self::Regenerate(_) => (true, true),
      Self::SummaryOnly(_) => (true, false),
      Self::ChangelogOnly(_) => (false, true),
    }
  }

  pub fn context(&self) -> Option<&str> {
    match self {
      Self::Regenerate(ctx) | Self::SummaryOnly(ctx) | Self::ChangelogOnly(ctx) => ctx.as_deref(),
      Self::Auto => None,
    }
  }

  pub fn log_status(&self) {
    match self {
      Self::Regenerate(Some(ctx)) => eprintln!("⚡ Regenerating all with context: {ctx}"),
      Self::Regenerate(None) => eprintln!("⚡ Regenerating all content"),
      Self::SummaryOnly(Some(ctx)) => eprintln!("📝 Regenerating summary with context: {ctx}"),
      Self::SummaryOnly(None) => eprintln!("📝 Regenerating summary only"),
      Self::ChangelogOnly(Some(ctx)) => eprintln!("📋 Regenerating changelog with context: {ctx}"),
      Self::ChangelogOnly(None) => eprintln!("📋 Regenerating changelog only"),
      Self::Auto => {}
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

#[derive(Debug)]
pub enum ReleaseCmd {
  Prepare { version: String },
  Publish { pr_id: Option<String> },
  Promote { version: String },
  Next { level: String },
  Verify,
  Highlights { context: Option<String> },
  SocialCard,
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

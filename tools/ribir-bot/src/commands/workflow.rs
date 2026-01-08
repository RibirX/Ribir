//! Workflow event handling for GitHub Actions.
//!
//! This module processes GitHub workflow events (pull_request, issue_comment,
//! workflow_dispatch) and determines what action to take.

use std::path::Path;

use serde::Deserialize;

use crate::{
  external::{self, comment_on_pr},
  types::{BotType, Config, EventResult, Result, WorkflowCmd},
  utils::is_rc_pr,
};

// ============================================================================
// Public Entry Point
// ============================================================================

/// Execute a workflow command.
pub fn cmd_workflow(_config: &Config, cmd: &WorkflowCmd) -> Result<()> {
  match cmd {
    WorkflowCmd::HandleEvent { event_path, dispatch_pr, dispatch_command } => {
      // Determine event type from GITHUB_EVENT_NAME env var
      let event_name = std::env::var("GITHUB_EVENT_NAME").unwrap_or_else(|_| "unknown".to_string());
      let repo =
        std::env::var("GITHUB_REPOSITORY").unwrap_or_else(|_| "unknown/unknown".to_string());

      // Determine which bot we're running based on GITHUB_WORKFLOW env var
      let workflow = std::env::var("GITHUB_WORKFLOW").unwrap_or_default();
      let is_rc_bot = workflow.to_lowercase().contains("rc");

      let result = if is_rc_bot {
        handle_rc_bot_event(
          &event_name,
          event_path,
          dispatch_pr.as_deref(),
          dispatch_command.as_deref(),
          &repo,
        )?
      } else {
        handle_pr_bot_event(&event_name, event_path, dispatch_pr.as_deref(), &repo)?
      };

      // Output result for GitHub Actions
      println!("{}", result.to_github_output());
      Ok(())
    }
    WorkflowCmd::PostHelp { pr, bot } => post_help(pr, *bot),
  }
}

// ============================================================================
// Bot Commands
// ============================================================================

#[derive(Debug, PartialEq, Eq)]
enum BotCommand {
  // PR Bot Commands
  PrFill(Option<String>),
  PrRegen(Option<String>),
  PrSummary(Option<String>),
  PrEntry(Option<String>),

  // RC Bot Commands
  ReleaseHighlights(Option<String>),
  ReleaseSocialCard(Option<String>),
  ReleaseStable,

  // Common
  Help,
}

impl BotCommand {
  fn parse(body: &str) -> Option<Self> {
    let magic = "@ribir-bot";
    // Find position where the command starts (beginning of body or after a newline)
    let start_idx = if body.starts_with(magic) {
      Some(0)
    } else {
      body.find(&format!("\n{}", magic)).map(|i| i + 1)
    }?;

    let rest = &body[start_idx + magic.len()..];
    let trimmed = rest.trim_start();

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next()?;
    let context = parts
      .next()
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());

    match command {
      "pr-fill" => Some(Self::PrFill(context)),
      "pr-regen" => Some(Self::PrRegen(context)),
      "pr-summary" => Some(Self::PrSummary(context)),
      "pr-entry" => Some(Self::PrEntry(context)),
      "release-highlights" => Some(Self::ReleaseHighlights(context)),
      "release-social-card" => Some(Self::ReleaseSocialCard(context)),
      "release-stable" => Some(Self::ReleaseStable),
      "help" => Some(Self::Help),
      _ => None,
    }
  }
}

// ============================================================================
// GitHub Event JSON Structures
// ============================================================================

#[derive(Deserialize)]
struct PullRequestEvent {
  pull_request: PullRequest,
}

#[derive(Deserialize)]
struct IssueCommentEvent {
  issue: Issue,
  comment: Comment,
}

#[derive(Deserialize)]
struct PullRequest {
  number: u32,
  #[serde(rename = "baseRefName", alias = "base")]
  base: BaseRef,
  #[serde(rename = "headRefName", alias = "head")]
  head: HeadRef,
  user: User,
}

#[derive(Deserialize)]
struct BaseRef {
  #[serde(rename = "ref")]
  ref_name: String,
}

#[derive(Deserialize)]
struct HeadRef {
  #[serde(rename = "ref")]
  ref_name: String,
}

#[derive(Deserialize)]
struct Issue {
  number: u32,
  user: User,
  pull_request: Option<serde_json::Value>, // Just to check if it's a PR
}

#[derive(Deserialize)]
struct Comment {
  id: u64,
  body: String,
  user: User,
}

#[derive(Deserialize)]
struct User {
  login: String,
}

// ============================================================================
// PR Bot Event Handler
// ============================================================================

/// Handle events for pr-bot workflow.
pub fn handle_pr_bot_event(
  event_name: &str, event_path: &Path, dispatch_pr: Option<&str>, repo: &str,
) -> Result<EventResult> {
  let event_data = std::fs::read_to_string(event_path)?;
  let mut result = EventResult::default();

  match event_name {
    "pull_request" => {
      let event: PullRequestEvent = serde_json::from_str(&event_data)?;
      let pr = &event.pull_request;

      if pr.user.login == "dependabot[bot]" {
        eprintln!("⏭️ Skipping dependabot PR");
        return Ok(result);
      }

      if pr.base.ref_name != "master" {
        eprintln!("⏭️ PR does not target master");
        return Ok(result);
      }

      if is_rc_pr(&pr.base.ref_name, &pr.head.ref_name) {
        eprintln!("⏭️ RC PR detected - skipping pr-bot");
        return Ok(result);
      }

      result.set_run("pr-fill", &pr.number.to_string());
    }

    "workflow_dispatch" => {
      if let Some(pr) = dispatch_pr {
        result.set_run("pr-fill", pr);
      }
    }

    "issue_comment" => {
      let event: IssueCommentEvent = serde_json::from_str(&event_data)?;
      if event.issue.pull_request.is_none() {
        return Ok(result);
      }

      let pr_number = event.issue.number;
      let (base, head) = external::get_pr_info(pr_number, repo)?;

      if is_rc_pr(&base, &head) {
        eprintln!("⏭️ RC PR detected - use @ribir-bot release-* commands instead");
        return Ok(result);
      }

      if !external::check_permission(&event.comment.user.login, &event.issue.user.login, repo)? {
        eprintln!("⏭️ User does not have permission");
        return Ok(result);
      }

      match BotCommand::parse(&event.comment.body) {
        Some(cmd) => {
          result.comment_id = Some(event.comment.id);
          match cmd {
            BotCommand::PrFill(ctx) => {
              result.set_run_with_context("pr-fill", &pr_number.to_string(), ctx)
            }
            BotCommand::PrRegen(ctx) => {
              result.set_run_with_context("pr-regen", &pr_number.to_string(), ctx)
            }
            BotCommand::PrSummary(ctx) => {
              result.set_run_with_context("pr-summary", &pr_number.to_string(), ctx)
            }
            BotCommand::PrEntry(ctx) => {
              result.set_run_with_context("pr-entry", &pr_number.to_string(), ctx)
            }
            BotCommand::Help => result.show_help = true,
            _ => {
              // Check if it looks like a valid bot command but for RC bot
              if matches!(
                cmd,
                BotCommand::ReleaseHighlights(_)
                  | BotCommand::ReleaseSocialCard(_)
                  | BotCommand::ReleaseStable
              ) {
                eprintln!("⏭️ RC command detected - handled by rc-bot");
              } else {
                // Ignore unknown or unrelated commands
              }
            }
          }
        }
        None => result.show_help = is_bot_mention_only(&event.comment.body),
      }
    }
    _ => eprintln!("⏭️ Unknown event: {event_name}"),
  }

  Ok(result)
}

// ============================================================================
// RC Bot Event Handler
// ============================================================================

/// Handle events for rc-bot workflow.
pub fn handle_rc_bot_event(
  event_name: &str, event_path: &Path, dispatch_pr: Option<&str>, dispatch_command: Option<&str>,
  repo: &str,
) -> Result<EventResult> {
  let event_data = std::fs::read_to_string(event_path)?;
  let mut result = EventResult::default();

  match event_name {
    "workflow_dispatch" => {
      if let (Some(pr), Some(cmd)) = (dispatch_pr, dispatch_command) {
        let pr_num: u32 = pr.parse()?;
        let (base, head) = external::get_pr_info(pr_num, repo)?;

        if is_rc_pr(&base, &head) {
          result.should_run = true;
          result.command = Some(cmd.to_string());
          result.pr_id = Some(pr.to_string());
          result.branch = Some(head);
        } else {
          eprintln!("❌ PR #{pr} is not an RC PR (requires base:master + head:release-X.Y.x)");
        }
      }
    }

    "issue_comment" => {
      let event: IssueCommentEvent = serde_json::from_str(&event_data)?;
      if event.issue.pull_request.is_none() {
        return Ok(result);
      }

      let pr_number = event.issue.number;
      let (base, head) = external::get_pr_info(pr_number, repo)?;

      if !is_rc_pr(&base, &head) {
        eprintln!("ℹ️ Not an RC PR - release-* commands only work on RC PRs");
        return Ok(result);
      }

      if !external::check_permission(&event.comment.user.login, &event.issue.user.login, repo)? {
        eprintln!("⏭️ User does not have permission");
        return Ok(result);
      }

      match BotCommand::parse(&event.comment.body) {
        Some(cmd) => {
          result.comment_id = Some(event.comment.id);
          match cmd {
            BotCommand::ReleaseHighlights(ctx) => result.set_run_with_branch_context(
              "release-highlights",
              &pr_number.to_string(),
              head,
              ctx,
            ),
            BotCommand::ReleaseSocialCard(ctx) => result.set_run_with_branch_context(
              "release-social-card",
              &pr_number.to_string(),
              head,
              ctx,
            ),
            BotCommand::ReleaseStable => {
              result.set_run_with_branch("release-stable", &pr_number.to_string(), head)
            }
            BotCommand::Help => result.show_help = true,
            _ => {
              // PR bot commands are ignored here or we could show an error
              if matches!(
                cmd,
                BotCommand::PrFill(_)
                  | BotCommand::PrRegen(_)
                  | BotCommand::PrSummary(_)
                  | BotCommand::PrEntry(_)
              ) {
                eprintln!("⏭️ PR bot command in RC context - handled by pr-bot");
              }
            }
          }
        }
        None => result.show_help = is_bot_mention_only(&event.comment.body),
      }
    }
    _ => eprintln!("⏭️ Unknown event: {event_name}"),
  }

  Ok(result)
}

fn is_bot_mention_only(body: &str) -> bool { body.trim() == "@ribir-bot" }

// ============================================================================
// Extensions for EventResult
// ============================================================================

impl EventResult {
  fn set_run(&mut self, cmd: &str, pr: &str) {
    self.should_run = true;
    self.command = Some(cmd.to_string());
    self.pr_id = Some(pr.to_string());
  }

  fn set_run_with_context(&mut self, cmd: &str, pr: &str, context: Option<String>) {
    self.set_run(cmd, pr);
    self.context = context;
  }

  fn set_run_with_branch(&mut self, cmd: &str, pr: &str, branch: String) {
    self.set_run(cmd, pr);
    self.branch = Some(branch);
  }

  fn set_run_with_branch_context(
    &mut self, cmd: &str, pr: &str, branch: String, context: Option<String>,
  ) {
    self.set_run_with_branch(cmd, pr, branch);
    self.context = context;
  }
}

// ============================================================================
// Help Messages
// ============================================================================

const PR_BOT_HELP: &str = r#"## 🤖 Ribir Bot Commands

Hi! I can help with generating PR summaries and changelog entries.

### Command Format
`@ribir-bot COMMAND [CONTEXT]`

Commands must start with `@ribir-bot` at the beginning of a line.

### Available Commands
- `@ribir-bot pr-fill` - Auto-fill placeholders in PR body
- `@ribir-bot pr-regen [context]` - Regenerate summary and changelog
- `@ribir-bot pr-summary [context]` - Regenerate only the summary section
- `@ribir-bot pr-entry [context]` - Regenerate only the changelog entries
- `@ribir-bot help` - Show this help message

### Note
Only users with write access can use these commands."#;

const RC_BOT_HELP: &str = r#"## 🚀 RC Bot Commands

Hi! I help manage Release Candidate PRs.

### Command Format
`@ribir-bot COMMAND [CONTEXT]`

Commands must start with `@ribir-bot` at the beginning of a line.

### Available Commands
- `@ribir-bot release-highlights [context]` - Regenerate highlights section
- `@ribir-bot release-social-card` - Generate social card
- `@ribir-bot release-stable` - Publish stable release and merge this PR
- `@ribir-bot help` - Show this help message

### Note
Only users with write access can use these commands."#;

/// Post help message as a PR comment.
pub fn post_help(pr_number: &str, bot_type: BotType) -> Result<()> {
  let help_text = match bot_type {
    BotType::Pr => PR_BOT_HELP,
    BotType::Rc => RC_BOT_HELP,
  };
  comment_on_pr(pr_number, help_text)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_bot_command() {
    assert_eq!(BotCommand::parse("@ribir-bot pr-fill"), Some(BotCommand::PrFill(None)));

    assert_eq!(
      BotCommand::parse("@ribir-bot pr-fill be concise"),
      Some(BotCommand::PrFill(Some("be concise".to_string())))
    );

    assert_eq!(
      BotCommand::parse("@ribir-bot pr-regen be concise"),
      Some(BotCommand::PrRegen(Some("be concise".to_string())))
    );

    assert_eq!(BotCommand::parse("Some text\n@ribir-bot help\nmore text"), Some(BotCommand::Help));

    // Multi-line context
    assert_eq!(
      BotCommand::parse("@ribir-bot pr-summary with\nmultiple lines"),
      Some(BotCommand::PrSummary(Some("with\nmultiple lines".to_string())))
    );

    assert_eq!(BotCommand::parse("no bot mention here"), None);
    assert_eq!(BotCommand::parse("@ribir-bot"), None);
  }
}

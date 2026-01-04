//! PR command implementation.

use crate::{
  changelog::{MARKER_END as CHANGELOG_END_MARKER, MARKER_START as CHANGELOG_START_MARKER},
  external::{call_gemini_with_fallback, extract_json, gh_diff, gh_edit_body, gh_json},
  types::{Commit, Config, GeminiResponse, PRCommits, PRView, PrSubCmd, Result, SKIP_CHANGELOG_CHECKED},
  utils::{sanitize_markdown, truncate},
};

const SUMMARY_START_MARKER: &str = "<!-- RIBIR_SUMMARY_START -->";
const SUMMARY_END_MARKER: &str = "<!-- RIBIR_SUMMARY_END -->";

const PR_PROMPT_TEMPLATE: &str = r#"Analyze this GitHub Pull Request and generate a summary + changelog.

## PR Information

**Title**: {title}

**Author's Description**:
{body}

**Commits** (author's step-by-step intent):
{commits}

**Code Diff** (actual changes - may be truncated for large PRs):
{diff}

## Analysis Guidelines

1. **Cross-reference sources**: Commits explain WHY, diff shows WHAT. Use both to understand the full picture.
2. **Trust diff over commit messages**: If there's a conflict, the actual code change is ground truth.
3. **For truncated diffs**: If the diff ends with "(Diff truncated...)" and you need more context, use `gh pr diff {pr_id}` to see full changes, or `gh pr view {pr_id} --json files` to see affected files.
4. **Path semantics**: Changes only in `/tests/`, `.github/`, `/scripts/`, or `Cargo.toml` (deps only) often indicate internal/infra work.

## Output Requirements

1. **Summary** with this EXACT structure:
   **Context**: One sentence explaining why this change is needed.
   **Changes**:
   - Bullet points of what changed (from diff, not just commit messages).
   Keep sentences short and clear.

2. **Changelog decision**:
   - skip_changelog=true for: CI/CD, bot updates, tests, internal tools, infrastructure.
   - skip_changelog=false for: features, bug fixes, breaking changes, docs, user-facing items.

3. **Changelog entries** (if not skipped): `- type(scope): description`
   Types: feat, fix, change, docs, breaking, perf
   Scopes: core, gpu, macros, widgets, themes, painter, cli, text, tools

## Output Format

Return ONLY valid JSON:
{"summary": "...", "changelog": "...", "skip_changelog": true/false}

Examples:
{"summary": "**Context**: The renderer was slow on large trees.\n**Changes**:\n- Refactored rendering pipeline to use batching.\n- Improved performance by 40%.", "changelog": "- perf(core): optimize rendering with batching", "skip_changelog": false}
{"summary": "**Context**: CI failing on Windows.\n**Changes**:\n- Fixed path handling in workflow file.", "changelog": "", "skip_changelog": true}"#;

/// Execute PR command.
pub fn cmd_pr(config: &Config, pr_cmd: &PrSubCmd) -> Result<()> {
  pr_cmd.log_status();

  let pr = gh_json::<PRView>(pr_cmd.pr_id(), "title,body")?;
  let mut body = pr.body.clone();

  let mut modified = false;
  let (needs_summary, mut needs_changelog) = pr_cmd.needs(&body);

  // If user manually skipped changelog, clean it up and skip AI generation
  if needs_changelog && body.contains(SKIP_CHANGELOG_CHECKED) {
    eprintln!("⏭️  User skipped changelog - cleaning up placeholder");
    body =
      replace_section(&body, CHANGELOG_START_MARKER, CHANGELOG_END_MARKER, SKIP_CHANGELOG_CHECKED);
    needs_changelog = false;
    modified = true;
  }

  if needs_summary || needs_changelog {
    let commits = gh_json::<PRCommits>(pr_cmd.pr_id(), "commits")?.commits;
    let diff = gh_diff(pr_cmd.pr_id())?;
    let prompt = build_pr_prompt(&pr, &body, &format_commits(&commits), &diff, pr_cmd);
    let response = generate_pr_content(&prompt)?;
    body = update_pr_body(&body, &response, needs_summary, needs_changelog);
    modified = true;
  }

  if modified {
    save_pr_body(config, pr_cmd.pr_id(), &body)
  } else {
    println!("No placeholders found - skipping. Use --regenerate to force.");
    Ok(())
  }
}

fn format_commits(commits: &[Commit]) -> String {
  if commits.is_empty() {
    "(No commits found)".into()
  } else {
    commits
      .iter()
      .map(|c| {
        if c.message_body.is_empty() {
          format!("- {}", c.message_headline)
        } else {
          format!("- {}\n  {}", c.message_headline, c.message_body.replace('\n', "\n  "))
        }
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}

fn build_pr_prompt(pr: &PRView, body: &str, commits: &str, diff: &str, cmd: &PrSubCmd) -> String {
  let pr_id_display = cmd.pr_id().unwrap_or("<current PR>");
  let mut prompt = PR_PROMPT_TEMPLATE
    .replace("{title}", &pr.title)
    .replace("{body}", body)
    .replace("{commits}", commits)
    .replace("{diff}", diff)
    .replace("{pr_id}", pr_id_display);

  if let Some(ctx) = cmd.context() {
    prompt = format!("ADDITIONAL CONTEXT FROM USER:\n{}\n\n{}", ctx, prompt);
  } else if !matches!(cmd, PrSubCmd::Fill { .. }) {
    prompt = format!(
      "TASK: Regenerate the content. The previous output was not satisfactory. Please generate a \
       fresh response.\n\n{}",
      prompt
    );
  }

  prompt
}

fn generate_pr_content(prompt: &str) -> Result<GeminiResponse> {
  let result = call_gemini_with_fallback(prompt)?;
  let json_str = extract_json(&result).ok_or("No JSON found in response")?;
  let response: GeminiResponse =
    serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON: {e}\nRaw: {result}"))?;
  sanitize_response(response)
}

fn update_pr_body(
  body: &str, response: &GeminiResponse, needs_summary: bool, needs_changelog: bool,
) -> String {
  let mut result = body.to_string();

  if needs_summary {
    result = replace_section(&result, SUMMARY_START_MARKER, SUMMARY_END_MARKER, &response.summary);
  }

  if needs_changelog {
    let content =
      if response.skip_changelog { SKIP_CHANGELOG_CHECKED } else { &response.changelog };
    result = replace_section(&result, CHANGELOG_START_MARKER, CHANGELOG_END_MARKER, content);
  }

  result
}

/// Replaces content within marker boundaries.
fn replace_section(body: &str, start_marker: &str, end_marker: &str, new_content: &str) -> String {
  let (Some(start_pos), Some(end_pos)) = (body.find(start_marker), body.find(end_marker)) else {
    return body.to_string();
  };

  let content_start = start_pos + start_marker.len();
  if content_start < end_pos {
    format!("{}\n\n{}\n\n{}", &body[..content_start], new_content, &body[end_pos..])
  } else {
    body.to_string()
  }
}

fn sanitize_response(mut response: GeminiResponse) -> Result<GeminiResponse> {
  response.summary = sanitize_markdown(&response.summary);
  response.changelog = sanitize_markdown(&response.changelog);

  if response.summary.trim().is_empty() {
    return Err("Empty summary".into());
  }

  // Only validate changelog format if not skipping
  if !response.skip_changelog
    && !response
      .changelog
      .lines()
      .any(|l| l.trim().starts_with('-'))
  {
    return Err("Invalid changelog format".into());
  }

  truncate(&mut response.summary, 1000, "...");
  truncate(&mut response.changelog, 5000, "\n...(truncated)");

  Ok(response)
}

fn save_pr_body(config: &Config, pr_id: Option<&str>, body: &str) -> Result<()> {
  if config.dry_run {
    print_pr_preview(body);
  } else {
    gh_edit_body(pr_id, body)?;
    println!("✅ PR updated successfully!");
  }
  Ok(())
}

fn print_pr_preview(body: &str) {
  println!("\n📝 Preview:\n{}\n", "─".repeat(50));
  println!("{body}");
  println!("{}\n💡 Run without --dry-run to apply.", "─".repeat(50));
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{CHANGELOG_PLACEHOLDER, SUMMARY_PLACEHOLDER};

  #[test]
  fn test_update_pr_body_with_placeholder() {
    let body = format!(
      "## Summary\n{}\n\nPlaceholder text\n\n{}\n\n## Changelog\n{}\n\nChangelog \
       placeholder\n\n{}\n\nOther content",
      SUMMARY_START_MARKER, SUMMARY_END_MARKER, CHANGELOG_START_MARKER, CHANGELOG_END_MARKER
    );
    let response = GeminiResponse {
      summary: "Fixed a bug.".into(),
      changelog: "- fix(core): fix crash".into(),
      skip_changelog: false,
    };

    let updated = update_pr_body(&body, &response, true, true);
    assert!(updated.contains("Fixed a bug."));
    assert!(updated.contains("- fix(core): fix crash"));
    assert!(!updated.contains("Placeholder text"));
    assert!(!updated.contains("Changelog placeholder"));
    assert!(updated.contains("Other content"));
  }

  #[test]
  fn test_update_pr_body_user_skipped_changelog() {
    let body_after_cleanup = format!(
      "## Summary\\n{}\\n\\nOld summary\\n\\n{}\\n\\n## Changelog\\n{}\\n\\n{}\\n\\n{}\\n\\nOther \
       content",
      SUMMARY_START_MARKER,
      SUMMARY_END_MARKER,
      CHANGELOG_START_MARKER,
      SKIP_CHANGELOG_CHECKED,
      CHANGELOG_END_MARKER
    );
    let response = GeminiResponse {
      summary: "New summary".into(),
      changelog: "- should not be used".into(),
      skip_changelog: false,
    };

    let updated = update_pr_body(&body_after_cleanup, &response, true, false);
    assert!(updated.contains("New summary"));
    assert!(updated.contains(SKIP_CHANGELOG_CHECKED));
    assert!(!updated.contains("- should not be used"));
  }

  #[test]
  fn test_update_pr_body_regenerate() {
    let body = format!(
      "## Summary\n{}\n\nOld AI summary\n\n{}\n\n## Changelog\n{}\n\n- old(change): \
       entry\n\n{}\n\nOther content",
      SUMMARY_START_MARKER, SUMMARY_END_MARKER, CHANGELOG_START_MARKER, CHANGELOG_END_MARKER
    );
    let response = GeminiResponse {
      summary: "New AI summary".into(),
      changelog: "- new(change): entry".into(),
      skip_changelog: false,
    };

    let updated = update_pr_body(&body, &response, true, true);
    assert!(updated.contains("New AI summary"));
    assert!(updated.contains("- new(change): entry"));
    assert!(!updated.contains("Old AI summary"));
    assert!(!updated.contains("- old(change): entry"));
    assert!(updated.contains("Other content"));
  }

  #[test]
  fn test_sanitize_response_valid() {
    let response = GeminiResponse {
      summary: "New feature".into(),
      changelog: "- feat(core): add".into(),
      skip_changelog: false,
    };
    assert!(sanitize_response(response).is_ok());
  }

  #[test]
  fn test_sanitize_response_empty_summary() {
    let response = GeminiResponse {
      summary: "   ".into(),
      changelog: "- feat: x".into(),
      skip_changelog: false,
    };
    assert!(sanitize_response(response).is_err());
  }

  #[test]
  fn test_sanitize_response_invalid_changelog() {
    let response = GeminiResponse {
      summary: "OK".into(),
      changelog: "no bullets".into(),
      skip_changelog: false,
    };
    assert!(sanitize_response(response).is_err());
  }

  #[test]
  fn test_mode_needs() {
    let body_with_both = format!("{}\n{}", SUMMARY_PLACEHOLDER, CHANGELOG_PLACEHOLDER);
    let fill = PrSubCmd::Fill { pr_id: None };
    let regen = PrSubCmd::Regen { pr_id: None, context: None };
    let summary = PrSubCmd::Summary { pr_id: None, context: None };
    let entry = PrSubCmd::Entry { pr_id: None, context: None };

    assert_eq!(fill.needs(&body_with_both), (true, true));
    assert_eq!(fill.needs("no placeholders"), (false, false));
    assert_eq!(regen.needs(""), (true, true));
    assert_eq!(summary.needs(""), (true, false));
    assert_eq!(entry.needs(""), (false, true));
  }
}

//! External tool integrations (Gemini AI and GitHub CLI).

use std::{
  io::Write as IoWrite,
  process::{Command, Stdio},
};

use semver::Version;

use crate::{types::Result, utils::run_command};

// ============================================================================
// Gemini AI Integration
// ============================================================================

const PREFERRED_MODELS: &[&str] = &[
  "gemini-3-flash-preview",
  "gemini-2.5-flash",
  "gemini-2.5-flash-lite",
  "gemini-3-pro-preview",
  "gemini-2.5-pro",
];

pub fn call_gemini_with_fallback(prompt: &str) -> Result<String> {
  let mut last_error = String::new();

  for model in PREFERRED_MODELS {
    eprintln!("Trying model: {model}");
    match call_gemini(prompt, model) {
      Ok(res) => {
        eprintln!("✓ Success: {model}");
        return Ok(res);
      }
      Err(e) => {
        eprintln!("✗ Failed: {model} - {e}");
        last_error = e;
      }
    }
  }

  Err(format!("All models failed. Last error: {last_error}").into())
}

fn call_gemini(prompt: &str, model: &str) -> std::result::Result<String, String> {
  let mut child = Command::new("gemini")
    .args(["--model", model, "--approval-mode", "yolo", "-o", "text"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| e.to_string())?;

  if let Some(mut stdin) = child.stdin.take() {
    stdin
      .write_all(prompt.as_bytes())
      .map_err(|e| e.to_string())?;
  }

  let output = child.wait_with_output().map_err(|e| e.to_string())?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).into())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).into())
  }
}

/// Extract JSON from a response string.
pub fn extract_json(s: &str) -> Option<String> {
  let start = s.find('{')?;
  let end = s.rfind('}')?;
  Some(s[start..=end].to_string())
}

// ============================================================================
// GitHub CLI Integration
// ============================================================================

/// Fetch PR data as JSON.
pub fn gh_json<T: for<'de> serde::Deserialize<'de>>(pr_id: Option<&str>, fields: &str) -> Result<T> {
  let mut args = vec!["pr", "view"];
  extend_with_pr_id(&mut args, pr_id);
  args.extend(["--json", fields]);

  let output = run_command("gh", &args)?;
  Ok(serde_json::from_slice(&output.stdout)?)
}

/// Fetch PR diff.
pub fn gh_diff(pr_id: Option<&str>) -> Result<String> {
  let mut args = vec!["pr", "diff"];
  extend_with_pr_id(&mut args, pr_id);
  args.push("--patch");

  let output = run_command("gh", &args)?;
  let full_diff = String::from_utf8_lossy(&output.stdout);

  // Truncate diff if it's too large (to avoid token limits)
  const MAX_DIFF_SIZE: usize = 50000;
  if full_diff.len() > MAX_DIFF_SIZE {
    Ok(format!(
      "{}...\n\n(Diff truncated - {} chars total)",
      &full_diff[..MAX_DIFF_SIZE],
      full_diff.len()
    ))
  } else {
    Ok(full_diff.to_string())
  }
}

/// Edit PR body.
pub fn gh_edit_body(pr_id: Option<&str>, body: &str) -> Result<()> {
  let mut args = vec!["pr", "edit"];
  extend_with_pr_id(&mut args, pr_id);
  args.extend(["--body", body]);

  run_command("gh", &args)?;
  Ok(())
}

fn extend_with_pr_id<'a>(args: &mut Vec<&'a str>, pr_id: Option<&'a str>) {
  if let Some(id) = pr_id {
    args.push(id);
  }
}

/// Create a new pull request.
pub fn create_pr(title: &str, body: &str, base: &str, head: &str) -> Result<String> {
  let output = Command::new("gh")
    .args(["pr", "create", "--title", title, "--body", body, "--base", base, "--head", head])
    .output()?;

  if !output.status.success() {
    return Err(format!("Failed to create PR: {}", String::from_utf8_lossy(&output.stderr)).into());
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Create a GitHub release.
pub fn create_github_release(version: &str, notes: &str, prerelease: bool) -> Result<()> {
  let tag = format!("v{}", version);
  let mut args = vec!["release", "create", &tag, "--title", &tag, "--notes", notes];

  if prerelease {
    args.push("--prerelease");
  }

  run_command("gh", &args)?;
  Ok(())
}

/// Comment on a PR.
pub fn comment_on_pr(pr_number: &str, comment: &str) -> Result<()> {
  run_command("gh", &["pr", "comment", pr_number, "--body", comment])?;
  Ok(())
}

/// Get merged PRs since a version.
pub fn get_merged_prs_since(ver: &Version) -> Result<Vec<crate::types::PR>> {
  let date = get_tag_date(ver).ok_or(format!("Tag for {} not found", ver))?;
  let out = Command::new("gh")
    .args([
      "pr",
      "list",
      "--state",
      "merged",
      "--base",
      "master",
      "--limit",
      "500",
      "--json",
      "number,title,body,author,mergedAt",
    ])
    .output()?;
  if !out.status.success() {
    return Err(format!("gh failed: {}", String::from_utf8_lossy(&out.stderr)).into());
  }

  let prs: Vec<crate::types::PR> = serde_json::from_slice(&out.stdout)?;
  Ok(
    prs
      .into_iter()
      .filter(|p| p.merged_at.as_ref().is_some_and(|d| d > &date))
      .collect(),
  )
}

/// Get the date of a version tag.
pub fn get_tag_date(ver: &Version) -> Option<String> {
  let tags = [format!("v{}", ver), format!("ribir-v{}", ver), ver.to_string()];
  for tag in tags {
    if let Ok(o) = Command::new("git")
      .args(["log", "-1", "--format=%aI", &tag])
      .output()
    {
      if o.status.success() {
        return Some(String::from_utf8_lossy(&o.stdout).trim().to_string());
      }
    }
  }
  None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_json() {
    let response = r#"Here is the JSON:
{"highlights": [{"emoji": "⚡", "description": "test"}]}
That's all."#;
    let json = extract_json(response).unwrap();
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
    assert!(json.contains("highlights"));
  }
}

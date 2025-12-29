//! Common utility functions.

use std::process::{Command, Output};

use crate::types::Result;

/// Run a command and return output if successful.
pub fn run_command(cmd: &str, args: &[&str]) -> Result<Output> {
  let output = Command::new(cmd).args(args).output()?;
  if !output.status.success() {
    return Err(
      format!("{} {:?} failed: {}", cmd, args, String::from_utf8_lossy(&output.stderr)).into(),
    );
  }
  Ok(output)
}

/// Run a git command.
pub fn run_git(args: &[&str]) -> Result<Output> { run_command("git", args) }

/// Get current git branch.
pub fn get_current_branch() -> Result<String> {
  // Allow override for testing
  if let Ok(branch) = std::env::var("CHANGELOG_BRANCH") {
    return Ok(branch);
  }

  let output = Command::new("git")
    .args(["rev-parse", "--abbrev-ref", "HEAD"])
    .output()?;

  if !output.status.success() {
    return Err("Failed to get current git branch".into());
  }

  Ok(
    String::from_utf8_lossy(&output.stdout)
      .trim()
      .to_string(),
  )
}

/// Get changelog path based on current branch.
pub fn get_changelog_path() -> Result<String> {
  let branch = get_current_branch()?;

  if branch == "master" || branch == "main" {
    return Ok("CHANGELOG.md".to_string());
  }

  // Parse release branch: release-0.5.x or release-0.5.0
  if let Some(version_part) = branch.strip_prefix("release-") {
    let parts: Vec<&str> = version_part.split('.').collect();
    if parts.len() >= 2 {
      return Ok(format!("changelogs/CHANGELOG-{}.{}.md", parts[0], parts[1]));
    }
  }

  // Default to CHANGELOG.md for feature branches, etc.
  Ok("CHANGELOG.md".to_string())
}

/// Check if a git branch exists.
pub fn branch_exists(branch: &str) -> Result<bool> {
  let output = Command::new("git")
    .args(["rev-parse", "--verify", branch])
    .output()?;
  Ok(output.status.success())
}

/// Create a release branch.
pub fn create_release_branch(version: &semver::Version) -> Result<()> {
  let branch_name = format!("release-{}.{}.x", version.major, version.minor);
  run_git(&["checkout", "-b", &branch_name])?;
  run_git(&["push", "-u", "origin", &branch_name])?;
  Ok(())
}

/// Get latest git tags.
pub fn get_latest_tags(count: usize) -> Result<Vec<String>> {
  let output = Command::new("git")
    .args(["tag", "--sort=-v:refname", "-l", "v*", &format!("--count={}", count)])
    .output()?;

  if !output.status.success() {
    return Ok(vec![]);
  }

  Ok(
    String::from_utf8_lossy(&output.stdout)
      .lines()
      .take(count)
      .map(String::from)
      .collect(),
  )
}

/// Get today's date in YYYY-MM-DD format.
pub fn today() -> String {
  String::from_utf8_lossy(
    &Command::new("date")
      .arg("+%Y-%m-%d")
      .output()
      .unwrap()
      .stdout,
  )
  .trim()
  .to_string()
}

/// Sanitize markdown content by removing potentially dangerous elements.
pub fn sanitize_markdown(s: &str) -> String {
  s.lines()
    .filter(|line| {
      let lower = line.to_lowercase();
      !lower.contains("<script") && !lower.contains("<iframe") && !lower.contains("javascript:")
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// Truncate a string to max length with a suffix.
pub fn truncate(s: &mut String, max_len: usize, suffix: &str) {
  if s.len() > max_len {
    *s = s.chars().take(max_len).collect();
    s.push_str(suffix);
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Mutex;

  use super::*;

  // Mutex to ensure tests that modify CHANGELOG_BRANCH run sequentially
  static ENV_MUTEX: Mutex<()> = Mutex::new(());

  fn with_branch<F, R>(branch: &str, f: F) -> R
  where
    F: FnOnce() -> R,
  {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
      std::env::set_var("CHANGELOG_BRANCH", branch);
    }
    let result = f();
    unsafe {
      std::env::remove_var("CHANGELOG_BRANCH");
    }
    result
  }

  #[test]
  fn test_sanitize_markdown() {
    let input = "Normal\n<script>alert('xss')</script>\nOK";
    let result = sanitize_markdown(input);
    assert!(!result.contains("<script"));
    assert!(result.contains("Normal"));
    assert!(result.contains("OK"));
  }

  #[test]
  fn test_truncate() {
    let mut s = "hello world".to_string();
    truncate(&mut s, 5, "...");
    assert_eq!(s, "hello...");
  }

  #[test]
  fn test_changelog_path_master() {
    with_branch("master", || {
      assert_eq!(get_changelog_path().unwrap(), "CHANGELOG.md");
    });
  }

  #[test]
  fn test_changelog_path_main() {
    with_branch("main", || {
      assert_eq!(get_changelog_path().unwrap(), "CHANGELOG.md");
    });
  }

  #[test]
  fn test_changelog_path_release_branch_x() {
    with_branch("release-0.5.x", || {
      assert_eq!(get_changelog_path().unwrap(), "changelogs/CHANGELOG-0.5.md");
    });
  }

  #[test]
  fn test_changelog_path_release_branch_specific() {
    with_branch("release-0.5.0", || {
      assert_eq!(get_changelog_path().unwrap(), "changelogs/CHANGELOG-0.5.md");
    });
  }

  #[test]
  fn test_changelog_path_feature_branch() {
    with_branch("feat/new-feature", || {
      assert_eq!(get_changelog_path().unwrap(), "CHANGELOG.md");
    });
  }
}

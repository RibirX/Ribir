//! External tool integrations (Gemini AI and GitHub CLI).

use std::{
  io::Write as IoWrite,
  process::{Command, Stdio},
};

use semver::Version;

use crate::{types::Result, utils::run_command};

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

  let output = child
    .wait_with_output()
    .map_err(|e| e.to_string())?;

  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).into())
  } else {
    Err(String::from_utf8_lossy(&output.stderr).into())
  }
}

pub fn extract_json(s: &str) -> Option<String> {
  let start = s.find('{')?;
  let end = s.rfind('}')?;
  Some(s[start..=end].to_string())
}

pub fn gh_json<T: for<'de> serde::Deserialize<'de>>(
  pr_id: Option<&str>, fields: &str,
) -> Result<T> {
  let mut args = vec!["pr", "view"];
  extend_with_pr_id(&mut args, pr_id);
  args.extend(["--json", fields]);

  let output = run_command("gh", &args)?;
  Ok(serde_json::from_slice(&output.stdout)?)
}

pub fn gh_diff(pr_id: Option<&str>) -> Result<String> {
  let mut args = vec!["pr", "diff"];
  extend_with_pr_id(&mut args, pr_id);
  args.push("--patch");

  let output = run_command("gh", &args)?;
  let full_diff = String::from_utf8_lossy(&output.stdout);

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

pub fn gh_get_pr_details() -> Result<(u32, String)> {
  #[derive(serde::Deserialize)]
  struct PrDetails {
    number: u32,
    body: String,
  }

  let pr: PrDetails = gh_json(None, "number,body")?;
  Ok((pr.number, pr.body))
}

pub fn gh_get_pr_body() -> Result<String> {
  let (_, body) = gh_get_pr_details()?;
  Ok(body)
}

pub fn create_pr(
  title: &str, body: &str, base: &str, head: &str, labels: Option<&[&str]>,
) -> Result<String> {
  let mut args =
    vec!["pr", "create", "--title", title, "--body", body, "--base", base, "--head", head];

  if let Some(lbls) = labels {
    for label in lbls {
      args.extend(["--label", label]);
    }
  }

  let out = run_command("gh", &args)?;
  Ok(
    String::from_utf8_lossy(&out.stdout)
      .trim()
      .to_string(),
  )
}

pub fn merge_pr(pr_number: &str) -> Result<()> {
  run_command("gh", &["pr", "merge", pr_number, "--merge", "--delete-branch"])?;
  Ok(())
}

pub fn add_label(id: &str, label: &str) -> Result<()> {
  run_command("gh", &["issue", "edit", id, "--add-label", label])?;
  Ok(())
}

pub fn remove_label(id: &str, label: &str) -> Result<()> {
  run_command("gh", &["issue", "edit", id, "--remove-label", label])?;
  Ok(())
}

pub fn create_github_release(version: &str, notes: &str, prerelease: bool) -> Result<()> {
  let tag = format!("v{}", version);
  let mut args = vec!["release", "create", &tag, "--title", &tag, "--notes", notes];

  if prerelease {
    args.push("--prerelease");
  }

  run_command("gh", &args)?;
  Ok(())
}

pub fn comment_on_pr(pr_number: &str, comment: &str) -> Result<()> {
  run_command("gh", &["pr", "comment", pr_number, "--body", comment])?;
  Ok(())
}

pub fn add_reaction(comment_id: u64, reaction: &str) -> Result<()> {
  let repo = get_origin_repo()?;
  run_command(
    "gh",
    &[
      "api",
      "--method",
      "POST",
      &format!("/repos/{repo}/issues/comments/{comment_id}/reactions"),
      "-f",
      &format!("content={reaction}"),
    ],
  )?;
  Ok(())
}

pub fn get_merged_prs_since(ver: &Version, repo: Option<&str>) -> Result<Vec<crate::types::PR>> {
  let tag_commit = get_tag_commit(ver).ok_or(format!("Tag for {} not found", ver))?;
  eprintln!("📌 Tag commit for {}: {}", ver, &tag_commit[..8]);

  let repo = match repo {
    Some(repo) => repo.to_string(),
    None => get_origin_repo()?,
  };
  eprintln!("📌 Querying PRs from: {}", repo);

  let out = run_command(
    "gh",
    &[
      "pr",
      "list",
      "--repo",
      &repo,
      "--state",
      "merged",
      "--base",
      "master",
      "--limit",
      "100",
      "--json",
      "number,title,body,author,mergeCommit",
    ],
  )?;

  #[derive(serde::Deserialize)]
  struct PRWithCommit {
    number: u32,
    title: String,
    body: Option<String>,
    author: crate::types::Author,
    #[serde(rename = "mergeCommit")]
    merge_commit: Option<MergeCommit>,
  }

  #[derive(serde::Deserialize)]
  struct MergeCommit {
    oid: String,
  }

  let prs: Vec<PRWithCommit> = serde_json::from_slice(&out.stdout)?;
  let result = prs
    .into_iter()
    .filter(|pr| {
      let Some(ref mc) = pr.merge_commit else { return false };
      !is_ancestor(&mc.oid, &tag_commit)
    })
    .map(|pr| crate::types::PR {
      number: pr.number,
      title: pr.title,
      body: pr.body,
      author: pr.author,
    })
    .collect::<Vec<_>>();

  eprintln!("🔍 Found {} PRs merged after tag", result.len());
  Ok(result)
}

fn is_ancestor(commit: &str, ancestor: &str) -> bool {
  Command::new("git")
    .args(["merge-base", "--is-ancestor", commit, ancestor])
    .output()
    .map(|o| o.status.success())
    .unwrap_or(false)
}

fn get_tag_commit(ver: &Version) -> Option<String> {
  let tags = [format!("v{}", ver), format!("ribir-v{}", ver), ver.to_string()];
  for tag in tags {
    if let Ok(o) = Command::new("git")
      .args(["rev-parse", &tag])
      .output()
      && o.status.success()
    {
      return Some(
        String::from_utf8_lossy(&o.stdout)
          .trim()
          .to_string(),
      );
    }
  }
  None
}

pub fn get_origin_repo() -> Result<String> {
  get_repo_from_remote("upstream").or_else(|_| get_repo_from_remote("origin"))
}

fn get_repo_from_remote(remote: &str) -> Result<String> {
  let out = run_command("git", &["remote", "get-url", remote])?;
  let url = String::from_utf8_lossy(&out.stdout)
    .trim()
    .to_string();
  Ok(parse_repo_from_url(&url))
}

pub fn parse_repo_from_url(url: &str) -> String {
  url
    .trim_end_matches(".git")
    .rsplit(['/', ':'])
    .take(2)
    .collect::<Vec<_>>()
    .into_iter()
    .rev()
    .collect::<Vec<_>>()
    .join("/")
}

pub fn check_permission(user: &str, _author: &str, repo: &str) -> Result<bool> {
  let out = run_command(
    "gh",
    &["api", &format!("/repos/{repo}/collaborators/{user}/permission"), "--jq", ".permission"],
  )?;

  let perm = String::from_utf8_lossy(&out.stdout)
    .trim()
    .to_string();
  Ok(perm == "write" || perm == "admin")
}

pub fn get_pr_info(pr_number: u32, repo: &str) -> Result<(String, String)> {
  let out = run_command(
    "gh",
    &["pr", "view", &pr_number.to_string(), "--repo", repo, "--json", "baseRefName,headRefName"],
  )?;

  #[derive(serde::Deserialize)]
  struct PrInfo {
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
  }

  let info: PrInfo = serde_json::from_slice(&out.stdout)?;
  Ok((info.base_ref_name, info.head_ref_name))
}

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

  #[test]
  fn test_get_repo_from_remote_parsing() {
    let url = "git@github.com:RibirX/Ribir.git";
    let repo = parse_repo_from_url(url);
    assert_eq!(repo, "RibirX/Ribir");
  }

  #[test]
  fn test_get_origin_repo_parsing_prefers_upstream() {
    let upstream = "git@github.com:RibirX/Ribir.git";
    let origin = "git@github.com:someone/Ribir.git";

    assert_eq!(parse_repo_from_url(upstream), "RibirX/Ribir");
    assert_eq!(parse_repo_from_url(origin), "someone/Ribir");
  }
}

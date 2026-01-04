//! Release command implementations.

use std::{fs, process::{Command, Stdio}};

use comrak::Arena;
use semver::Version;

use crate::{
  changelog::{
    ChangelogContext, extract_version_section, find_rc_highlights, find_rc_versions,
    format_highlights, insert_highlights, parse_latest_version, replace_version_header,
  },
  external::{
    call_gemini_with_fallback, comment_on_pr, create_github_release, create_pr, extract_json,
  },
  types::{Config, Highlight, HighlightsResponse, ReleaseCmd, Result},
  utils::{
    branch_exists, create_release_branch, get_changelog_path, get_current_branch, get_latest_tags,
    run_git,
  },
};

const HIGHLIGHTS_PROMPT: &str = r#"Analyze these changelog entries and select 3-5 highlights for a release announcement.

## Changelog Entries

{changelog_entries}

## Selection Criteria

1. **Impact** - Prioritize user-facing changes over internal refactors
2. **Newsworthy** - Features and performance improvements over minor fixes
3. **Diversity** - Cover different areas (widgets, core, performance, etc.)
4. **Clarity** - Changes that are easy to understand and explain

## Output Requirements

Generate 3-5 highlights (no more, no less) with:
- **Emoji** - Match the change type: ✨ (new), 🎨 (features), ⚡ (perf), 🐛 (fix), 📚 (docs), 💥 (breaking), 🔧 (internal)
- **Description** - Under 60 characters, user-friendly, active voice
  - Good: "50% faster WASM rendering"
  - Bad: "WASM rendering performance was improved by 50%"

## Output Format

Return ONLY valid JSON:
{"highlights": [{"emoji": "⚡", "description": "50% faster WASM rendering"}, ...]}

Example output:
{"highlights": [
  {"emoji": "⚡", "description": "50% faster WASM rendering"},
  {"emoji": "🎨", "description": "Dark mode support for all widgets"},
  {"emoji": "🔧", "description": "Plugin system for extensibility"},
  {"emoji": "🐛", "description": "Fixed memory leak in event handling"}
]}"#;

/// Execute release command.
pub fn cmd_release(config: &Config, cmd: &ReleaseCmd) -> Result<()> {
  match cmd {
    ReleaseCmd::Next { level } => cmd_release_next(config, level),
    ReleaseCmd::Prepare { version } => cmd_release_prepare(config, version),
    ReleaseCmd::Publish { pr_id } => cmd_release_publish(config, pr_id.as_deref()),
    ReleaseCmd::Promote { version } => cmd_release_promote(config, version),
    ReleaseCmd::Verify => cmd_release_verify(config),
    ReleaseCmd::Highlights { context } => cmd_release_highlights(config, context.as_deref()),
    ReleaseCmd::SocialCard => cmd_release_social_card(config),
  }
}

/// Execute full release at the specified level.
/// Levels: alpha, rc, patch, minor, major
pub fn cmd_release_next(config: &Config, level: &str) -> Result<()> {
  // 1. Validate level
  validate_release_level(level)?;

  println!("🚀 Starting {} release...", level);

  // 2. Get next version from cargo release dry-run
  let version = get_next_version(level)?;
  println!("📦 Next version: {}", version);

  // 3. Collect changelog entries
  println!("📋 Collecting changelog entries...");
  if !config.dry_run {
    run_log_collect(&version)?;
    run_git(&["add", "CHANGELOG.md"])?;
  } else {
    println!("   Would collect changelog for {}", version);
  }

  // 4. Run cargo release (version bump, commit, tag, push, optional publish)
  println!("🔧 Running cargo release...");
  if !config.dry_run {
    run_cargo_release(level)?;
  } else {
    println!("   Would run: cargo release {} --execute --no-confirm", level);
  }

  // 5. Create GitHub Release
  let is_prerelease = level == "alpha" || level == "rc";
  println!("🎉 Creating GitHub Release (prerelease: {})...", is_prerelease);
  if !config.dry_run {
    let changelog = fs::read_to_string("CHANGELOG.md")?;
    let notes = extract_version_section(&changelog, &version)
      .ok_or_else(|| format!("Release notes not found for version {}", version))?;
    create_github_release(&version, &notes, is_prerelease)?;
  } else {
    println!("   Would create GitHub Release v{}", version);
  }

  if config.dry_run {
    println!("\n💡 This is a dry-run. Use --execute to apply changes.");
  } else {
    println!("\n✅ Release {} complete!", version);
  }
  Ok(())
}

fn validate_release_level(level: &str) -> Result<()> {
  match level {
    "alpha" | "rc" | "patch" | "minor" | "major" => Ok(()),
    _ => Err(format!(
      "Invalid level: '{}'. Use: alpha, rc, patch, minor, major",
      level
    )
    .into()),
  }
}

fn get_next_version(level: &str) -> Result<String> {
  let output = Command::new("cargo")
    .args(["release", level, "--dry-run"])
    .stderr(Stdio::piped())
    .stdout(Stdio::piped())
    .output()?;

  let combined = format!(
    "{}{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );

  // Find "Upgrading ... to X.Y.Z" pattern and extract version using semver
  // Look for lines containing "Upgrading" and extract the last valid semver
  for line in combined.lines() {
    if line.contains("Upgrading") && line.contains(" to ") {
      // Split by " to " and take the second part
      if let Some(after_to) = line.split(" to ").nth(1) {
        // The version is the first word after "to"
        let version_str = after_to.split_whitespace().next().unwrap_or("");
        // Validate with semver
        if Version::parse(version_str).is_ok() {
          return Ok(version_str.to_string());
        }
      }
    }
  }

  Err(
    format!(
      "Could not parse version from cargo release output:\n{}",
      &combined[..combined.len().min(500)]
    )
    .into(),
  )
}

fn run_cargo_release(level: &str) -> Result<()> {
  let status = Command::new("cargo")
    .args(["release", level, "--execute", "--no-confirm"])
    .status()?;

  if !status.success() {
    return Err(format!("cargo release failed with exit code: {:?}", status.code()).into());
  }
  Ok(())
}

fn run_log_collect(version: &str) -> Result<()> {
  use crate::commands::cmd_collect;

  // Create a config that writes (not dry-run)
  let collect_config = Config {
    command: crate::types::Cmd::Verify, // Dummy, not used
    dry_run: false,
  };

  cmd_collect(&collect_config, version, true)
}

/// Prepare RC release.
pub fn cmd_release_prepare(config: &Config, version_str: &str) -> Result<()> {
  let version = Version::parse(version_str)?;
  let rc_version = format!("{}.{}.{}-rc.1", version.major, version.minor, version.patch);

  println!("🚀 Preparing RC for version {}", version_str);

  // Step 1: Archive changelog if needed
  let needs_archive = should_archive_changelog(&version)?;
  if needs_archive {
    println!(
      "📦 Archiving CHANGELOG.md to changelogs/CHANGELOG-{}.{}.md",
      version.major, version.minor
    );
    if !config.dry_run {
      archive_changelog(&version)?;
    }
  }

  // Step 2: Merge alpha changelog entries
  println!("🔀 Merging alpha changelog entries for {}...", rc_version);
  if !config.dry_run {
    run_changelog_merge(&rc_version)?;
  }

  // Step 3: Generate and insert AI highlights
  let (highlights, updated_changelog) = prepare_highlights(&rc_version)?;
  println!("📝 Generated {} highlights", highlights.len());

  if config.dry_run {
    println!("\n📝 Preview of highlights:\n{}", format_highlights(&highlights));
  } else {
    let changelog_path = get_changelog_path()?;
    fs::write(&changelog_path, &updated_changelog)?;
    println!("✅ Updated {}", changelog_path);
  }

  // Step 4: Create release branch if needed
  let branch_name = format!("release-{}.{}.x", version.major, version.minor);
  if !branch_exists(&branch_name)? {
    println!("🌿 Creating release branch: {}", branch_name);
    if !config.dry_run {
      create_release_branch(&version)?;
    }
  }

  // Step 5: Commit and create PR
  if config.dry_run {
    println!("\n💡 Run without --dry-run to apply changes.");
  } else {
    commit_and_create_release_pr(&rc_version, &branch_name, needs_archive)?;
  }

  Ok(())
}

/// Publish GitHub release.
pub fn cmd_release_publish(config: &Config, pr_number: Option<&str>) -> Result<()> {
  let version = get_version_from_context()?;
  let ver = Version::parse(&version)?;
  let branch_name = format!("release-{}.{}.x", ver.major, ver.minor);

  println!("📦 Publishing release {}...", version);

  if !branch_exists(&branch_name)? {
    println!("🌿 Creating release branch: {}", branch_name);
    if !config.dry_run {
      create_release_branch(&ver)?;
    }
  }

  let changelog_path = get_changelog_path()?;
  let changelog = fs::read_to_string(&changelog_path)?;
  let release_notes = extract_version_section(&changelog, &version)
    .ok_or_else(|| format!("Release notes not found for version {}", version))?;

  let is_prerelease = version.contains("-rc") || version.contains("-alpha");
  println!("🎉 Creating GitHub Release (prerelease={})...", is_prerelease);
  if !config.dry_run {
    create_github_release(&version, &release_notes, is_prerelease)?;
  }

  if let Some(pr) = pr_number {
    let comment = format!(
      "🎉 Release **v{}** has been published!\n\n[View Release](https://github.com/RibirX/Ribir/releases/tag/v{})",
      version, version
    );
    if !config.dry_run {
      comment_on_pr(pr, &comment)?;
    }
    println!("💬 Commented on PR #{}", pr);
  }

  println!("✅ Release v{} published successfully!", version);
  Ok(())
}

/// Promote RC to stable.
pub fn cmd_release_promote(config: &Config, version_str: &str) -> Result<()> {
  let version = Version::parse(version_str)?;
  let rc1_version = format!("{}-rc.1", version_str);
  let changelog_path = get_changelog_path()?;

  println!("🚀 Promoting {} to stable...", version_str);

  let changelog = fs::read_to_string(&changelog_path)?;
  let has_highlights = find_rc_highlights(&changelog, &rc1_version).is_some();
  let rc_versions = find_rc_versions(&changelog, &version);

  if has_highlights {
    println!("📋 Reusing highlights from RC.1");
  } else {
    eprintln!("⚠️  No highlights found in RC.1, proceeding without highlights");
  }

  if rc_versions.len() > 1 {
    println!("🔀 Found {} RC versions, merging bug fix entries...", rc_versions.len());
    if !config.dry_run {
      run_changelog_merge(version_str)?;
    }
  }

  let changelog = fs::read_to_string(&changelog_path)?;
  let updated_changelog = replace_version_header(&changelog, &rc1_version, version_str);

  if !config.dry_run {
    fs::write(&changelog_path, &updated_changelog)?;
    run_git(&["add", &changelog_path])?;
    println!("✅ Updated CHANGELOG.md with stable version");
  }

  // Run cargo release to bump version, commit, tag, and optionally publish to crates.io
  println!("📦 Running cargo release {}...", version_str);
  if !config.dry_run {
    let status = Command::new("cargo")
      .args(["release", version_str, "--execute", "--no-confirm"])
      .status()?;

    if !status.success() {
      return Err(format!("cargo release failed with exit code: {:?}", status.code()).into());
    }
  } else {
    println!("   Would run: cargo release {} --execute --no-confirm", version_str);
  }

  let release_notes = extract_version_section(&updated_changelog, version_str)
    .ok_or_else(|| format!("Release notes not found for version {}", version_str))?;

  println!("🎉 Creating stable GitHub Release...");
  if !config.dry_run {
    create_github_release(version_str, &release_notes, false)?;
  }

  if config.dry_run {
    println!("\n💡 This is a dry-run. Use --execute to apply changes.");
  } else {
    println!("\n✅ Stable release {} published!", version_str);
  }
  Ok(())
}

/// Verify release state.
pub fn cmd_release_verify(_config: &Config) -> Result<()> {
  println!("🔍 Verifying release state...\n");

  let branch = get_current_branch()?;
  println!("📍 Current branch: {}", branch);

  let tags = get_latest_tags(5)?;
  println!("\n🏷️  Recent tags:");
  for tag in &tags {
    println!("   {}", tag);
  }

  let changelog_path = get_changelog_path()?;
  println!("\n📄 Changelog path: {}", changelog_path);

  if let Ok(changelog) = fs::read_to_string(&changelog_path) {
    if let Some(latest) = parse_latest_version(&changelog) {
      println!("   Latest version: {}", latest);
    }
  }

  println!("\n🔧 Required tools:");
  let tools = [("gh", "GitHub CLI"), ("gemini", "Gemini CLI")];
  for (cmd, name) in tools {
    let status = if Command::new(cmd)
      .arg("--version")
      .output()
      .is_ok()
    {
      "✅"
    } else {
      "❌"
    };
    println!("   {} {}", status, name);
  }

  println!("\n✅ Verification complete");
  Ok(())
}

// Helper functions

fn should_archive_changelog(version: &Version) -> Result<bool> {
  let changelog = fs::read_to_string("CHANGELOG.md").unwrap_or_default();
  let latest = parse_latest_version(&changelog);

  Ok(match latest {
    Some(latest_ver) => {
      let latest_parsed = Version::parse(&latest_ver).ok();
      latest_parsed.is_some_and(|v| version.minor != v.minor || version.major != v.major)
    }
    None => false,
  })
}

fn archive_changelog(version: &Version) -> Result<()> {
  let source = "CHANGELOG.md";
  let dest = format!("changelogs/CHANGELOG-{}.{}.md", version.major, version.minor);

  fs::create_dir_all("changelogs")?;
  fs::copy(source, &dest)?;

  let prev_minor = if version.minor > 0 { version.minor - 1 } else { 0 };
  let new_content = format!(
    "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\nFor \
     older versions:\n- [{}.{}.x changelog](changelogs/CHANGELOG-{}.{}.md)\n\n<!-- next-header \
     -->\n",
    version.major, prev_minor, version.major, prev_minor
  );

  fs::write(source, new_content)?;
  Ok(())
}

fn prepare_highlights(version: &str) -> Result<(Vec<Highlight>, String)> {
  println!("✨ Generating highlights with AI...");

  let changelog_path = get_changelog_path()?;
  let changelog = fs::read_to_string(&changelog_path)?;
  let entries = extract_version_section(&changelog, version)
    .ok_or_else(|| format!("No entries found for version {}", version))?;

  let highlights = generate_highlights(&entries)?;
  let updated_changelog = insert_highlights(&changelog, version, &highlights)?;

  Ok((highlights, updated_changelog))
}

fn generate_highlights(changelog_entries: &str) -> Result<Vec<Highlight>> {
  let prompt = HIGHLIGHTS_PROMPT.replace("{changelog_entries}", changelog_entries);
  let response = call_gemini_with_fallback(&prompt)?;

  let json_str = extract_json(&response).ok_or("No JSON found in AI response")?;
  let parsed: HighlightsResponse = serde_json::from_str(&json_str)
    .map_err(|e| format!("Invalid JSON from AI: {e}\nRaw: {response}"))?;

  validate_highlights(&parsed.highlights)?;
  Ok(parsed.highlights)
}

fn validate_highlights(highlights: &[Highlight]) -> Result<()> {
  if highlights.len() < 3 || highlights.len() > 5 {
    return Err(
      format!("Expected 3-5 highlights, got {}. Please regenerate.", highlights.len()).into(),
    );
  }

  for h in highlights {
    if h.description.len() > 60 {
      eprintln!("⚠️  Highlight too long ({}): {}", h.description.len(), h.description);
    }
  }

  Ok(())
}

fn commit_and_create_release_pr(
  rc_version: &str, branch_name: &str, needs_archive: bool,
) -> Result<()> {
  let changelog_path = get_changelog_path()?;

  run_git(&["add", &changelog_path])?;
  if needs_archive {
    let parts: Vec<&str> = rc_version.split('.').collect();
    if parts.len() >= 2 {
      let archive_path = format!("changelogs/CHANGELOG-{}.{}.md", parts[0], parts[1]);
      run_git(&["add", &archive_path])?;
    }
  }

  run_git(&[
    "commit",
    "-m",
    &format!(
      "chore(release): prepare {}\n\n🤖 Generated with ribir-bot\n\nCo-Authored-By: Claude \
       <noreply@anthropic.com>",
      rc_version
    ),
  ])?;

  let pr_title = format!("Release {} Preparation", rc_version);
  let pr_body = format!(
    "## Release Preparation for {}\n\nThis PR prepares the release materials:\n\n- Merged \
     changelog from all alpha versions\n- AI-generated highlights section\n\n**Review \
     Checklist:**\n- [ ] Verify highlights are accurate and well-written\n- [ ] Check all \
     important PRs are included\n- [ ] Confirm version and date are correct\n\n---\n🤖 Generated \
     by ribir-bot",
    rc_version
  );

  let pr_url = create_pr(&pr_title, &pr_body, "master", branch_name)?;
  println!("✅ Created PR: {}", pr_url);

  Ok(())
}

fn get_version_from_context() -> Result<String> {
  // First try: get version from latest git tag (most reliable after cargo release)
  if let Ok(output) = std::process::Command::new("git")
    .args(["describe", "--tags", "--abbrev=0"])
    .output()
  {
    if output.status.success() {
      let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
      if let Some(version) = tag.strip_prefix('v') {
        return Ok(version.to_string());
      }
    }
  }

  // Fallback: parse from CHANGELOG.md
  let changelog = fs::read_to_string("CHANGELOG.md")?;
  parse_latest_version(&changelog).ok_or("Could not determine version from context".into())
}

fn run_changelog_merge(version: &str) -> Result<()> {
  let arena = Arena::new();
  let ctx = ChangelogContext::load(&arena)?;
  let target_ver = Version::parse(version)?;

  ctx.merge_prereleases(&target_ver)?;
  ctx.save(false)
}

/// Regenerate highlights section in CHANGELOG.md.
pub fn cmd_release_highlights(config: &Config, context: Option<&str>) -> Result<()> {
  println!("🔄 Regenerating highlights in CHANGELOG.md...");

  // 1. Get version from changelog (latest version in file)
  let changelog_path = get_changelog_path()?;
  let changelog = fs::read_to_string(&changelog_path)?;
  let version =
    parse_latest_version(&changelog).ok_or("Could not find version in CHANGELOG.md")?;

  println!("📌 Found version: {}", version);

  // 2. Extract entries for the version
  let entries = extract_version_section(&changelog, &version)
    .ok_or_else(|| format!("No entries found for version {}", version))?;

  // 3. Generate highlights with AI
  let highlights = generate_highlights_with_context(&entries, context)?;
  println!("📝 Generated {} highlights", highlights.len());

  // 4. Replace highlights section
  let updated = insert_highlights(&changelog, &version, &highlights)?;

  if config.dry_run {
    println!("\n📝 Preview:\n{}", format_highlights(&highlights));
    println!("\n💡 Run without --dry-run to apply changes.");
  } else {
    fs::write(&changelog_path, &updated)?;
    println!("✅ Updated {}", changelog_path);
  }

  Ok(())
}

/// Stub for social card generation.
pub fn cmd_release_social_card(_config: &Config) -> Result<()> {
  println!("⚠️  Social card generation is not yet implemented.");
  println!("📌 This feature is planned for future releases.");
  println!("\nSee: dev-docs/release-system/03-social-card-generation.md");
  Ok(())
}

fn generate_highlights_with_context(entries: &str, context: Option<&str>) -> Result<Vec<Highlight>> {
  println!("✨ Generating highlights with AI...");

  let mut prompt = HIGHLIGHTS_PROMPT.replace("{changelog_entries}", entries);
  if let Some(ctx) = context {
    prompt = format!(
      "ADDITIONAL CONTEXT FROM USER:\n{}\n\nPlease consider this context when selecting and \
       writing highlights.\n\n{}",
      ctx, prompt
    );
  }

  let response = call_gemini_with_fallback(&prompt)?;
  let json_str = extract_json(&response).ok_or("No JSON found in AI response")?;
  let parsed: HighlightsResponse =
    serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON from AI: {e}\nRaw: {response}"))?;

  validate_highlights(&parsed.highlights)?;
  Ok(parsed.highlights)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_highlights_response() {
    let json = r#"{"highlights": [
      {"emoji": "⚡", "description": "Faster rendering"},
      {"emoji": "🐛", "description": "Fixed memory leak"},
      {"emoji": "🎨", "description": "New widgets"}
    ]}"#;
    let response: HighlightsResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.highlights.len(), 3);
    assert_eq!(response.highlights[0].emoji, "⚡");
  }

  #[test]
  fn test_highlights_validation_count() {
    let too_few = vec![Highlight { emoji: "⚡".into(), description: "x".into() }];
    assert!(validate_highlights(&too_few).is_err());

    let valid = vec![
      Highlight { emoji: "⚡".into(), description: "x".into() },
      Highlight { emoji: "🎨".into(), description: "y".into() },
      Highlight { emoji: "🐛".into(), description: "z".into() },
    ];
    assert!(validate_highlights(&valid).is_ok());

    let too_many = vec![
      Highlight { emoji: "1".into(), description: "a".into() },
      Highlight { emoji: "2".into(), description: "b".into() },
      Highlight { emoji: "3".into(), description: "c".into() },
      Highlight { emoji: "4".into(), description: "d".into() },
      Highlight { emoji: "5".into(), description: "e".into() },
      Highlight { emoji: "6".into(), description: "f".into() },
    ];
    assert!(validate_highlights(&too_many).is_err());
  }
}

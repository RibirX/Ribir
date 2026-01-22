//! Release command implementations.

use std::{fs, process::Command};

use comrak::Arena;
use semver::Version;

use crate::{changelog::*, external::*, types::*, utils::*};

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

// ============================================================================
// Public API
// ============================================================================

/// Execute release command.
pub fn cmd_release(config: &Config, cmd: &ReleaseCmd) -> Result<()> {
  match cmd {
    ReleaseCmd::Next { level, .. } => cmd_release_next(config, *level),
    ReleaseCmd::EnterRc { .. } => cmd_release_enter_rc(config),
    ReleaseCmd::Publish { pr_id } => cmd_release_publish(config, pr_id.as_deref()),
    ReleaseCmd::Stable { version, .. } => cmd_release_stable(config, version.as_deref()),
    ReleaseCmd::Verify => cmd_release_verify(),
    ReleaseCmd::Highlights { context } => cmd_release_highlights(config, context.as_deref()),
    ReleaseCmd::SocialCard => cmd_release_social_card(config),
  }
}

/// Execute full release at the specified level.
pub fn cmd_release_next(config: &Config, level: ReleaseLevel) -> Result<()> {
  let level_str = level.as_str();
  println!("🚀 Starting {} release...", level_str);

  let version = get_next_version(level_str)?;
  println!("📦 Next version: {}", version);

  println!("📋 Collecting changelog entries...");
  let changelog_entries = collect_changelog_entries(&version, config.dry_run)?;

  // Commit changelog first, then cargo publish will --amend it
  let has_changelog_commit = if !config.dry_run {
    run_git(&["add", "CHANGELOG.md"])?;
    run_git(&[
      "commit",
      "-m",
      &format!("chore(release): v{}\n\n🤖 Generated with ribir-bot", version),
    ])?;
    true
  } else {
    false
  };

  println!("🔧 Publishing with cargo-edit & cargo publish...");
  run_release_publish(CargoWsPublishConfig {
    version: &version,
    has_changelog_commit,
    dry_run: config.dry_run,
  })?;

  if !config.dry_run {
    run_git(&["push", "--follow-tags"])?;
  }

  let is_prerelease = matches!(level, ReleaseLevel::Alpha | ReleaseLevel::Rc);
  println!("🎉 Creating GitHub Release (prerelease: {})...", is_prerelease);

  let release_notes = get_release_notes(&version, Some(&changelog_entries))?;

  if !config.dry_run {
    create_github_release(&version, &release_notes, is_prerelease)?;
    println!("\n✅ Release {} complete!", version);
  } else {
    print_dry_run_summary(&version, &changelog_entries, &release_notes);
  }

  Ok(())
}

/// Enter RC phase: create release branch, merge changelog, generate highlights,
/// create PR, and publish RC.1. Version is auto-detected from the latest git
/// tag.
pub fn cmd_release_enter_rc(config: &Config) -> Result<()> {
  let version = detect_version_from_tag()?;
  let rc_version = format!("{}.{}.{}-rc.1", version.major, version.minor, version.patch);
  let branch_name = format!("release-{}.{}.x", version.major, version.minor);
  let archive_path = format!("changelogs/CHANGELOG-{}.{}.md", version.major, version.minor);

  println!("🚀 Entering RC phase for version {}", version);

  // Step 1: Verify environment and archive changelog on master
  verify_changelog_version(&version)?;
  println!("📦 Archiving CHANGELOG.md to {}", archive_path);
  if !config.dry_run {
    archive_changelog(&version)?;
    run_git(&["add", "CHANGELOG.md", &archive_path])?;
    run_git(&[
      "commit",
      "-m",
      &format!(
        "chore: archive changelog for v{}.{}\n\n🤖 Generated with ribir-bot",
        version.major, version.minor
      ),
    ])?;
    run_git(&["push"])?;
    println!("✅ Archived changelog committed to master");
  }

  // Step 2: Create release branch
  if branch_exists(&branch_name)? {
    return Err(
      format!("Release branch {} already exists. Cannot re-enter RC phase.", branch_name).into(),
    );
  }

  println!("🌿 Creating release branch: {}", branch_name);
  if !config.dry_run {
    create_release_branch(&version)?;
  }

  // Step 3: Merge alpha changelog entries
  println!("🔀 Merging alpha changelog entries for {}...", rc_version);
  let source_path = if config.dry_run { "CHANGELOG.md" } else { &archive_path };
  let changelog_content = run_changelog_merge(&rc_version, config.dry_run, Some(source_path))?;

  // Step 4: Generate AI highlights (for PR, not changelog)
  if !config.dry_run {
    let highlights = generate_and_log_highlights(&changelog_content, &rc_version, None)?;

    // Save changelog without highlights (highlights go in PR body)
    fs::write(&archive_path, &changelog_content)?;
    println!("✅ Updated {}", archive_path);

    commit_and_create_release_pr(&rc_version, &branch_name, &highlights)?;

    println!("📦 Publishing {}...", rc_version);
    // commit_and_create_release_pr already committed, so use --amend
    run_release_publish(CargoWsPublishConfig {
      version: &rc_version,
      has_changelog_commit: true,
      dry_run: config.dry_run,
    })?;
    run_git(&["push", "--follow-tags"])?;

    println!("🎉 Creating GitHub Release for {}...", rc_version);
    let release_notes = extract_version_section(&changelog_content, &rc_version)
      .ok_or_else(|| format!("Release notes not found for {}", rc_version))?;
    create_github_release(&rc_version, &release_notes, true)?;
  } else {
    println!("📝 Skipping AI highlights generation in dry-run mode");
    println!("\n💡 Run without --dry-run to apply changes.");
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

  let release_notes = get_release_notes(&version, None)?;
  let is_prerelease = version.contains("-rc") || version.contains("-alpha");

  println!("🎉 Creating GitHub Release (prerelease={})...", is_prerelease);
  if !config.dry_run {
    create_github_release(&version, &release_notes, is_prerelease)?;
  }

  if let Some(pr) = pr_number {
    let comment = format!(
      "🎉 Release **v{}** has been published!\n\n\
       [View Release](https://github.com/RibirX/Ribir/releases/tag/v{})",
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

/// Release stable version.
pub fn cmd_release_stable(config: &Config, version: Option<&str>) -> Result<()> {
  let version_str = version
    .map(String::from)
    .unwrap_or_else(|| detect_stable_version_from_branch().expect("Failed to detect version"));

  let changelog_path = get_changelog_path()?;

  println!("🚀 Releasing stable version {}...", version_str);

  // Step 1: Extract highlights from PR body
  let highlights_text = match gh_get_pr_body() {
    Ok(body) => extract_highlights_from_pr_body(&body),
    Err(e) => {
      eprintln!("⚠️  Could not get PR body: {}", e);
      None
    }
  };

  if highlights_text.is_some() {
    println!("📋 Found highlights in PR body");
  } else {
    eprintln!("⚠️  No highlights found in PR body, proceeding without highlights");
  }

  // Step 2: Merge RC versions (always merge all pre-releases)
  let changelog = fs::read_to_string(&changelog_path)?;
  println!("🔀 Merging RC versions into stable...");
  let mut updated_changelog = run_changelog_merge(&version_str, config.dry_run, None)?;

  // Track if we made any changelog changes
  let has_changelog_changes = highlights_text.is_some() || changelog != updated_changelog;

  // Insert highlights if available
  if let Some(ref text) = highlights_text {
    updated_changelog = insert_highlights_text(&updated_changelog, &version_str, text)?;
    println!("✅ Inserted highlights into changelog");
  }

  // Commit changelog changes if any
  let has_changelog_commit = if !config.dry_run && has_changelog_changes {
    fs::write(&changelog_path, &updated_changelog)?;
    run_git(&["add", &changelog_path])?;
    run_git(&[
      "commit",
      "-m",
      &format!("chore(release): v{}\n\n🤖 Generated with ribir-bot", version_str),
    ])?;
    println!("✅ Updated CHANGELOG.md with stable version and highlights");
    true
  } else {
    false
  };

  println!("📦 Publishing {}...", version_str);
  run_release_publish(CargoWsPublishConfig {
    version: &version_str,
    has_changelog_commit,
    dry_run: config.dry_run,
  })?;

  if !config.dry_run {
    run_git(&["push", "--follow-tags"])?;
  }

  let release_notes = extract_version_section(&updated_changelog, &version_str)
    .ok_or_else(|| format!("Release notes not found for version {}", version_str))?;

  println!("🎉 Creating stable GitHub Release...");
  if !config.dry_run {
    create_github_release(&version_str, &release_notes, false)?;
    println!("\n✅ Stable release {} published!", version_str);

    // Auto-merge the release PR
    if let Ok(branch) = get_current_branch() {
      // Find PR for this branch
      println!("🔍 Looking for PR for branch {}...", branch);
      // We can iterate PRs or use `gh pr list --head current_branch`
      // Or more simply, since we are usually ON the branch (or checkout it), try `gh
      // pr view --json number -q .number` But wait, the workflow might be
      // running on a detached HEAD or similar if not careful,
      // but `detect_stable_version_from_branch` relies on `get_current_branch`, so we
      // must have a branch name.

      // Using `gh pr list` seems safer if we know the branch name
      let pr_number_output = Command::new("gh")
        .args(["pr", "list", "--head", &branch, "--json", "number", "-q", ".[0].number"])
        .output()?;

      let pr_number = String::from_utf8_lossy(&pr_number_output.stdout)
        .trim()
        .to_string();

      if !pr_number.is_empty() {
        println!("🔀 Merging release PR #{}...", pr_number);
        if let Err(e) = remove_label(&pr_number, "do-not-merge") {
          eprintln!("⚠️ Failed to remove label: {}", e);
        }
        if let Err(e) = merge_pr(&pr_number) {
          eprintln!("⚠️ Failed to merge PR: {}", e);
        } else {
          println!("✅ PR #{} merged!", pr_number);
        }
      } else {
        eprintln!("⚠️ Could not find PR for branch {}", branch);
      }
    }
  } else {
    println!("\n💡 This is a dry-run. Use --execute to apply changes.");
  }

  try_add_reaction(config);
  Ok(())
}

/// Regenerate highlights section in PR body.
///
/// This command is used during RC phase to update highlights in the release PR.
/// The highlights are stored in PR body (between HIGHLIGHTS_START/END markers)
/// and will be written to CHANGELOG.md only when `release-stable` is executed.
pub fn cmd_release_highlights(config: &Config, context: Option<&str>) -> Result<()> {
  println!("🔄 Regenerating highlights in PR body...");

  // Get current PR body and number
  let (pr_number, pr_body) = gh_get_pr_details()?;

  // Get changelog to read entries for AI generation
  let changelog_path = get_changelog_path()?;
  let changelog = fs::read_to_string(&changelog_path)?;
  let version = parse_latest_version(&changelog).ok_or("Could not find version in CHANGELOG.md")?;

  println!("📌 Found version: {}", version);

  let highlights = generate_and_log_highlights(&changelog, &version, context)?;
  let highlights_md = format_highlights(&highlights);
  let updated_body = update_pr_body_highlights(&pr_body, &highlights_md)?;

  if !config.dry_run {
    // Ensure safeguards are in place
    if let Err(e) = add_label(&pr_number.to_string(), "do-not-merge") {
      eprintln!("⚠️ Failed to ensure 'do-not-merge' label: {}", e);
    }
    gh_edit_body(None, &updated_body)?;
    println!("✅ Updated PR body with new highlights");
    try_add_reaction(config);
  } else {
    println!("\n📝 Preview:\n{}", highlights_md);
    println!("\n💡 Run without --dry-run to apply changes.");
  }

  Ok(())
}

/// Verify release state.
pub fn cmd_release_verify() -> Result<()> {
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

  if let Ok(changelog) = fs::read_to_string(&changelog_path)
    && let Some(latest) = parse_latest_version(&changelog)
  {
    println!("   Latest version: {}", latest);
  }

  println!("\n🔧 Required tools:");
  for (cmd, args, name) in [
    ("gh", vec!["--version"], "GitHub CLI"),
    ("gemini", vec!["--version"], "Gemini CLI"),
    ("cargo", vec!["set-version", "--version"], "cargo-edit"),
  ] {
    let status = if Command::new(cmd).args(args).output().is_ok() { "✅" } else { "❌" };
    println!("   {} {}", status, name);
  }

  println!("\n✅ Verification complete");
  Ok(())
}

/// Stub for social card generation.
pub fn cmd_release_social_card(config: &Config) -> Result<()> {
  println!("⚠️  Social card generation is not yet implemented.");
  println!("📌 This feature is planned for future releases.");
  println!("\nSee: dev-docs/release-system/03-social-card-generation.md");

  try_add_reaction(config);
  Ok(())
}

// ============================================================================
// Internal Helpers - Version & Cargo
// ============================================================================

impl ReleaseLevel {
  fn as_str(self) -> &'static str {
    match self {
      ReleaseLevel::Alpha => "alpha",
      ReleaseLevel::Rc => "rc",
      ReleaseLevel::Patch => "patch",
      ReleaseLevel::Minor => "minor",
      ReleaseLevel::Major => "major",
    }
  }
}

fn get_latest_git_tag() -> Result<String> {
  let output = Command::new("git")
    .args(["describe", "--tags", "--abbrev=0"])
    .output()?;

  if !output.status.success() {
    return Err("Failed to get latest git tag".into());
  }

  let tag = String::from_utf8_lossy(&output.stdout)
    .trim()
    .to_string();
  Ok(strip_tag_prefix(&tag).to_string())
}

/// Strip version prefix from git tag.
/// Finds the first position where a valid semver version starts.
fn strip_tag_prefix(tag: &str) -> &str {
  for i in 0..tag.len() {
    if tag.as_bytes()[i].is_ascii_digit() {
      let candidate = &tag[i..];
      // Try parsing as semver (handles prerelease like 0.4.0-alpha.54)
      if Version::parse(candidate).is_ok() {
        return candidate;
      }
      // Also try base version (before first '-') for cases like "0.4.0-alpha.54"
      // where we want to validate "0.4.0" is valid semver structure
      if let Some(base) = candidate.split('-').next()
        && Version::parse(base).is_ok()
      {
        return candidate;
      }
    }
  }
  tag
}

/// Detect version from latest git tag (e.g., v0.4.0-alpha.54 -> 0.4.0).
fn detect_version_from_tag() -> Result<Version> {
  let tag = get_latest_git_tag()?;
  // Extract base version: 0.4.0-alpha.54 -> 0.4.0
  let base_version = tag.split('-').next().unwrap_or(&tag);

  Version::parse(base_version)
    .map_err(|_| format!("Could not parse version from tag: {}", tag).into())
}

fn get_next_version(level: &str) -> Result<String> {
  let tag = get_latest_git_tag()?;
  let mut version = Version::parse(&tag)?;

  match level {
    "alpha" | "rc" => {
      let prefix = format!("{}.", level);
      if let Some(pre_str) = version.pre.as_str().strip_prefix(&prefix)
        && let Ok(num) = pre_str.parse::<u64>()
      {
        version.pre = semver::Prerelease::new(&format!("{}{}", prefix, num + 1)).unwrap();
      } else {
        if version.pre.is_empty() {
          version.patch += 1;
        }
        version.pre = semver::Prerelease::new(&format!("{}1", prefix)).unwrap();
      }
    }
    "patch" => {
      version.patch += 1;
      version.pre = semver::Prerelease::EMPTY;
    }
    "minor" => {
      version.minor += 1;
      version.patch = 0;
      version.pre = semver::Prerelease::EMPTY;
    }
    "major" => {
      version.major += 1;
      version.minor = 0;
      version.patch = 0;
      version.pre = semver::Prerelease::EMPTY;
    }
    _ => return Err(format!("Unsupported release level: {level}").into()),
  }

  Ok(version.to_string())
}

/// Configuration for release publish
struct CargoWsPublishConfig<'a> {
  /// The version to publish (e.g., "0.4.0" or "0.4.0-alpha.55")
  version: &'a str,
  /// Whether changelog was modified and committed before this call
  /// If true, use --amend to merge version bump into that commit
  has_changelog_commit: bool,
  /// Run in dry-run mode (default: true for safety)
  dry_run: bool,
}

/// Publish using cargo-edit (set-version) and cargo publish.
///
/// If `has_changelog_commit` is true, uses `--amend` to merge the version
/// bump into the existing changelog commit, resulting in a single clean commit.
fn run_release_publish(cfg: CargoWsPublishConfig) -> Result<()> {
  // 1. Set Version
  let mut args = vec![
    "set-version".to_string(),
    "--workspace".to_string(),
    cfg.version.to_string(),
    "--exclude".to_string(),
    "ribir-bot".to_string(),
    "--exclude".to_string(),
    "cli".to_string(),
  ];

  if cfg.dry_run {
    args.push("--dry-run".to_string());
  }

  println!("🔧 Running: cargo {}", args.join(" "));
  let status = Command::new("cargo").args(&args).status()?;

  if !status.success() {
    return Err(format!("cargo set-version failed with exit code: {:?}", status.code()).into());
  }

  if !cfg.dry_run {
    // 2. Update Cargo.lock
    println!("🔧 Updating Cargo.lock...");
    let status = Command::new("cargo")
      .args(&["check", "--workspace"])
      .status()?;
    if !status.success() {
      return Err("Failed to update Cargo.lock".into());
    }

    // 3. Commit
    run_git(&["add", "."])?;

    let commit_msg = format!("chore(release): v{}\n\n🤖 Generated with ribir-bot", cfg.version);

    if cfg.has_changelog_commit {
      run_git(&["commit", "--amend", "-m", &commit_msg])?;
    } else {
      run_git(&["commit", "-m", &commit_msg])?;
    }

    // 4. Tag
    println!("🏷️  Creating git tag v{}...", cfg.version);
    run_git(&["tag", &format!("v{}", cfg.version)])?;
  } else {
    println!("📝 Skipping Lockfile update, Commit, and Tag in dry-run mode");
  }

  // 5. Publish
  println!("🚀 Publishing workspace...");
  let mut cmd = Command::new("cargo");
  cmd.args(&["publish", "--workspace", "--exclude", "ribir-bot", "--exclude", "cli"]);

  if cfg.dry_run {
    cmd.arg("--dry-run");
    cmd.arg("--allow-dirty");
  }

  let status = cmd.status()?;

  if !status.success() {
    return Err("Failed to publish workspace".into());
  }

  Ok(())
}

fn get_version_from_context() -> Result<String> {
  // Try git tag first (most reliable after cargo publish)

  if let Ok(version) = get_latest_git_tag() {
    return Ok(version);
  }

  // Fallback: parse from CHANGELOG.md
  let changelog = fs::read_to_string("CHANGELOG.md")?;
  parse_latest_version(&changelog).ok_or("Could not determine version from context".into())
}

fn detect_stable_version_from_branch() -> Result<String> {
  let branch = get_current_branch()?;

  if let Some(suffix) = branch.strip_prefix("release-") {
    let parts: Vec<&str> = suffix.split('.').collect();
    if parts.len() == 3
      && parts[2] == "x"
      && let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
    {
      let version = format!("{}.{}.0", major, minor);
      println!("📌 Auto-detected version {} from branch {}", version, branch);
      return Ok(version);
    }
  }

  Err(
    format!(
      "Cannot auto-detect version: current branch '{}' is not a release branch (expected \
       release-X.Y.x)",
      branch
    )
    .into(),
  )
}

// ============================================================================
// Internal Helpers - Changelog
// ============================================================================

fn collect_changelog_entries(version: &str, dry_run: bool) -> Result<String> {
  use crate::commands::cmd_collect;

  let collect_config = Config {
    command: crate::types::Cmd::Release { cmd: ReleaseCmd::Verify },
    dry_run,
    repo: None,
    comment_id: None,
  };

  let generated_content = cmd_collect(&collect_config, version, !dry_run)?;

  if dry_run && !generated_content.is_empty() {
    Ok(
      extract_version_section(&generated_content, version)
        .unwrap_or_else(|| format!("(Changelog entries for {} will be collected)", version)),
    )
  } else {
    let changelog = fs::read_to_string("CHANGELOG.md")?;
    Ok(
      extract_version_section(&changelog, version)
        .unwrap_or_else(|| format!("(Changelog entries for {} will be collected)", version)),
    )
  }
}

fn get_release_notes(version: &str, fallback: Option<&str>) -> Result<String> {
  let changelog = fs::read_to_string("CHANGELOG.md")?;

  extract_version_section(&changelog, version)
    .or_else(|| fallback.map(String::from))
    .ok_or_else(|| format!("Release notes not found for version {}", version).into())
}

/// Verify that the current environment is correct for entering RC phase.
/// The CHANGELOG.md should contain entries for the same major.minor version.
fn verify_changelog_version(version: &Version) -> Result<()> {
  let changelog = fs::read_to_string("CHANGELOG.md")?;

  let changelog_version = parse_latest_version(&changelog)
    .and_then(|v| Version::parse(&v).ok())
    .ok_or("Could not parse version from CHANGELOG.md")?;

  if version.major != changelog_version.major || version.minor != changelog_version.minor {
    return Err(
      format!(
        "Version mismatch: git tag indicates {}.{}.x but CHANGELOG.md contains {}.{}.x",
        version.major, version.minor, changelog_version.major, changelog_version.minor
      )
      .into(),
    );
  }

  Ok(())
}

fn archive_changelog(version: &Version) -> Result<()> {
  let source = "CHANGELOG.md";
  let dest = format!("changelogs/CHANGELOG-{}.{}.md", version.major, version.minor);

  fs::create_dir_all("changelogs")?;
  fs::copy(source, &dest)?;

  let new_content = "# Changelog\n\nAll notable changes to this project will be documented in \
                     this file.\n\nFor older versions, see the [changelogs](changelogs/) \
                     folder.\n\n<!-- next-header -->\n"
    .to_string();

  fs::write(source, new_content)?;
  Ok(())
}

fn run_changelog_merge(
  version: &str, dry_run: bool, changelog_path: Option<&str>,
) -> Result<String> {
  let arena = Arena::new();
  let ctx = match changelog_path {
    Some(path) => ChangelogContext::load_from_path(&arena, path)?,
    None => ChangelogContext::load(&arena)?,
  };
  let target_ver = Version::parse(version)?;

  ctx.merge_prereleases(&target_ver)?;
  ctx.save_and_get_content(dry_run)
}

// ============================================================================
// Internal Helpers - Highlights
// ============================================================================

fn generate_and_log_highlights(
  changelog: &str, version: &str, context: Option<&str>,
) -> Result<Vec<Highlight>> {
  println!("✨ Generating highlights with AI...");
  let entries = extract_version_section(changelog, version)
    .ok_or_else(|| format!("No entries found for version {}", version))?;

  let highlights = generate_highlights(&entries, context)?;
  println!("📝 Generated {} highlights", highlights.len());
  Ok(highlights)
}

fn generate_highlights(entries: &str, context: Option<&str>) -> Result<Vec<Highlight>> {
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

  let parsed: HighlightsResponse = serde_json::from_str(&json_str)
    .map_err(|e| format!("Invalid JSON from AI: {e}\nRaw: {response}"))?;

  validate_highlights(&parsed.highlights)?;
  Ok(parsed.highlights)
}

fn validate_highlights(highlights: &[Highlight]) -> Result<()> {
  if !(3..=5).contains(&highlights.len()) {
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

// ============================================================================
// Internal Helpers - Git & PR
// ============================================================================

fn commit_and_create_release_pr(
  rc_version: &str, branch_name: &str, highlights: &[Highlight],
) -> Result<()> {
  let changelog_path = get_changelog_path()?;
  run_git(&["add", &changelog_path])?;

  run_git(&[
    "commit",
    "-m",
    &format!("chore(release): v{}\n\n🤖 Generated with ribir-bot\n", rc_version),
  ])?;

  run_git(&["push", "-u", "origin", branch_name])?;

  // Extract stable version from rc_version (e.g., "0.4.0-rc.1" -> "0.4.0")
  let stable_version = rc_version.split('-').next().unwrap_or(rc_version);

  // Format highlights for PR body
  let highlights_md = format_highlights(highlights);

  let pr_title = format!("Release {} Preparation", stable_version);
  let pr_body = format!(
    r#"## 🚀 Release Preparation for v{stable_version}

### Version Info
| Item | Value |
|------|-------|
| Target Stable | v{stable_version} |
| Release Branch | `{branch_name}` |

### Changes
- ✅ Merged changelog from all alpha versions
- ✅ AI-generated highlights (editable below)

### 📝 Highlights

> [!TIP]
> Edit the highlights below. They will be written to CHANGELOG.md when `release-stable` is executed.

<!-- HIGHLIGHTS_START -->
{highlights_md}
<!-- HIGHLIGHTS_END -->

### Bot Commands
Comment on this PR to trigger actions:
| Command | Description |
|---------|-------------|
| `@ribir-bot release-highlights` | Regenerate highlights section |
| `@ribir-bot release-highlights --context "..."` | Regenerate with additional context |
| `@ribir-bot release-stable` | Publish stable and auto-merge this PR |

### Review Checklist
- [ ] Verify highlights are accurate and well-written
- [ ] Check all important PRs are included
- [ ] Confirm version and date are correct

### Next Steps
1. 🧪 Community tests the RC.1 (already published to crates.io)
2. 🐛 If bugs found:
   - Fix and push to this branch
   - [Trigger "Release RC" workflow](../../actions/workflows/release-rc.yml) for rc.2, rc.3, etc.
3. ✅ When ready, comment `@ribir-bot release-stable` to:
   - Publish stable version to crates.io
   - Create GitHub Release
   - Auto-merge this PR to master

---
🤖 Generated by ribir-bot"#,
    stable_version = stable_version,
    branch_name = branch_name,
    highlights_md = highlights_md
  );

  let pr_url = create_pr(&pr_title, &pr_body, "master", branch_name, Some(&["do-not-merge"]))?;
  println!("✅ Created PR: {}", pr_url);

  Ok(())
}

// ============================================================================
// Internal Helpers - Misc
// ============================================================================

fn print_dry_run_summary(version: &str, entries: &str, notes: &str) {
  let separator = "─".repeat(60);
  println!("\n{}", separator);
  println!("📝 Changelog entries for {}:\n", version);
  println!("{}", entries);
  println!("\n{}", separator);
  println!("📄 Release notes preview:\n");
  println!("{}", notes);
  println!("\n{}", separator);
  println!("\n💡 This is a dry-run. Use --execute to apply changes.");
}

fn try_add_reaction(config: &Config) {
  if let Some(comment_id) = config.comment_id.flatten()
    && let Err(e) = add_reaction(comment_id, "rocket")
  {
    eprintln!("⚠️ Failed to add reaction: {e}");
  }
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

  #[test]
  fn test_strip_tag_prefix() {
    // Various prefix formats
    assert_eq!(strip_tag_prefix("v0.4.0-alpha.54"), "0.4.0-alpha.54");
    assert_eq!(strip_tag_prefix("v1.0.0"), "1.0.0");
    assert_eq!(strip_tag_prefix("ribir-v0.4.0-alpha.53"), "0.4.0-alpha.53");
    assert_eq!(strip_tag_prefix("ribir_painter-v0.0.1-alpha.1"), "0.0.1-alpha.1");
    assert_eq!(strip_tag_prefix("foo-bar-v2.0.0"), "2.0.0");

    // Prefix with numbers (should skip non-semver numbers)
    assert_eq!(strip_tag_prefix("release2-v1.0.0"), "1.0.0");
    assert_eq!(strip_tag_prefix("v2alpha-1.0.0"), "1.0.0");

    // No prefix
    assert_eq!(strip_tag_prefix("0.4.0"), "0.4.0");
    assert_eq!(strip_tag_prefix("1.2.3-rc.1"), "1.2.3-rc.1");

    // Invalid (no semver found, returns original)
    assert_eq!(strip_tag_prefix("invalid"), "invalid");
  }
}

//! Changelog AST manipulation and parsing.
//!
//! This module provides AST-based changelog parsing and manipulation using comrak.

use std::{cell::RefCell, fs};

use comrak::{
  Arena, Options, arena_tree::Node, nodes::{Ast, LineColumn, ListDelimType, ListType, NodeCode, NodeHeading, NodeList, NodeValue}, parse_document
};
use semver::Version;

use crate::{
  types::{Highlight, Result},
  utils::get_changelog_path,
};

/// Changelog marker constants.
pub const MARKER_START: &str = "<!-- RIBIR_CHANGELOG_START -->";
pub const MARKER_END: &str = "<!-- RIBIR_CHANGELOG_END -->";

// ============================================================================
// Changelog AST Types
// ============================================================================

/// Parsed release from changelog.
pub struct Release<'a> {
  pub version: Version,
  pub date: String,
  pub header: &'a Node<'a, RefCell<Ast>>,
}

/// Changelog AST wrapper.
pub struct Changelog<'a> {
  pub root: &'a Node<'a, RefCell<Ast>>,
}

impl<'a> Changelog<'a> {
  pub fn analyze(root: &'a Node<'a, RefCell<Ast>>) -> Self { Self { root } }

  pub fn releases(&self) -> Vec<Release<'a>> {
    self
      .root
      .children()
      .filter_map(|n| {
        if let NodeValue::Heading(ref h) = n.data.borrow().value {
          if h.level == 2 {
            return Release::parse(n);
          }
        }
        None
      })
      .collect()
  }

  pub fn latest_version(&self) -> Option<Version> {
    self.releases().into_iter().map(|r| r.version).next()
  }

  /// Returns (pre-releases to merge, target release if exists)
  pub fn find_merge_candidates(
    &self, target: &Version,
  ) -> (Vec<Release<'a>>, Option<&'a Node<'a, RefCell<Ast>>>) {
    let mut pres = Vec::new();
    let mut target_node = None;

    for r in self.releases() {
      if &r.version == target {
        target_node = Some(r.header);
      } else if is_prerelease(&r.version, target) {
        pres.push(r);
      }
    }
    (pres, target_node)
  }

  /// Find RC versions for a given base version (AST-based).
  pub fn find_rc_versions(&self, target: &Version) -> Vec<String> {
    self
      .releases()
      .into_iter()
      .filter(|r| {
        r.version.major == target.major
          && r.version.minor == target.minor
          && r.version.patch == target.patch
          && r.version.pre.to_string().starts_with("rc.")
      })
      .map(|r| r.version.to_string())
      .collect()
  }
}

impl<'a> Release<'a> {
  pub fn parse(node: &'a Node<'a, RefCell<Ast>>) -> Option<Self> {
    let text = collect_text(node);
    if text.to_lowercase().contains("unreleased") {
      return None;
    }

    let parts: Vec<&str> = text.split(" - ").collect();
    let ver_str = parts
      .first()?
      .trim()
      .trim_matches(|c| c == '[' || c == ']' || c == 'v');
    let version = Version::parse(ver_str).ok()?;
    let date = parts.get(1).unwrap_or(&"").to_string();

    Some(Self { version, date, header: node })
  }

  /// Extract content section between this release header and the next h2.
  pub fn extract_section(&self) -> String {
    let mut content = String::new();
    let mut curr = self.header.next_sibling();

    while let Some(node) = curr {
      if matches!(node.data.borrow().value, NodeValue::Heading(ref h) if h.level <= 2) {
        break;
      }
      content.push_str(&render_node(node));
      content.push('\n');
      curr = node.next_sibling();
    }

    content.trim().to_string()
  }

  /// Find highlights section within this release.
  pub fn find_highlights(&self) -> Option<String> {
    let section = self.extract_section();
    let start = section.find("**Highlights:**")?;
    let rest = &section[start..];
    let end = rest.find("\n###").unwrap_or(rest.len());
    Some(rest[..end].to_string())
  }
}

/// Check if a version is a prerelease of a target version.
pub fn is_prerelease(pre: &Version, target: &Version) -> bool {
  pre.major == target.major
    && pre.minor == target.minor
    && pre.patch == target.patch
    && !pre.pre.is_empty()
}

/// Collect text content from a node.
pub fn collect_text<'a>(node: &'a Node<'a, RefCell<Ast>>) -> String {
  let mut s = String::new();
  for c in node.children() {
    match &c.data.borrow().value {
      NodeValue::Text(t) | NodeValue::Code(NodeCode { literal: t, .. }) => s.push_str(t),
      _ => s.push_str(&collect_text(c)),
    }
  }
  s
}

/// Render a node to markdown string (simple version).
fn render_node<'a>(node: &'a Node<'a, RefCell<Ast>>) -> String {
  let mut out = Vec::new();
  comrak::format_commonmark(node, &Options::default(), &mut out).ok();
  String::from_utf8_lossy(&out).into_owned()
}

// ============================================================================
// Changelog Context
// ============================================================================

/// Context for changelog AST manipulation.
pub struct ChangelogContext<'a> {
  pub arena: &'a Arena<Node<'a, RefCell<Ast>>>,
  pub changelog: Changelog<'a>,
  pub root: &'a Node<'a, RefCell<Ast>>,
  pub path: String,
}

impl<'a> ChangelogContext<'a> {
  pub fn load(arena: &'a Arena<Node<'a, RefCell<Ast>>>) -> Result<Self> {
    let path = get_changelog_path()?;
    println!("📂 Using changelog: {}", path);

    // Ensure parent directory exists for release branch paths
    if let Some(parent) = std::path::Path::new(&path).parent() {
      if !parent.as_os_str().is_empty() && !parent.exists() {
        fs::create_dir_all(parent)?;
      }
    }

    // If file doesn't exist, create with basic structure
    let content = if std::path::Path::new(&path).exists() {
      fs::read_to_string(&path)?
    } else {
      println!("📄 Creating new changelog: {}", path);
      create_initial_changelog()
    };

    Self::load_from_content_with_path(arena, &content, path)
  }

  /// Load from content string (for testing).
  #[cfg(test)]
  pub fn load_from_content(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>, content: &str,
  ) -> Result<Self> {
    Self::load_from_content_with_path(arena, content, String::new())
  }

  fn load_from_content_with_path(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>, content: &str, path: String,
  ) -> Result<Self> {
    let root = parse_document(arena, content, &Options::default());
    let changelog = Changelog::analyze(root);
    Ok(Self { arena, changelog, root, path })
  }

  pub fn save(&self, dry_run: bool) -> Result<()> {
    self.save_and_get_content(dry_run)?;
    Ok(())
  }

  /// Save changelog and return the generated content.
  pub fn save_and_get_content(&self, dry_run: bool) -> Result<String> {
    let mut out = Vec::new();
    comrak::format_commonmark(self.root, &Options::default(), &mut out)?;
    let content = String::from_utf8(out)?;

    if dry_run {
      println!("📝 Preview:\n{}\n... (truncated)", &content.chars().take(2000).collect::<String>());
      println!("\n💡 Run with --write to apply.");
    } else {
      // Ensure parent directory exists
      if let Some(parent) = std::path::Path::new(&self.path).parent() {
        if !parent.as_os_str().is_empty() {
          fs::create_dir_all(parent)?;
        }
      }
      fs::write(&self.path, &content)?;
      println!("✅ Saved {}", self.path);
    }
    Ok(content)
  }

  pub fn new_node(&self, value: NodeValue) -> &'a Node<'a, RefCell<Ast>> {
    self
      .arena
      .alloc(Node::new(RefCell::new(Ast::new(value, LineColumn { line: 0, column: 0 }))))
  }

  pub fn new_text(&self, text: String) -> &'a Node<'a, RefCell<Ast>> {
    self.new_node(NodeValue::Text(text))
  }

  pub fn new_heading(&self, level: u8, text: &str) -> &'a Node<'a, RefCell<Ast>> {
    let h = self.new_node(NodeValue::Heading(NodeHeading { level, setext: false }));
    h.append(self.new_text(text.to_string()));
    h
  }

  pub fn deep_clone<'b>(&self, node: &'b Node<'b, RefCell<Ast>>) -> &'a Node<'a, RefCell<Ast>> {
    let new_node = self.new_node(node.data.borrow().value.clone());
    for child in node.children() {
      new_node.append(self.deep_clone(child));
    }
    new_node
  }

  pub fn new_list_item(&self, text: &str) -> &'a Node<'a, RefCell<Ast>> {
    let item = self.new_node(NodeValue::Item(NodeList {
      list_type: ListType::Bullet,
      delimiter: ListDelimType::Period,
      bullet_char: b'-',
      tight: true,
      ..NodeList::default()
    }));

    let p = self.new_node(NodeValue::Paragraph);
    p.append(self.new_text(text.to_string()));
    item.append(p);
    item
  }

  pub fn ensure_release(&self, ver: &Version, date: &str) -> &'a Node<'a, RefCell<Ast>> {
    if let Some(r) = self.changelog.releases().iter().find(|r| &r.version == ver) {
      return r.header;
    }

    // Create new
    let text = format!("[{}] - {}", ver, date);
    let h2 = self.new_heading(2, &text);

    // Insert: Find insertion point (first H2 or specific marker)
    let insert_node = self
      .root
      .children()
      .find(|n| {
        // After start marker
        if let NodeValue::HtmlBlock(ref h) = n.data.borrow().value {
          return h.literal.contains("next-header");
        }
        // Or before first H2
        matches!(n.data.borrow().value, NodeValue::Heading(ref h) if h.level == 2)
      })
      .unwrap_or(self.root.last_child().unwrap_or(self.root));

    if matches!(insert_node.data.borrow().value, NodeValue::HtmlBlock(_)) {
      insert_node.insert_after(h2);
    } else {
      insert_node.insert_before(h2);
    }

    h2
  }

  /// Merge prereleases into target version (shared logic for changelog and release commands).
  pub fn merge_prereleases(&self, target: &Version) -> Result<()> {
    let (mut prereleases, target_node) = self.changelog.find_merge_candidates(target);
    if prereleases.is_empty() {
      return Err(format!("No pre-releases found for {}", target).into());
    }
    println!("📦 Merging {} pre-releases", prereleases.len());

    let target_release =
      target_node.unwrap_or_else(|| self.ensure_release(target, &crate::utils::today()));
    let mut insert_point = target_release;

    for pre in prereleases.drain(..) {
      let mut curr = pre.header.next_sibling();
      while let Some(node) = curr {
        let next = node.next_sibling();
        if matches!(node.data.borrow().value, NodeValue::Heading(ref h) if h.level <= 2) {
          break;
        }
        node.detach();
        insert_point.insert_after(node);
        insert_point = node;
        curr = next;
      }
      pre.header.detach();
    }

    Ok(())
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create initial changelog content.
pub fn create_initial_changelog() -> String {
  "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n<!-- \
   next-header -->\n"
    .to_string()
}

/// Format highlights as markdown.
pub fn format_highlights(highlights: &[Highlight]) -> String {
  let mut result = String::from("**Highlights:**\n");
  for h in highlights {
    result.push_str(&format!("- {} {}\n", h.emoji, h.description));
  }
  result
}

/// Insert highlights into changelog after version header.
pub fn insert_highlights(changelog: &str, version: &str, highlights: &[Highlight]) -> Result<String> {
  let header_pattern = format!("## [{}]", version);
  let header_pos = changelog.find(&header_pattern).ok_or("Version header not found")?;

  let rest = &changelog[header_pos..];
  let header_end = rest.find('\n').unwrap_or(rest.len());
  let insert_pos = header_pos + header_end;

  let highlights_text = format!("\n\n{}", format_highlights(highlights));

  let mut result = String::with_capacity(changelog.len() + highlights_text.len());
  result.push_str(&changelog[..insert_pos]);
  result.push_str(&highlights_text);
  result.push_str(&changelog[insert_pos..]);

  Ok(result)
}

/// Replace version header in changelog.
pub fn replace_version_header(changelog: &str, old_version: &str, new_version: &str) -> String {
  let old_header = format!("## [{}]", old_version);
  let new_header = format!("## [{}]", new_version);
  changelog.replace(&old_header, &new_header)
}

/// Extract a version section from the changelog (string-based, for output).
pub fn extract_version_section(changelog: &str, version: &str) -> Option<String> {
  // Try both escaped and unescaped patterns (comrak may escape brackets)
  let patterns = [
    format!("## [{}]", version),
    format!("## \\[{}\\]", version),
  ];

  let (start, matched_pattern) = patterns
    .iter()
    .find_map(|p| changelog.find(p).map(|pos| (pos, p.as_str())))?;

  let rest = &changelog[start..];

  // Find the next version header (either escaped or unescaped)
  let end = rest[3..]
    .find("\n## [")
    .or_else(|| rest[3..].find("\n## \\["))
    .map(|pos| pos + 3)
    .unwrap_or(rest.len());

  let section = rest[..end].to_string();

  // Unescape the section if it was escaped
  if matched_pattern.contains("\\[") {
    Some(section.replace("\\[", "[").replace("\\]", "]"))
  } else {
    Some(section)
  }
}

/// Find RC highlights from changelog (uses AST via Release).
pub fn find_rc_highlights(changelog: &str, rc_version: &str) -> Option<String> {
  let arena = Arena::new();
  let root = parse_document(&arena, changelog, &Options::default());
  let cl = Changelog::analyze(root);

  let ver = Version::parse(rc_version).ok()?;
  cl.releases()
    .into_iter()
    .find(|r| r.version == ver)
    .and_then(|r| r.find_highlights())
}

/// Find RC versions for a given base version.
pub fn find_rc_versions(changelog: &str, version: &Version) -> Vec<String> {
  let arena = Arena::new();
  let root = parse_document(&arena, changelog, &Options::default());
  let cl = Changelog::analyze(root);
  cl.find_rc_versions(version)
}

/// Parse the latest version from changelog.
pub fn parse_latest_version(changelog: &str) -> Option<String> {
  let arena = Arena::new();
  let root = parse_document(&arena, changelog, &Options::default());
  let cl = Changelog::analyze(root);
  cl.latest_version().map(|v| v.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_prerelease() {
    let target = Version::parse("0.5.0").unwrap();
    assert!(is_prerelease(&Version::parse("0.5.0-alpha.1").unwrap(), &target));
    assert!(is_prerelease(&Version::parse("0.5.0-rc.1").unwrap(), &target));
    assert!(!is_prerelease(&Version::parse("0.5.0").unwrap(), &target));
    assert!(!is_prerelease(&Version::parse("0.4.0-alpha.1").unwrap(), &target));
  }

  #[test]
  fn test_parse_release_header() {
    let arena = Arena::new();
    let content = "## [0.5.0-alpha.1] - 2025-01-15\n\n### Features\n- feat: test";
    let root = parse_document(&arena, content, &Options::default());
    let changelog = Changelog::analyze(root);

    let releases = changelog.releases();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].version.to_string(), "0.5.0-alpha.1");
    assert_eq!(releases[0].date, "2025-01-15");
  }

  #[test]
  fn test_parse_multiple_releases() {
    let arena = Arena::new();
    let content = r#"
## [0.5.0-alpha.2] - 2025-01-20

### Features
- feat: feature 2

## [0.5.0-alpha.1] - 2025-01-15

### Features
- feat: feature 1
"#;
    let root = parse_document(&arena, content, &Options::default());
    let changelog = Changelog::analyze(root);

    let releases = changelog.releases();
    assert_eq!(releases.len(), 2);
    assert_eq!(releases[0].version.to_string(), "0.5.0-alpha.2");
    assert_eq!(releases[1].version.to_string(), "0.5.0-alpha.1");
  }

  #[test]
  fn test_find_merge_candidates() {
    let arena = Arena::new();
    let content = r#"
## [0.5.0] - 2025-02-01

### Features
- feat: stable

## [0.5.0-rc.1] - 2025-01-25

### Features
- feat: rc1

## [0.5.0-alpha.2] - 2025-01-20

### Features
- feat: alpha2

## [0.5.0-alpha.1] - 2025-01-15

### Features
- feat: alpha1
"#;
    let root = parse_document(&arena, content, &Options::default());
    let changelog = Changelog::analyze(root);

    let target = Version::parse("0.5.0").unwrap();
    let (prereleases, target_node) = changelog.find_merge_candidates(&target);

    assert_eq!(prereleases.len(), 3);
    assert!(target_node.is_some());
  }

  #[test]
  fn test_skip_unreleased_section() {
    let arena = Arena::new();
    let content = r#"
## [Unreleased]

### Features
- feat: wip

## [0.5.0-alpha.1] - 2025-01-15

### Features
- feat: released
"#;
    let root = parse_document(&arena, content, &Options::default());
    let changelog = Changelog::analyze(root);

    let releases = changelog.releases();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].version.to_string(), "0.5.0-alpha.1");
  }

  #[test]
  fn test_format_highlights() {
    let highlights = vec![
      Highlight { emoji: "⚡".into(), description: "50% faster rendering".into() },
      Highlight { emoji: "🎨".into(), description: "Dark mode support".into() },
    ];
    let result = format_highlights(&highlights);
    assert!(result.contains("**Highlights:**"));
    assert!(result.contains("- ⚡ 50% faster rendering"));
    assert!(result.contains("- 🎨 Dark mode support"));
  }

  #[test]
  fn test_parse_latest_version() {
    let changelog = r#"# Changelog

## [0.5.0-alpha.5] - 2025-01-15

### Features
- feat: something

## [0.5.0-alpha.4] - 2025-01-10
"#;
    let version = parse_latest_version(changelog).unwrap();
    assert_eq!(version, "0.5.0-alpha.5");
  }

  #[test]
  fn test_find_rc_versions() {
    let changelog = r#"
## [0.5.0-rc.2] - 2025-01-20

### Fixed

## [0.5.0-rc.1] - 2025-01-15

### Features

## [0.5.0-alpha.5] - 2025-01-10
"#;
    let ver = Version::parse("0.5.0").unwrap();
    let rcs = find_rc_versions(changelog, &ver);
    assert_eq!(rcs.len(), 2);
    assert!(rcs.contains(&"0.5.0-rc.1".to_string()));
    assert!(rcs.contains(&"0.5.0-rc.2".to_string()));
  }

  #[test]
  fn test_extract_version_section() {
    let changelog = r#"## [0.5.0-rc.1] - 2025-01-15

### Features
- feat: something new

### Fixed
- fix: some bug

## [0.4.0] - 2025-01-01

### Features
- feat: old stuff
"#;
    let section = extract_version_section(changelog, "0.5.0-rc.1").unwrap();
    assert!(section.contains("feat: something new"));
    assert!(section.contains("fix: some bug"));
    assert!(!section.contains("feat: old stuff"));
  }

  #[test]
  fn test_insert_highlights_into_changelog() {
    let changelog = r#"## [0.5.0-rc.1] - 2025-01-15

### 🎨 Features
- feat(widgets): Add dark mode
"#;
    let highlights = vec![Highlight { emoji: "🎨".into(), description: "Dark mode support".into() }];

    let result = insert_highlights(changelog, "0.5.0-rc.1", &highlights).unwrap();

    assert!(result.contains("**Highlights:**"));
    assert!(result.contains("- 🎨 Dark mode support"));
    let highlights_pos = result.find("**Highlights:**").unwrap();
    let features_pos = result.find("### 🎨 Features").unwrap();
    assert!(highlights_pos < features_pos);
  }

  #[test]
  fn test_find_rc_highlights_reuse() {
    let changelog = r#"## [0.5.0-rc.1] - 2025-01-15

**Highlights:**
- ⚡ 50% faster rendering
- 🎨 Dark mode support

### Features
- feat: something
"#;
    let highlights = find_rc_highlights(changelog, "0.5.0-rc.1").unwrap();
    assert!(highlights.contains("50% faster rendering"));
    assert!(highlights.contains("Dark mode support"));
  }

  #[test]
  fn test_replace_version_header() {
    let changelog = "## [0.5.0-rc.1] - 2025-01-15\n\nContent here\n";
    let result = replace_version_header(changelog, "0.5.0-rc.1", "0.5.0");
    assert!(result.contains("## [0.5.0] - 2025-01-15"));
    assert!(!result.contains("-rc.1"));
  }

  #[test]
  fn test_create_initial_changelog() {
    let content = create_initial_changelog();
    assert!(content.contains("# Changelog"));
    assert!(content.contains("<!-- next-header -->"));
  }
}

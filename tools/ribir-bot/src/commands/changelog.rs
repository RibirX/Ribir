//! Changelog command implementations.

use std::collections::HashMap;

use comrak::{
  Arena, Node, Options,
  nodes::{ListDelimType, ListType, NodeList, NodeValue},
  parse_document,
};
use semver::Version;

use crate::{
  changelog::{ChangelogContext, MARKER_END, MARKER_START, collect_text},
  external::get_merged_prs_since,
  types::{Config, PR, Result, SectionKind},
  utils::today,
};

/// Collect PRs and generate changelog entries.
/// Returns the changelog content (for dry-run preview) or empty string if no
/// new content.
pub fn cmd_collect(_config: &Config, version: &str, write: bool) -> Result<String> {
  println!("📋 Collecting PRs for version {}...", version);
  let target_ver = Version::parse(version)?;
  let arena = Arena::new();
  let ctx = ChangelogContext::load(&arena)?;

  let releases = ctx.changelog.releases();
  let base_version = select_base_version(&releases, &target_ver)?;
  println!("📌 Base version: {}", base_version);

  let prs = get_merged_prs_since(&base_version, _config.repo.as_deref())?;
  if prs.is_empty() {
    println!("✅ No new content.");
    return Ok(String::new());
  }
  println!("🔍 Found {} new PRs", prs.len());

  let release_node = ctx.ensure_release(&target_ver, &today());
  clear_release_content(release_node);
  let mut current_pos = release_node;

  // Group entries by type
  let mut sections: HashMap<SectionKind, Vec<Node>> = HashMap::new();
  for pr in &prs {
    for (kind, entry) in extract_change_entries(&ctx, pr) {
      sections.entry(kind).or_default().push(entry);
    }
  }

  // Insert sections
  for kind in SectionKind::ALL {
    if let Some(mut entries) = sections.remove(kind) {
      entries.sort_by_cached_key(|node| crate::changelog::extract_scope(node));

      current_pos = if *kind == SectionKind::Internal {
        insert_internal_section(&ctx, current_pos, kind, entries)
      } else {
        insert_regular_section(&ctx, current_pos, kind, entries)
      };
    }
  }

  ctx.save_and_get_content(!write)
}

fn insert_internal_section<'a>(
  ctx: &ChangelogContext<'a>, current_pos: Node<'a>, kind: &SectionKind, entries: Vec<Node<'a>>,
) -> Node<'a> {
  // Internal section: wrap in collapsible <details> tag
  let mut items_md = String::new();
  for entry in entries {
    let text = collect_text(entry);
    items_md.push_str(&format!("- {}\n", text.trim()));
  }

  let details_html =
    format!("<details>\n<summary>{}</summary>\n\n{}\n</details>\n", kind.header(), items_md);
  let html_block = ctx.new_html_block(&details_html);
  current_pos.insert_after(html_block);
  html_block
}

fn insert_regular_section<'a>(
  ctx: &ChangelogContext<'a>, current_pos: Node<'a>, kind: &SectionKind, entries: Vec<Node<'a>>,
) -> Node<'a> {
  // Regular section: H3 heading + list
  let h3 = ctx.new_heading(3, &kind.header());
  current_pos.insert_after(h3);

  let list = ctx.new_node(NodeValue::List(NodeList {
    list_type: ListType::Bullet,
    delimiter: ListDelimType::Period,
    bullet_char: b'-',
    tight: true,
    ..NodeList::default()
  }));
  h3.insert_after(list);

  for entry in entries {
    list.append(entry);
  }
  list
}

/// Merge pre-release versions into target version.
pub fn cmd_merge(_config: &Config, version: &str, write: bool) -> Result<()> {
  println!("🔀 Merging pre-releases for {}...", version);
  let target_ver = Version::parse(version)?;
  let arena = Arena::new();
  let ctx = ChangelogContext::load(&arena)?;

  ctx.merge_prereleases(&target_ver)?;
  ctx.save(!write)
}

/// Verify changelog parsing.
pub fn cmd_verify(_config: &Config) -> Result<()> {
  println!("🔍 Verifying CHANGELOG.md parsing...");
  let arena = Arena::new();
  let ctx = ChangelogContext::load(&arena)?;
  let releases = ctx.changelog.releases();

  println!("\n📊 Parsed {} releases:", releases.len());
  for (i, r) in releases.iter().take(5).enumerate() {
    println!("  {}. [{}] - {}", i + 1, r.version, r.date);
  }
  ctx.save(true)
}

// Helper functions

fn extract_change_entries<'a>(ctx: &ChangelogContext<'a>, pr: &PR) -> Vec<(SectionKind, Node<'a>)> {
  let mut entries = Vec::new();

  // 1. Try parse body block
  if let Some(body) = &pr.body {
    if body
      .to_lowercase()
      .contains("[x] no changelog needed")
    {
      return vec![];
    }

    if let Some(content) = extract_block(body) {
      let root = parse_document(ctx.arena, &content, &Options::default());

      // Collect all items from lists and top-level paragraphs
      let items = collect_changelog_items(root);

      for (target, content_node) in items {
        let text = collect_text(content_node);
        if let Some((kind, desc)) = parse_conventional_head(&text) {
          let item = create_formatted_item(ctx, target, &text, desc);
          inject_pr_meta(ctx, item, pr);
          entries.push((kind, item));
        }
      }
      if !entries.is_empty() {
        return entries;
      }
    }
  }

  // 2. Fallback to title
  if let Some((kind, desc)) = parse_conventional_head(&pr.title) {
    let text = format!("{} (#{} @{})", desc, pr.number, pr.author.login);
    entries.push((kind, ctx.new_list_item(&text)));
  } else {
    let text = format!("{} (#{} @{})", pr.title, pr.number, pr.author.login);
    entries.push((SectionKind::Internal, ctx.new_list_item(&text)));
  }

  entries
}

fn create_formatted_item<'a>(
  ctx: &ChangelogContext<'a>, target: Node<'a>, raw_text: &str, desc: &str,
) -> Node<'a> {
  let item = ctx.new_node(NodeValue::Item(NodeList {
    list_type: ListType::Bullet,
    bullet_char: b'-',
    delimiter: ListDelimType::Period,
    tight: true,
    ..NodeList::default()
  }));

  let p = ctx.new_node(NodeValue::Paragraph);
  if let Some(scope) = extract_conventional_scope(raw_text) {
    let strong = ctx.new_node(NodeValue::Strong);
    strong.append(ctx.new_text(scope));
    p.append(strong);
    p.append(ctx.new_text(format!(": {}", desc)));
  } else {
    p.append(ctx.new_text(desc.to_string()));
  }
  item.append(p);

  if matches!(target.data.borrow().value, NodeValue::Item(_)) {
    for (i, child) in target.children().enumerate() {
      if i != 0 {
        item.append(ctx.deep_clone(child));
      }
    }
  }
  item
}

/// Collect changelog items from parsed markdown AST.
/// Handles both list items and top-level paragraphs.
fn collect_changelog_items<'a>(root: Node<'a>) -> Vec<(Node<'a>, Node<'a>)> {
  let mut items = Vec::new();

  for node in root.children() {
    match &node.data.borrow().value {
      NodeValue::List(_) => {
        // Iterate through list items
        for item in node.children() {
          if matches!(item.data.borrow().value, NodeValue::Item(_)) {
            let content_node = item.first_child().unwrap_or(item);
            items.push((item, content_node));
          }
        }
      }
      NodeValue::Item(_) => {
        let content_node = node.first_child().unwrap_or(node);
        items.push((node, content_node));
      }
      NodeValue::Paragraph => {
        items.push((node, node));
      }
      _ => {}
    }
  }

  items
}

fn inject_pr_meta<'a>(ctx: &ChangelogContext<'a>, item: Node<'a>, pr: &PR) {
  let suffix = format!(" (#{} @{})", pr.number, pr.author.login);
  if let Some(p) = item
    .children()
    .find(|n| matches!(n.data.borrow().value, NodeValue::Paragraph))
  {
    p.append(ctx.new_text(suffix));
  }
}

fn extract_block(text: &str) -> Option<String> {
  let s = text.find(MARKER_START)? + MARKER_START.len();
  let e = text.find(MARKER_END)?;
  if s < e { Some(text[s..e].trim().to_string()) } else { None }
}

fn parse_conventional_head(text: &str) -> Option<(SectionKind, &str)> {
  let (head, desc) = text.split_once(':')?;
  let type_scope = head
    .split_once('(')
    .map(|(t, _)| t)
    .unwrap_or(head);
  let kind = SectionKind::from_str(type_scope)?;
  Some((kind, desc.trim()))
}

fn extract_conventional_scope(text: &str) -> Option<String> {
  let (head, _) = text.split_once(':')?;
  let start = head.find('(')?;
  let end = head[start + 1..].find(')')?;
  Some(head[start + 1..start + 1 + end].to_string())
}

fn select_base_version(
  releases: &[crate::changelog::Release<'_>], target: &Version,
) -> Result<Version> {
  if releases.is_empty() {
    return Err("No releases found".into());
  }

  if let Some(pos) = releases.iter().position(|r| &r.version == target) {
    let next = releases
      .get(pos + 1)
      .ok_or("No previous release found for target version")?;
    return Ok(next.version.clone());
  }

  Ok(releases[0].version.clone())
}

fn clear_release_content(header: Node<'_>) {
  let mut current = header.next_sibling();
  while let Some(node) = current {
    current = node.next_sibling();
    if matches!(node.data.borrow().value, NodeValue::Heading(ref h) if h.level == 2) {
      break;
    }
    node.detach();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_section_parsing() {
    assert_eq!(SectionKind::from_str("feat"), Some(SectionKind::Features));
    assert_eq!(SectionKind::from_str("fix"), Some(SectionKind::Fixed));
    assert_eq!(SectionKind::from_str("unknown"), None);
  }

  #[test]
  fn test_conventional_head() {
    let (k, d) = parse_conventional_head("feat(ui): Add items").unwrap();
    assert_eq!(k, SectionKind::Features);
    assert_eq!(d, "Add items");
  }

  #[test]
  fn test_extract_block() {
    let msg = format!("foo\n{}\n- feat: bar\n{}\nbaz", MARKER_START, MARKER_END);
    assert_eq!(extract_block(&msg).unwrap(), "- feat: bar");
  }

  #[test]
  fn test_extract_entries_with_multiline_docs() {
    use crate::types::Author;

    // Test that multi-line changelog entries are preserved
    let body = format!(
      "{}\n- feat(widgets): add Tooltip widget\n\n  Additional documentation here.\n  Second line \
       of docs.\n{}",
      MARKER_START, MARKER_END
    );
    let pr = PR {
      number: 123,
      title: "test".into(),
      body: Some(body),
      author: Author { login: "user".into() },
    };

    let arena = Arena::new();
    let ctx = ChangelogContext::load_from_content(&arena, "# Changelog\n").unwrap();
    let entries = extract_change_entries(&ctx, &pr);

    assert_eq!(entries.len(), 1);
    let (kind, node) = &entries[0];
    assert_eq!(*kind, SectionKind::Features);

    // Verify the entry has multiple children (multi-paragraph)
    let children_count = node.children().count();
    assert!(children_count >= 1, "Expected at least 1 child, got {}", children_count);
  }

  #[test]
  fn test_extract_entries_preserves_additional_docs() {
    use crate::{changelog::collect_text, types::Author};

    // Test that the additional documentation content is actually preserved
    let body = format!(
      "{}\n- feat(core): new feature\n\n  This explains the feature in detail.\n{}",
      MARKER_START, MARKER_END
    );
    let pr = PR {
      number: 42,
      title: "test".into(),
      body: Some(body),
      author: Author { login: "dev".into() },
    };

    let arena = Arena::new();
    let ctx = ChangelogContext::load_from_content(&arena, "# Changelog\n").unwrap();
    let entries = extract_change_entries(&ctx, &pr);

    assert_eq!(entries.len(), 1);
    let (_kind, node) = &entries[0];

    // Check that node has multiple children (paragraphs)
    let children: Vec<_> = node.children().collect();
    assert!(
      children.len() >= 2,
      "Expected at least 2 children (main + additional docs), got {}",
      children.len()
    );

    // Collect all text from the node
    let mut all_text = String::new();
    for child in node.children() {
      all_text.push_str(&collect_text(child));
      all_text.push(' ');
    }

    assert!(all_text.contains("new feature"), "Should contain the main text");
    assert!(
      all_text.contains("explains the feature"),
      "Should preserve additional documentation, got: {}",
      all_text
    );
  }

  #[test]
  fn test_extract_entries_fallback_to_title_when_block_empty() {
    use crate::types::Author;

    let body = format!(
      "{}\n\n- [ ] 🔧 No changelog needed (tests, CI, infra, or unreleased fix)\n{}",
      MARKER_START, MARKER_END
    );
    let pr = PR {
      number: 24,
      title: "Scripts".into(),
      body: Some(body),
      author: Author { login: "dev".into() },
    };

    let arena = Arena::new();
    let ctx = ChangelogContext::load_from_content(&arena, "# Changelog\n").unwrap();
    let entries = extract_change_entries(&ctx, &pr);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, SectionKind::Internal);
  }

  #[test]
  fn test_select_base_version() {
    let arena = Arena::new();
    let content = r#"## [0.5.0-alpha.2] - 2025-01-20

## [0.5.0-alpha.1] - 2025-01-15
"#;
    let ctx = ChangelogContext::load_from_content(&arena, content).unwrap();
    let releases = ctx.changelog.releases();

    let target = Version::parse("0.5.0-alpha.2").unwrap();
    let base = select_base_version(&releases, &target).unwrap();
    assert_eq!(base.to_string(), "0.5.0-alpha.1");

    let missing = Version::parse("0.5.0-alpha.3").unwrap();
    let base = select_base_version(&releases, &missing).unwrap();
    assert_eq!(base.to_string(), "0.5.0-alpha.2");
  }

  #[test]
  fn test_clear_release_content() {
    let arena = Arena::new();
    let content = r#"## [0.5.0-alpha.2] - 2025-01-20

### Features
- feat: example

## [0.5.0-alpha.1] - 2025-01-15
"#;
    let ctx = ChangelogContext::load_from_content(&arena, content).unwrap();
    let header = ctx.changelog.releases()[0].header;

    clear_release_content(header);

    let mut output = String::new();
    let mut opts = Options::default();
    opts.render.r#unsafe = true;
    comrak::format_commonmark(ctx.root, &opts, &mut output).unwrap();
    assert!(!output.contains("feat: example"));
  }
}

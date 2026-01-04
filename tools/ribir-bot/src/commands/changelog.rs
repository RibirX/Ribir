//! Changelog command implementations.

use std::collections::HashMap;

use comrak::{
  Arena, Options,
  arena_tree::Node,
  nodes::{Ast, ListDelimType, ListType, NodeList, NodeValue},
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
/// Returns the changelog content (for dry-run preview) or empty string if no new content.
pub fn cmd_collect(_config: &Config, version: &str, write: bool) -> Result<String> {
  println!("📋 Collecting PRs for version {}...", version);
  let target_ver = Version::parse(version)?;
  let arena = Arena::new();
  let ctx = ChangelogContext::load(&arena)?;

  let latest = ctx
    .changelog
    .latest_version()
    .ok_or("No releases found")?;
  println!("📌 Latest version: {}", latest);

  let prs = get_merged_prs_since(&latest)?;
  if prs.is_empty() {
    println!("✅ No new content.");
    return Ok(String::new());
  }
  println!("🔍 Found {} new PRs", prs.len());

  let release_node = ctx.ensure_release(&target_ver, &today());
  let mut current_pos = release_node;

  // Group entries by type
  let mut sections: HashMap<SectionKind, Vec<&Node<std::cell::RefCell<Ast>>>> = HashMap::new();
  for pr in &prs {
    for (kind, entry) in extract_change_entries(&ctx, pr) {
      sections.entry(kind).or_default().push(entry);
    }
  }

  // Insert sections
  for kind in SectionKind::ALL {
    if let Some(entries) = sections.get(kind) {
      // 1. Heading
      let h3 = ctx.new_heading(3, &kind.header());
      current_pos.insert_after(h3);
      current_pos = h3;

      // 2. List
      let list = ctx.new_node(NodeValue::List(NodeList {
        list_type: ListType::Bullet,
        delimiter: ListDelimType::Period,
        bullet_char: b'-',
        tight: true,
        ..NodeList::default()
      }));
      current_pos.insert_after(list);
      current_pos = list;

      // 3. Items
      for entry in entries {
        list.append(entry);
      }
    }
  }

  ctx.save_and_get_content(!write)
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

fn extract_change_entries<'a>(
  ctx: &ChangelogContext<'a>, pr: &PR,
) -> Vec<(SectionKind, &'a Node<'a, std::cell::RefCell<Ast>>)> {
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
      let arena = Arena::new();
      let root = parse_document(&arena, &content, &Options::default());

      // Collect all items from lists and top-level paragraphs
      let items = collect_changelog_items(root);

      for (target, content_node) in items {
        let text = collect_text(content_node);
        if let Some((kind, _desc)) = parse_conventional_head(&text) {
          let item = ctx.new_node(NodeValue::Item(NodeList {
            list_type: ListType::Bullet,
            bullet_char: b'-',
            delimiter: ListDelimType::Period,
            tight: true,
            ..NodeList::default()
          }));

          if matches!(target.data.borrow().value, NodeValue::Item(_)) {
            for child in target.children() {
              item.append(ctx.deep_clone(child));
            }
          } else {
            let p = ctx.new_node(NodeValue::Paragraph);
            p.append(ctx.deep_clone(target));
            item.append(p);
          }

          inject_pr_meta(ctx, item, pr);
          entries.push((kind, item));
        }
      }
      return entries;
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

/// Collect changelog items from parsed markdown AST.
/// Handles both list items and top-level paragraphs.
fn collect_changelog_items<'a, 'b>(
  root: &'b Node<'b, std::cell::RefCell<Ast>>,
) -> Vec<(&'b Node<'b, std::cell::RefCell<Ast>>, &'b Node<'b, std::cell::RefCell<Ast>>)> {
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

fn inject_pr_meta<'a>(
  ctx: &ChangelogContext<'a>, item: &'a Node<'a, std::cell::RefCell<Ast>>, pr: &PR,
) {
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
      "{}\n- feat(widgets): add Tooltip widget\n\n  Additional documentation here.\n  Second line of docs.\n{}",
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
    use crate::changelog::collect_text;
    use crate::types::Author;

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
    assert!(children.len() >= 2, "Expected at least 2 children (main + additional docs), got {}", children.len());

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
}

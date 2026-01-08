//! Changelog merge utilities.
//!
//! This module handles merging pre-release changelog entries into stable
//! releases.

use comrak::{
  Node, Options,
  nodes::{ListDelimType, ListType, NodeList, NodeValue},
  parse_document,
};
use semver::Version;

use crate::{
  changelog::{ChangelogContext, collect_text, extract_scope},
  types::Result,
};

/// A bucket for collecting section items during merge.
struct SectionBucket<'a> {
  title: String,
  items: Vec<Node<'a>>,
  is_collapsible: bool,
}

/// Merge pre-releases into a target stable version.
pub fn merge<'a>(ctx: &ChangelogContext<'a>, target: &Version) -> Result<()> {
  let (prereleases, target_node) = ctx.changelog.find_merge_candidates(target);
  if prereleases.is_empty() {
    return Err(format!("No pre-releases found for {}", target).into());
  }
  println!("📦 Merging {} pre-releases", prereleases.len());

  let (intro, sections) = collect_content(ctx, prereleases);
  let target_release =
    target_node.unwrap_or_else(|| ctx.ensure_release(target, &crate::utils::today()));

  insert_content(ctx, target_release, intro, sections)
}

/// Collect content from pre-releases into intro and sections.
fn collect_content<'a>(
  ctx: &ChangelogContext<'a>, prereleases: Vec<crate::changelog::Release<'a>>,
) -> (Vec<Node<'a>>, Vec<SectionBucket<'a>>) {
  let mut sections: Vec<SectionBucket<'a>> = Vec::new();
  let mut intro: Vec<Node<'a>> = Vec::new();

  for release in prereleases {
    let mut curr = release.header.next_sibling();
    let mut section_title: Option<String> = None;
    let mut collapsible_summary: Option<String> = None;

    while let Some(node) = curr {
      curr = node.next_sibling();
      let value = node.data.borrow().value.clone();

      // Handle nodes inside a collapsible section
      if let Some(ref summary) = collapsible_summary {
        if is_details_end(&value) {
          collapsible_summary = None;
        } else if matches!(value, NodeValue::List(_)) {
          add_to_section(&mut sections, summary, node, true);
          continue;
        }
        node.detach();
        continue;
      }

      // Check for collapsible section start
      if let Some((summary, is_single_block)) = parse_details_start(&value) {
        if is_single_block {
          parse_inline_list_items(ctx, &value, &summary, &mut sections);
        } else {
          collapsible_summary = Some(summary);
        }
        node.detach();
        continue;
      }

      // Handle headings
      if let NodeValue::Heading(ref h) = value {
        if h.level <= 2 {
          break;
        }
        if h.level == 3 {
          section_title = Some(collect_text(node).trim().to_string());
          node.detach();
          continue;
        }
      }

      // Regular content
      node.detach();
      match &section_title {
        Some(title) => add_to_section(&mut sections, title, node, false),
        None => intro.push(node),
      }
    }
    release.header.detach();
  }

  (intro, sections)
}

/// Insert collected content after the target release header.
fn insert_content<'a>(
  ctx: &ChangelogContext<'a>, target: Node<'a>, intro: Vec<Node<'a>>,
  sections: Vec<SectionBucket<'a>>,
) -> Result<()> {
  let mut cursor = target;

  for node in intro {
    cursor.insert_after(node);
    cursor = node;
  }

  for bucket in sections {
    let items = collect_sorted_items(bucket.items);
    if items.is_empty() {
      continue;
    }

    if bucket.is_collapsible {
      cursor = insert_collapsible_section(ctx, cursor, &bucket.title, items)?;
    } else {
      cursor = insert_regular_section(ctx, cursor, &bucket.title, items);
    }
  }

  Ok(())
}

/// Collect and sort items from section buckets.
fn collect_sorted_items(nodes: Vec<Node<'_>>) -> Vec<Node<'_>> {
  let mut items: Vec<_> = nodes
    .into_iter()
    .flat_map(|node| {
      node.children().filter(|child| {
        if matches!(child.data.borrow().value, NodeValue::Item(_)) {
          child.detach();
          true
        } else {
          false
        }
      })
    })
    .collect();

  items.sort_by(|a, b| {
    let scope_a = non_empty_scope(extract_scope(a));
    let scope_b = non_empty_scope(extract_scope(b));
    scope_a.cmp(&scope_b)
  });

  items
}

/// Convert empty scope to None for proper sorting (empty scopes first).
fn non_empty_scope(s: String) -> Option<String> { if s.is_empty() { None } else { Some(s) } }

/// Create a new bullet list node.
fn new_bullet_list<'a>(ctx: &ChangelogContext<'a>) -> Node<'a> {
  ctx.new_node(NodeValue::List(NodeList {
    list_type: ListType::Bullet,
    delimiter: ListDelimType::Period,
    bullet_char: b'-',
    tight: true,
    ..NodeList::default()
  }))
}

/// Insert a collapsible `<details>` section.
fn insert_collapsible_section<'a>(
  ctx: &ChangelogContext<'a>, cursor: Node<'a>, title: &str, items: Vec<Node<'a>>,
) -> Result<Node<'a>> {
  let list = new_bullet_list(ctx);
  for item in items {
    list.append(item);
  }

  let mut list_md = String::new();
  comrak::format_commonmark(list, &Options::default(), &mut list_md)?;

  let html = format!("<details>\n<summary>{title}</summary>\n\n{list_md}\n</details>\n");
  let block = ctx.new_html_block(&html);
  cursor.insert_after(block);
  Ok(block)
}

/// Insert a regular section with h3 heading and list.
fn insert_regular_section<'a>(
  ctx: &ChangelogContext<'a>, cursor: Node<'a>, title: &str, items: Vec<Node<'a>>,
) -> Node<'a> {
  let h3 = ctx.new_heading(3, title);
  cursor.insert_after(h3);

  let list = new_bullet_list(ctx);
  for item in items {
    list.append(item);
  }
  h3.insert_after(list);
  list
}

/// Check if an HTML block ends a `<details>` section.
fn is_details_end(value: &NodeValue) -> bool {
  matches!(value, NodeValue::HtmlBlock(h) if h.literal.contains("</details>"))
}

/// Parse the start of a `<details>` block.
/// Returns (summary_text, is_single_block) if found.
fn parse_details_start(value: &NodeValue) -> Option<(String, bool)> {
  if let NodeValue::HtmlBlock(h) = value
    && h.literal.contains("<details>")
    && let Some(summary) = extract_summary(&h.literal)
  {
    let is_single = h.literal.contains("</details>");
    return Some((summary, is_single));
  }
  None
}

/// Parse inline list items from a single-block `<details>` element.
fn parse_inline_list_items<'a>(
  ctx: &ChangelogContext<'a>, value: &NodeValue, summary: &str,
  sections: &mut Vec<SectionBucket<'a>>,
) {
  let NodeValue::HtmlBlock(h) = value else { return };

  for line in h.literal.lines() {
    let Some(content) = line.trim().strip_prefix("- ") else { continue };

    let doc = parse_document(ctx.arena, content, &Options::default());
    let item = ctx.new_node(NodeValue::Item(NodeList {
      list_type: ListType::Bullet,
      delimiter: ListDelimType::Period,
      bullet_char: b'-',
      tight: true,
      ..NodeList::default()
    }));

    for child in doc.children() {
      child.detach();
      item.append(child);
    }
    add_to_section(sections, summary, item, true);
  }
}

fn add_to_section<'a>(
  sections: &mut Vec<SectionBucket<'a>>, title: &str, node: Node<'a>, is_collapsible: bool,
) {
  if let Some(bucket) = sections.iter_mut().find(|s| s.title == title) {
    bucket.items.push(node);
  } else {
    sections.push(SectionBucket { title: title.to_string(), items: vec![node], is_collapsible });
  }
}

fn extract_summary(html: &str) -> Option<String> {
  const START: &str = "<summary>";
  const END: &str = "</summary>";

  let start_idx = html.find(START)?;
  let end_idx = html[start_idx..].find(END)?;
  Some(
    html[start_idx + START.len()..start_idx + end_idx]
      .trim()
      .to_string(),
  )
}

#[cfg(test)]
mod tests {
  use comrak::Options;

  use crate::changelog::ChangelogContext;

  #[test]
  fn test_merge_prereleases_with_grouping() {
    let arena = comrak::Arena::new();
    let content = r#"
## [0.5.0-rc.2] - 2025-01-25

This is an introduction in RC2.

### Features
- feat: rc2 feature

## [0.5.0-rc.1] - 2025-01-20

### Features
- feat: rc1 feature

### Fixes
- fix: rc1 fix
"#;
    let ctx = ChangelogContext::load_from_content(&arena, content).unwrap();
    let target = semver::Version::parse("0.5.0").unwrap();

    ctx.merge_prereleases(&target).unwrap();

    // Verify content
    let mut output = String::new();
    comrak::format_commonmark(ctx.root, &Options::default(), &mut output).unwrap();

    // Verify sections are merged
    assert!(output.contains("## [0.5.0]") || output.contains("## \\[0.5.0\\]"));
    assert!(output.contains("This is an introduction in RC2."));

    // Check Features section
    let features_pos = output.find("### Features").unwrap();
    let fixes_pos = output.find("### Fixes").unwrap();
    let intro_pos = output
      .find("This is an introduction in RC2.")
      .unwrap();

    // Intro should be before Features
    assert!(intro_pos < features_pos);

    // Ensure both features are under the same header (concatenated)
    let rc2_feat_pos = output.find("feat: rc2 feature").unwrap();
    let rc1_feat_pos = output.find("feat: rc1 feature").unwrap();

    // first, rc.1 is second. So releases() order: [rc.2, rc.1].
    // So rc.2 processed first.

    assert!(features_pos < rc2_feat_pos);
    assert!(features_pos < rc1_feat_pos);
    // Fixes should be separate
    assert!(fixes_pos > rc1_feat_pos);
    assert!(output.find("fix: rc1 fix").unwrap() > fixes_pos);

    // Ensure only one Features header
    assert_eq!(output.matches("### Features").count(), 1);
  }

  #[test]
  fn test_merge_prereleases_sorts_by_scope() {
    let arena = comrak::Arena::new();
    let content = r#"
## [0.5.0-rc.2] - 2025-01-25

### Features

- **widgets**: widget feature from rc2
- **core**: core feature from rc2

## [0.5.0-rc.1] - 2025-01-20

### Features

- **painter**: painter feature from rc1
- **macros**: macros feature from rc1
"#;
    let ctx = ChangelogContext::load_from_content(&arena, content).unwrap();
    let target = semver::Version::parse("0.5.0").unwrap();

    ctx.merge_prereleases(&target).unwrap();

    // Verify content
    let mut output = String::new();
    comrak::format_commonmark(ctx.root, &Options::default(), &mut output).unwrap();

    // Verify sections are merged and sorted by scope
    // Expected order: core, macros, painter, widgets (alphabetical)
    let core_pos = output.find("**core**").unwrap();
    let macros_pos = output.find("**macros**").unwrap();
    let painter_pos = output.find("**painter**").unwrap();
    let widgets_pos = output.find("**widgets**").unwrap();

    assert!(core_pos < macros_pos, "core should come before macros");
    assert!(macros_pos < painter_pos, "macros should come before painter");
    assert!(painter_pos < widgets_pos, "painter should come before widgets");
  }

  #[test]
  fn test_merge_prereleases_with_collapsible_internal() {
    let arena = comrak::Arena::new();
    // Note: use the exact string that SectionKind::Internal.header() returns
    let content = r#"
## [0.5.0-rc.2] - 2025-01-25

<details>
<summary>🔧 Internal</summary>

- chore: rc2 internal
</details>

## [0.5.0-rc.1] - 2025-01-20

<details>
<summary>🔧 Internal</summary>

- chore: rc1 internal
</details>
"#;
    let ctx = ChangelogContext::load_from_content(&arena, content).unwrap();
    let target = semver::Version::parse("0.5.0").unwrap();

    ctx.merge_prereleases(&target).unwrap();

    // Verify content
    let mut output = String::new();
    let mut opts = Options::default();
    opts.render.r#unsafe = true;
    comrak::format_commonmark(ctx.root, &opts, &mut output).unwrap();

    println!("Output:\n{}", output);

    assert!(output.contains("<details>"));
    assert!(output.contains("<summary>🔧 Internal</summary>"));
    assert!(output.contains("- chore: rc1 internal"));
    assert!(output.contains("- chore: rc2 internal"));
    // Ensure only one block
    assert_eq!(output.matches("<details>").count(), 1);
  }
}

use std::{
  borrow::Cow,
  collections::{BTreeSet, HashSet},
  path::{Path, PathBuf},
  sync::OnceLock,
  time::Instant,
};

use super::*;

#[derive(Default)]
pub(super) struct MacosLibraryFonts {
  missing_families: HashSet<String>,
  warned_files: HashSet<PathBuf>,
}

impl MacosLibraryFonts {
  pub(super) fn new() -> Self { Self::default() }

  pub(super) fn ensure_named_families<Brush>(
    &mut self, collection: &mut parley::fontique::Collection, source: &AttributedText<Brush>,
    text_style: &TextStyle,
  ) {
    for family_name in requested_named_families(source, text_style) {
      self.ensure_named_family(collection, &family_name);
    }
  }

  // Temporary workaround for fontique/parley 0.7 on macOS: CoreText misses some
  // fonts (notably San Francisco and PingFang), so we scan known system font
  // directories on demand and register only the missing named families that the
  // current text actually requests.
  // TODO: Remove this macOS fallback scan after upgrading to a parley release
  // that includes the upstream system-font fix from main.
  fn ensure_named_family(
    &mut self, collection: &mut parley::fontique::Collection, family_name: &str,
  ) {
    if collection.family_id(family_name).is_some() || self.missing_families.contains(family_name) {
      return;
    }

    if self.alias_family_already_available(collection, family_name)
      || self.alias_family_already_missing(family_name)
    {
      return;
    }

    let preferred_candidate_files = macos_preferred_font_files(family_name);
    let candidate_files = macos_library_font_files();
    let start = Instant::now();
    let mut matched_any_file = false;
    let mut registered_files = 0usize;
    let mut resolved = false;

    for path in preferred_candidate_files
      .iter()
      .chain(candidate_files.iter())
    {
      match read_matching_font_file(path, family_name) {
        Ok(Some(data)) => {
          matched_any_file = true;
          registered_files += 1;
          collection.register_fonts(Blob::from(data), None);
          if collection.family_id(family_name).is_some()
            || self.alias_family_already_available(collection, family_name)
          {
            resolved = true;
            break;
          }
        }
        Ok(None) => {}
        Err(err) => {
          if self.warned_files.insert(path.clone()) {
            tracing::warn!("Failed to inspect macOS font file {}: {}", path.display(), err);
          }
        }
      }
    }

    tracing::debug!(
      family = family_name,
      preferred_candidate_count = preferred_candidate_files.len(),
      candidate_count = preferred_candidate_files.len() + candidate_files.len(),
      registered_files,
      matched_any_file,
      resolved,
      elapsed_ms = start.elapsed().as_millis(),
      "Completed macOS font workaround lookup",
    );

    if resolved {
      return;
    }

    if matched_any_file {
      tracing::warn!(
        "Requested macOS font family '{}' was found in the system font scan but remained \
         unavailable after registration.",
        family_name
      );
    }
    self
      .missing_families
      .insert(family_name.to_owned());
  }

  fn alias_family_already_available(
    &mut self, collection: &mut parley::fontique::Collection, family_name: &str,
  ) -> bool {
    macos_font_family_aliases(family_name).is_some_and(|aliases| {
      aliases
        .iter()
        .any(|alias| *alias != family_name && collection.family_id(alias).is_some())
    })
  }

  fn alias_family_already_missing(&self, family_name: &str) -> bool {
    macos_font_family_aliases(family_name).is_some_and(|aliases| {
      aliases
        .iter()
        .any(|alias| *alias != family_name && self.missing_families.contains(*alias))
    })
  }
}

pub(super) fn named_font_family_variants(name: &str) -> Vec<Cow<'static, str>> {
  if let Some(aliases) = macos_font_family_aliases(name) {
    return aliases
      .iter()
      .map(|alias| Cow::Borrowed(*alias))
      .collect();
  }

  vec![Cow::Owned(name.to_string())]
}

fn requested_named_families<Brush>(
  source: &AttributedText<Brush>, text_style: &TextStyle,
) -> Vec<String> {
  let mut families = Vec::new();
  let mut seen = BTreeSet::new();
  let text = source.text.as_ref();
  collect_named_families_from_face(&text_style.font_face, text, &mut families, &mut seen);
  for span in source.spans.iter() {
    if let Some(font) = span.style.font.as_ref() {
      collect_named_families_from_face(&font.face, text, &mut families, &mut seen);
    }
  }
  families
}

fn collect_named_families_from_face(
  face: &crate::FontFace, text: &str, families: &mut Vec<String>, seen: &mut BTreeSet<String>,
) {
  let mut remaining_scripts = text_script_needs(text);
  let mut is_first_named_family = true;

  for family in face.families.iter() {
    if let FontFamily::Name(name) = family {
      if !is_first_named_family
        && !named_family_is_relevant_for_text(name.as_ref(), &mut remaining_scripts)
      {
        continue;
      }

      remember_named_family_variants(name.as_ref(), families, seen);
      is_first_named_family = false;
    }
  }
}

fn remember_named_family_variants(
  name: &str, families: &mut Vec<String>, seen: &mut BTreeSet<String>,
) {
  for name in named_font_family_variants(name) {
    let name = name.into_owned();
    if seen.insert(name.clone()) {
      families.push(name);
    }
  }
}

#[derive(Clone, Copy, Default)]
struct TextScriptNeeds {
  han: bool,
  kana: bool,
  hangul: bool,
}

impl TextScriptNeeds {
  fn intersects(self, support: TextScriptNeeds) -> bool {
    (self.han && support.han) || (self.kana && support.kana) || (self.hangul && support.hangul)
  }

  fn cover(&mut self, support: TextScriptNeeds) {
    if support.han {
      self.han = false;
    }
    if support.kana {
      self.kana = false;
    }
    if support.hangul {
      self.hangul = false;
    }
  }
}

fn named_family_is_relevant_for_text(name: &str, remaining_scripts: &mut TextScriptNeeds) -> bool {
  let Some(support) = macos_named_family_script_support(name) else {
    return true;
  };

  if !remaining_scripts.intersects(support) {
    return false;
  }

  remaining_scripts.cover(support);
  true
}

fn macos_named_family_script_support(name: &str) -> Option<TextScriptNeeds> {
  match name {
    "PingFang SC" | "苹方-简" | "PingFang TC" | "苹方-繁" | "PingFang HK" | "苹方-港"
    | "PingFang MO" | "苹方-澳" => {
      Some(TextScriptNeeds { han: true, ..TextScriptNeeds::default() })
    }
    "Hiragino Sans" => {
      Some(TextScriptNeeds { han: true, kana: true, ..TextScriptNeeds::default() })
    }
    "Noto Sans CJK SC" | "Source Han Sans" => {
      Some(TextScriptNeeds { han: true, kana: true, hangul: true })
    }
    _ => None,
  }
}

fn text_script_needs(text: &str) -> TextScriptNeeds {
  let mut needs = TextScriptNeeds::default();
  for ch in text.chars() {
    let ch = ch as u32;
    match ch {
      0x3000..=0x303F
      | 0x3400..=0x4DBF
      | 0x4E00..=0x9FFF
      | 0xF900..=0xFAFF
      | 0x20000..=0x2A6DF
      | 0x2A700..=0x2B73F
      | 0x2B740..=0x2B81F
      | 0x2B820..=0x2CEAF
      | 0x2CEB0..=0x2EBEF
      | 0x30000..=0x3134F
      | 0xFF00..=0xFF60
      | 0xFFE0..=0xFFEF => needs.han = true,
      0x3040..=0x309F | 0x30A0..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9D => needs.kana = true,
      0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7AF | 0xD7B0..=0xD7FF => {
        needs.hangul = true
      }
      _ => {}
    }
  }
  needs
}

fn macos_font_family_aliases(name: &str) -> Option<&'static [&'static str]> {
  match name {
    "PingFang SC" | "苹方-简" => Some(&["苹方-简", "PingFang SC"]),
    "PingFang TC" | "苹方-繁" => Some(&["苹方-繁", "PingFang TC"]),
    "PingFang HK" | "苹方-港" => Some(&["苹方-港", "PingFang HK"]),
    "PingFang MO" | "苹方-澳" => Some(&["苹方-澳", "PingFang MO"]),
    _ => None,
  }
}

fn macos_library_font_files() -> &'static [PathBuf] {
  static FILES: OnceLock<Box<[PathBuf]>> = OnceLock::new();
  FILES.get_or_init(|| {
    let start = Instant::now();
    let files = collect_macos_library_font_files().into_boxed_slice();
    tracing::debug!(
      candidate_count = files.len(),
      elapsed_ms = start.elapsed().as_millis(),
      "Indexed macOS system font candidates for Parley workaround",
    );
    files
  })
}

fn macos_preferred_font_files(family_name: &str) -> &'static [PathBuf] {
  if is_pingfang_family_name(family_name) { macos_pingfang_font_files() } else { &[] }
}

fn macos_pingfang_font_files() -> &'static [PathBuf] {
  static FILES: OnceLock<Box<[PathBuf]>> = OnceLock::new();
  FILES.get_or_init(|| {
    let start = Instant::now();
    let files = collect_macos_pingfang_font_files().into_boxed_slice();
    tracing::debug!(
      candidate_count = files.len(),
      elapsed_ms = start.elapsed().as_millis(),
      "Indexed macOS PingFang candidates for Parley workaround",
    );
    files
  })
}

fn collect_macos_library_font_files() -> Vec<PathBuf> {
  let mut files = BTreeSet::new();

  for dir in macos_library_font_dirs() {
    if dir.is_dir() {
      collect_font_files(&dir, 8, 0, &mut files);
    }
  }

  files.into_iter().collect()
}

fn collect_macos_pingfang_font_files() -> Vec<PathBuf> {
  let mut files = BTreeSet::new();

  for dir in macos_pingfang_font_dirs() {
    if dir.is_dir() {
      collect_font_files(&dir, 4, 0, &mut files);
    }
  }

  files.into_iter().collect()
}

fn macos_library_font_dirs() -> [PathBuf; 4] {
  [
    PathBuf::from("/System/Library/Fonts"),
    PathBuf::from("/Library/Fonts"),
    PathBuf::from("/Network/Library/Fonts"),
    std::env::var_os("HOME")
      .map(PathBuf::from)
      .map(|home| home.join("Library/Fonts"))
      .unwrap_or_else(|| PathBuf::from("/var/empty")),
  ]
}

fn macos_pingfang_font_dirs() -> [PathBuf; 2] {
  [
    // Newer macOS releases ship PingFang.ttc under AssetsV2 instead of
    // /System/Library/Fonts.
    PathBuf::from("/System/Library/AssetsV2/com_apple_MobileAsset_Font7"),
    PathBuf::from("/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved"),
  ]
}

fn is_pingfang_family_name(name: &str) -> bool {
  matches!(
    name,
    "PingFang SC"
      | "苹方-简"
      | "PingFang TC"
      | "苹方-繁"
      | "PingFang HK"
      | "苹方-港"
      | "PingFang MO"
      | "苹方-澳"
  )
}

fn collect_font_files(dir: &Path, max_depth: u32, depth: u32, out: &mut BTreeSet<PathBuf>) {
  let Ok(entries) = std::fs::read_dir(dir) else {
    return;
  };

  for entry in entries.filter_map(Result::ok) {
    let path = entry.path();
    if path.is_dir() {
      if depth < max_depth {
        collect_font_files(&path, max_depth, depth + 1, out);
      }
    } else if is_font_file(&path) {
      out.insert(path);
    }
  }
}

fn is_font_file(path: &Path) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "ttc" | "otf" | "otc" | "dfont"))
    .unwrap_or(false)
}

fn read_matching_font_file(
  path: &Path, family_name: &str,
) -> Result<Option<Vec<u8>>, std::io::Error> {
  let data = std::fs::read(path)?;
  Ok(font_data_matches_requested_family(&data, family_name).then_some(data))
}

fn font_data_matches_requested_family(data: &[u8], family_name: &str) -> bool {
  let normalized_family = normalize_font_family_name(family_name);
  let face_count = ttf_parser::fonts_in_collection(data).unwrap_or(1);

  (0..face_count)
    .filter_map(|index| ttf_parser::Face::parse(data, index).ok())
    .flat_map(|face| face.names().into_iter())
    .filter(|name| {
      matches!(
        name.name_id,
        ttf_parser::name_id::TYPOGRAPHIC_FAMILY
          | ttf_parser::name_id::FAMILY
          | ttf_parser::name_id::WWS_FAMILY
          | ttf_parser::name_id::FULL_NAME
      )
    })
    .filter_map(|name| name.to_string())
    .map(|name| normalize_font_family_name(&name))
    .any(|name| name == normalized_family || name.starts_with(&normalized_family))
}

fn normalize_font_family_name(name: &str) -> String {
  name
    .chars()
    .filter(|c| c.is_alphanumeric())
    .flat_map(char::to_lowercase)
    .collect()
}

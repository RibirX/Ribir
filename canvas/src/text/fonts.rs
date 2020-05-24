pub use font_kit::properties::{
  Properties, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight,
};
use font_kit::{
  error::FontLoadingError, family_name::FamilyName, font::Font, loader::Loader,
  source::SystemSource,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Manage loaded font.
pub(crate) struct Fonts {
  load_fonts: HashMap<String, Vec<Font>>,
  source: SystemSource,
}

impl Fonts {
  pub fn new() -> Self {
    Self {
      load_fonts: <_>::default(),
      source: SystemSource::new(),
    }
  }

  /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index`
  /// specifies the index of the font to load from it. If the data represents
  /// a single font, pass 0 for `font_index`.
  pub fn load_from_bytes(
    &mut self,
    font_data: Arc<Vec<u8>>,
    font_index: u32,
  ) -> Result<&Font, FontLoadingError> {
    let font = Font::from_bytes(font_data, font_index)?;
    Ok(self.insert_font(font))
  }

  /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
  ///
  /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies
  /// the index of the font to load from it. If the file represents a single
  /// font, pass 0 for `font_index`.
  pub fn load_from_path<P: AsRef<std::path::Path>>(
    &mut self,
    path: P,
    font_index: u32,
  ) -> Result<&Font, FontLoadingError> {
    let font = <Font as Loader>::from_path(path, font_index)?;
    Ok(self.insert_font(font))
  }

  /// Performs font matching according to the CSS Fonts Level 3 specification
  /// and returns matched fonts.
  pub fn select_best_match(&mut self, family_names: &str, props: &Properties) -> Option<&Font> {
    for family in family_names.split(',') {
      let family = family.replace('\'', "");
      let family = family.trim();

      let idx = self
        .load_fonts
        .get(family)
        .map(|families| families.iter().position(|f| &f.properties() == props))
        .flatten();

      if let Some(idx) = idx {
        return self.load_fonts.get(family).map(|fonts| &fonts[idx]);
      } else {
        let font = self
          .source
          .select_best_match(&[family_name(family)], props)
          .ok()
          .map(|handle| handle.load().ok())
          .flatten();
        if let Some(font) = font {
          return Some(self.insert_font(font));
        }
      }
    }

    None
  }

  fn insert_font(&mut self, font: Font) -> &Font {
    let fonts = self
      .load_fonts
      .entry(font.family_name())
      .or_insert_with(Vec::new);

    if let Some(index) = fonts
      .iter()
      .position(|f| f.properties() == font.properties())
    {
      &fonts[index]
    } else {
      fonts.push(font);
      fonts.last().unwrap()
    }
  }
}

fn family_name(name: &str) -> FamilyName {
  match name {
    "serif" => FamilyName::Serif,
    "sans-serif" => FamilyName::SansSerif,
    "monospace" => FamilyName::Monospace,
    "cursive" => FamilyName::Cursive,
    "fantasy" => FamilyName::Fantasy,
    _ => FamilyName::Title(name.to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn load_font_from_path() {
    let mut fonts = Fonts::new();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/DejaVuSans.ttf";
    let font = fonts.load_from_path(path, 0);
    assert_eq!(font.unwrap().family_name(), "DejaVu Sans");
  }

  #[test]
  fn load_font_from_bytes() {
    let mut fonts = Fonts::new();
    let bytes = include_bytes!("../../fonts/GaramondNo8-Reg.ttf");
    let font = fonts.load_from_bytes(std::sync::Arc::new(bytes.to_vec()), 0);
    assert_eq!(font.unwrap().family_name(), "GaramondNo8");
  }

  #[test]
  fn match_font() {
    let mut fonts = Fonts::new();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/DejaVuSans.ttf";
    let _ = fonts.load_from_path(path, 0);
    let mut props = Properties::new();

    let font = fonts.select_best_match("DejaVu Sans, Arial", &props);
    // match custom load fonts.
    assert_eq!(font.unwrap().family_name(), "DejaVu Sans");

    props.style = FontStyle::Italic;
    let font = fonts.select_best_match("Arial, DejaVu Sans", &props);
    // match default fonts
    assert_eq!(font.unwrap().family_name(), "Arial");
    assert_eq!(font.unwrap().properties().style, FontStyle::Italic);
  }
}
